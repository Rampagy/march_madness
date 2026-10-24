# Hosting Plan

## Goal

Serve the static bracket-scoring site from a Raspberry Pi 3B, publish updates without opening inbound WAN ports, and keep the large bracket files on the scoring machine rather than the Pi.

## Architecture

```text
Scoring computer --SSH/rsync over LAN--> Raspberry Pi
                                           |-- Nginx serves site files on localhost
                                           `-- cloudflared tunnel --> Cloudflare --> visitors
```

- **Scoring computer:** stores and scores the bracket files, then exports `site/results.json`.
- **Raspberry Pi:** stores only the site assets, latest `results.json`, and team-name data. It does not rescore the full bracket collection.
- **Nginx:** serves static files locally on the Pi. No application server is needed.
- **Cloudflare Tunnel:** makes the local Nginx site reachable through an outbound tunnel. No router port-forward is required.

## Deployment Steps

### 1. Prepare the Pi

- Install a supported Raspberry Pi OS release and apply updates.
- Give the Pi a stable LAN address, preferably with a DHCP reservation in OPNsense.
- Store the served site on a USB SSD if available; otherwise use the existing storage with a backup plan.
- Install Nginx and `cloudflared` from their supported package sources.

### 2. Configure Nginx

- Serve the contents of the repository's `site/` directory from a dedicated web root such as `/var/www/march-madness`.
- Listen on `127.0.0.1` only; `cloudflared` will connect to this local address.
- Return `Cache-Control: no-store` for `results.json` so polling does not return stale scores.
- Static assets such as `index.html`, `app.js`, `style.css`, and `teams.json` can use normal revalidation or a modest cache lifetime. Use versioned filenames before setting long-lived immutable caching.
- Do not expose directory listings or any bracket data files from the scoring machine.

### 3. Configure Cloudflare Tunnel

- Create a tunnel associated with the site's hostname.
- Run `cloudflared` as an unprivileged service on the Pi.
- Route the hostname to Nginx at `http://127.0.0.1:<port>`.
- Do not add OPNsense WAN port forwards. Keep the Pi's outbound access limited to what the tunnel and system updates require where practical.
- The site can be public. If access should be restricted, add Cloudflare Access in front of the hostname.

### 4. Update the Site Directly from the Scoring Computer

- Generate the new export on the scoring computer, for example:

  ```sh
  ./target/release/march_madness --export site/results.json
  ```

- Push only the changed results file to the Pi using SSH/rsync over the LAN. Copy the site assets separately when they change.
- Add a narrow OPNsense rule permitting the scoring computer to reach the Pi over SSH. The Pi does not need permission to initiate connections to the LAN.
- Use a dedicated SSH key for deployment. Restrict it to the site's web root and avoid using a general-purpose administrative key.
- Upload to a temporary filename and rename it into place after transfer, so visitors see either the previous complete JSON or the new complete JSON, never a partial transfer.
- Do not synchronize the large `.bin` bracket files to the Pi.

### 5. Verify an Update

After a deployment, check that:

- `results.json` on the Pi is valid JSON and has the expected update timestamp and bracket count.
- Fetching `results.json` through the public hostname returns the new data and a `no-store` cache policy.
- The page loads its CSS, JavaScript, and `teams.json` successfully.
- The browser refreshes results on its normal interval and continues showing the last good results if a fetch temporarily fails.
- Nginx and `cloudflared` recover after a Pi reboot.

## Security and Reliability

- Keep Nginx bound to loopback and use the tunnel as the only public route.
- Allow only the scoring computer to SSH to the Pi, and only for deployment access.
- Keep private keys off the Pi where possible; the deployment key should have narrowly scoped write access.
- Back up the site directory and the tunnel configuration/credentials securely. Do not put tunnel credentials in the repository.
- Use atomic replacement for both the scorer's local JSON export and the transferred Pi copy.
- Monitor free disk space and service status. The Pi should serve static files only, keeping its CPU and memory use low.

## Scope and Non-Goals

- The Pi is a static web host, not the bracket-generation or full-dataset scoring machine.
- The public site receives only summary results and top-bracket data, not the 100GB bracket dataset.
- No custom Rust HTTP server, Apache stack, database, or third-party results-storage service is needed for the initial deployment.
- Hash manifests and public verification links can be added as a separate step; file names and sizes alone are not cryptographic verification.

## Rollout Order

1. Test Nginx locally on the Pi using a copied `site/` directory.
2. Test an SSH/rsync update from the scoring computer over the LAN.
3. Confirm `results.json` replacement is atomic and Nginx does not cache it.
4. Bring up the Cloudflare Tunnel and test the public hostname.
5. Reboot the Pi and repeat the checks to confirm services start automatically.
