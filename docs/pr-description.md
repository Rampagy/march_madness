# Improve bracket generation and document deployment

## Summary

- Reduce memory overhead while tracking unique generated brackets and clear the repeat cache after pruning.
- Reuse the random number generator and batch bracket writes to reduce per-bracket overhead.
- Update the README with current scoring, generation, and JSON export usage.
- Add a Raspberry Pi and Cloudflare Tunnel hosting plan for the static scoring site.
- Update the package version to `1.7.1` and include the current reference bracket.

## Validation

- `cargo test`

## Notes

- Generated bracket output remains in version 2 binary format.
- The deployment plan keeps the large bracket files on the scoring machine and serves only the static site and exported results from the Pi.