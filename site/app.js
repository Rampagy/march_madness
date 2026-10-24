const RESULTS_URL = "results.json";
const REFRESH_MS = 60000;
const DRIVE_URL = ""; // public Drive link to the bracket files; shown when set

const number = new Intl.NumberFormat();
const SVG_NS = "http://www.w3.org/2000/svg";

let latest = null;

function el(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}

function formatBytes(bytes) {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

function card(label, value, sub) {
  const node = el("div", "card");
  node.append(el("div", "label", label), el("div", "value", value));
  if (sub) node.append(el("div", "sub", sub));
  return node;
}

function renderStats(data) {
  const percentPerfect = data.total_brackets > 0
    ? (100 * data.perfect_brackets / data.total_brackets).toFixed(4)
    : "0";
  const commonPercent = data.total_brackets > 0
    ? (100 * data.most_common_count / data.total_brackets).toFixed(1)
    : "0";

  document.getElementById("stats").replaceChildren(
    card("Total brackets", number.format(data.total_brackets)),
    card("Perfect brackets", number.format(data.perfect_brackets), `${percentPerfect}%`),
    card("Max possible score", number.format(data.max_bracket_score)),
    card("Average score", data.average_score.toFixed(1)),
    card("Most common score", number.format(data.most_common_score),
         `${number.format(data.most_common_count)} brackets (${commonPercent}%)`),
  );
}

function renderProgress(data) {
  const finished = data.games_finished;
  document.getElementById("progress-bar").style.width = `${(100 * finished / 63).toFixed(1)}%`;
  document.getElementById("progress").setAttribute("aria-valuenow", finished);
  document.getElementById("progress-text").textContent = `${finished} of 63 games finished`;
}

function svg(tag, attrs) {
  const node = document.createElementNS(SVG_NS, tag);
  for (const [key, value] of Object.entries(attrs || {})) node.setAttribute(key, value);
  return node;
}

function renderHistogram(data) {
  const counts = data.score_distribution;
  const useLog = document.getElementById("log-scale").checked;
  const scale = (count) => (useLog ? Math.log10(count + 1) : count);

  let first = counts.findIndex((c) => c > 0);
  let last = counts.length - 1 - [...counts].reverse().findIndex((c) => c > 0);
  const target = document.getElementById("histogram");
  if (first < 0) {
    target.replaceChildren(el("p", "muted", "No data yet."));
    return;
  }

  const width = 900, height = 260;
  const margin = { top: 10, right: 10, bottom: 28, left: 10 };
  const plotWidth = width - margin.left - margin.right;
  const plotHeight = height - margin.top - margin.bottom;
  const bins = last - first + 1;
  const barWidth = plotWidth / bins;
  const maxValue = Math.max(...counts.slice(first, last + 1).map(scale));

  const chart = svg("svg", { viewBox: `0 0 ${width} ${height}`, role: "img",
                             "aria-label": "Number of brackets at each score" });

  for (let i = 0; i < bins; i++) {
    const score = first + i;
    const count = counts[score];
    const barHeight = maxValue > 0 ? (scale(count) / maxValue) * plotHeight : 0;
    const bar = svg("rect", {
      class: "bar",
      x: margin.left + i * barWidth,
      y: margin.top + plotHeight - barHeight,
      width: Math.max(barWidth - 1, 1),
      height: barHeight,
    });
    const tip = svg("title");
    tip.textContent = `${score} points: ${number.format(count)} brackets`;
    bar.append(tip);
    chart.append(bar);
  }

  chart.append(svg("line", { x1: margin.left, x2: width - margin.right,
                             y1: margin.top + plotHeight, y2: margin.top + plotHeight }));

  const step = Math.max(1, Math.ceil(bins / 12));
  for (let score = first; score <= last; score += step) {
    const label = svg("text", { x: margin.left + (score - first + 0.5) * barWidth,
                                y: height - 8, "text-anchor": "middle" });
    label.textContent = score;
    chart.append(label);
  }

  target.replaceChildren(chart);
}

// ---- bracket drawing -------------------------------------------------------
// Games are indexed like the scorer: 0-31 round 1, 32-47 round 2, 48-55 sweet 16,
// 56-59 regional finals, 60-61 semifinals, 62 final. Teams 1-64 run region by region.

const TEAMS_URL = "teams.json";
const SEED_ORDER = [1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15];
const REGION_NAMES = ["East", "West", "South", "Midwest"];

const SLOT_W = 122, BOX_H = 32, PITCH = 36, STEP = 144, EDGE = 8, CENTER_GAP = 26, SEMI_OFFSET = 72;
const PICK_CHARS = 16, ACTUAL_CHARS = 15;
const REGION_H = 16 * PITCH, LABEL_H = 18, REGION_GAP = 16;
const SVG_W = 2 * EDGE + 2 * (4 * STEP + SLOT_W) + 2 * CENTER_GAP + SLOT_W;
const SVG_H = 4 + 2 * (REGION_H + LABEL_H) + REGION_GAP + 4;

let teams = {};

async function loadTeams() {
  try {
    const response = await fetch(TEAMS_URL);
    if (response.ok) teams = await response.json();
  } catch (error) {
    // names are optional; slots fall back to "Team N"
  }
}

function parseBracket(text) {
  return text.split(/[;\s]+/).filter(Boolean).map(Number);
}

function seedOf(team) {
  return ((team - 1) % 16) + 1;
}

function teamName(team) {
  return teams[team] || `Team ${team}`;
}

function teamLabel(team, maxChars) {
  const text = `${seedOf(team)} ${teamName(team)}`;
  return text.length > maxChars ? `${text.slice(0, maxChars - 1)}\u2026` : text;
}

function startingTeam(index) {
  return Math.floor(index / 16) * 16 + SEED_ORDER[index % 16];
}

function eliminatedTeams(actual) {
  const out = new Set();
  for (let game = 0; game < 63; game++) {
    if (!actual[game]) continue;
    const top = game < 32 ? startingTeam(2 * game) : actual[2 * (game - 32)];
    const bottom = game < 32 ? startingTeam(2 * game + 1) : actual[2 * (game - 32) + 1];
    out.add(actual[game] === top ? bottom : top);
  }
  return out;
}

function gameStatus(pick, winner, eliminated) {
  if (winner) return pick === winner ? "correct" : "wrong";
  return eliminated.has(pick) ? "busted" : "pending";
}

function gameIndex(column, region, index) {
  if (column === 1) return region * 8 + index;
  if (column === 2) return 32 + region * 4 + index;
  if (column === 3) return 48 + region * 2 + index;
  return 56 + region;
}

function drawSlot(parent, x, centerY, status, pick, winner) {
  const slot = svg("g", { class: `slot ${status}` });
  slot.append(svg("rect", { x, y: centerY - BOX_H / 2, width: SLOT_W, height: BOX_H, rx: 4 }));

  const wrong = status === "wrong";
  const pickText = svg("text", { class: "pick", x: x + 7, y: centerY + (wrong ? -3 : 4.5) });
  pickText.textContent = teamLabel(pick, PICK_CHARS);
  slot.append(pickText);

  if (wrong) {
    const actualText = svg("text", { class: "actual", x: x + 7, y: centerY + 12 });
    actualText.textContent = `\u2192 ${teamLabel(winner, ACTUAL_CHARS)}`;
    slot.append(actualText);
  }

  const tip = svg("title");
  const names = `${seedOf(pick)} ${teamName(pick)}`;
  tip.textContent = wrong
    ? `Picked ${names}; actual winner ${seedOf(winner)} ${teamName(winner)}`
    : status === "busted" ? `${names} (already eliminated)` : names;
  slot.append(tip);
  parent.append(slot);
}

function connector(parent, x1, y1, x2, y2) {
  const mid = (x1 + x2) / 2;
  parent.append(svg("path", { class: "line", d: `M${x1} ${y1}H${mid}V${y2}H${x2}` }));
}

function renderBracket(bracketText, winningText) {
  const bracket = parseBracket(bracketText);
  const actual = parseBracket(winningText);
  const eliminated = eliminatedTeams(actual);

  const chart = svg("svg", { class: "bracket", viewBox: `0 0 ${SVG_W} ${SVG_H}`, role: "img",
                             "aria-label": "Bracket with correct and wrong picks marked" });
  const lines = svg("g");
  const slots = svg("g");
  chart.append(lines, slots);

  const columnX = (side, column) => (side === 0
    ? EDGE + column * STEP
    : SVG_W - EDGE - SLOT_W - column * STEP);

  for (let region = 0; region < 4; region++) {
    const side = region < 2 ? 0 : 1;
    const blockTop = 4 + (region % 2) * (REGION_H + LABEL_H + REGION_GAP);
    const areaTop = blockTop + LABEL_H;

    const label = svg("text", { class: "region", x: columnX(side, 0), y: blockTop + 11 });
    label.textContent = REGION_NAMES[region];
    chart.append(label);

    for (let column = 0; column < 5; column++) {
      const count = 16 >> column;
      const x = columnX(side, column);

      for (let i = 0; i < count; i++) {
        const y = areaTop + (i + 0.5) * (REGION_H / count);

        if (column === 0) {
          const slot = svg("g", { class: "slot neutral" });
          slot.append(svg("rect", { x, y: y - BOX_H / 2, width: SLOT_W, height: BOX_H, rx: 4 }));
          const text = svg("text", { class: "pick", x: x + 7, y: y + 4.5 });
          text.textContent = teamLabel(startingTeam(region * 16 + i), PICK_CHARS);
          slot.append(text);
          slots.append(slot);
        } else {
          const game = gameIndex(column, region, i);
          drawSlot(slots, x, y, gameStatus(bracket[game], actual[game], eliminated), bracket[game], actual[game]);
        }

        if (column < 4) {
          const nextCount = count / 2;
          const nextY = areaTop + (Math.floor(i / 2) + 0.5) * (REGION_H / nextCount);
          const nextX = columnX(side, column + 1);
          if (side === 0) connector(lines, x + SLOT_W, y, nextX, nextY);
          else connector(lines, x, y, nextX + SLOT_W, nextY);
        }
      }
    }
  }

  const centerX = EDGE + 4 * STEP + SLOT_W + CENTER_GAP;
  const middleY = SVG_H / 2;
  const semis = [{ game: 60, y: middleY - SEMI_OFFSET, regions: [0, 1], side: 0 },
                 { game: 61, y: middleY + SEMI_OFFSET, regions: [2, 3], side: 1 }];

  for (const semi of semis) {
    for (const region of semi.regions) {
      const blockTop = 4 + (region % 2) * (REGION_H + LABEL_H + REGION_GAP);
      const regionY = blockTop + LABEL_H + REGION_H / 2;
      const fromX = columnX(semi.side, 4);
      if (semi.side === 0) connector(lines, fromX + SLOT_W, regionY, centerX, semi.y);
      else connector(lines, fromX, regionY, centerX + SLOT_W, semi.y);
    }
    drawSlot(slots, centerX, semi.y, gameStatus(bracket[semi.game], actual[semi.game], eliminated),
             bracket[semi.game], actual[semi.game]);
  }

  const lineX = centerX + SLOT_W / 2;
  lines.append(svg("path", { class: "line", d: `M${lineX} ${middleY - SEMI_OFFSET + BOX_H / 2}V${middleY - BOX_H / 2}` }));
  lines.append(svg("path", { class: "line", d: `M${lineX} ${middleY + SEMI_OFFSET - BOX_H / 2}V${middleY + BOX_H / 2}` }));

  const champion = svg("text", { class: "region", x: centerX + 4, y: middleY - BOX_H / 2 - 4 });
  champion.textContent = "Champion";
  chart.append(champion);
  drawSlot(slots, centerX, middleY, gameStatus(bracket[62], actual[62], eliminated), bracket[62], actual[62]);

  return chart;
}

function legend() {
  const wrapper = el("div", "legend muted");
  for (const [status, text] of [["correct", "Correct"], ["wrong", "Wrong (actual winner shown)"],
                                ["busted", "Pick already eliminated"], ["pending", "Not played yet"]]) {
    const chip = el("span", `chip ${status}`);
    wrapper.append(chip, el("span", null, text));
  }
  return wrapper;
}

function renderTop(data) {
  const list = document.getElementById("top");
  const firstRender = list.children.length === 0;
  const wasOpen = new Set([...list.querySelectorAll("details[data-place][open]")].map((d) => d.dataset.place));

  const items = data.top_brackets.map((bracket) => {
    const item = el("div", "top-item");
    const head = el("div", "top-head");
    const body = el("div");

    const score = el("div", "score", `${bracket.score} points`);
    const detail = el("div", "muted",
      `tiebreaker ${bracket.tiebreaker} - ${bracket.file} @ byte ${number.format(bracket.starting_byte)}`);

    const view = el("details");
    view.dataset.place = bracket.place;
    view.open = firstRender ? bracket.place === 1 : wasOpen.has(String(bracket.place));
    const holder = el("div", "bracket-scroll");
    const draw = () => holder.replaceChildren(renderBracket(bracket.bracket, data.winning_bracket), legend());
    if (view.open) draw();
    view.addEventListener("toggle", () => {
      if (view.open && !holder.firstChild) draw();
    });
    view.append(el("summary", null, "View bracket"), holder);

    const raw = el("details");
    raw.append(el("summary", null, "Bracket data"), el("code", null, bracket.bracket));

    body.append(score, detail);
    head.append(el("div", "place", bracket.place), body);
    item.append(head, view, raw);
    return item;
  });
  list.replaceChildren(...items);
}

function renderFiles(data) {
  const body = document.querySelector("#files tbody");
  const rows = data.files.map((file) => {
    const row = el("tr");
    row.append(el("td", null, file.name), el("td", "num", formatBytes(file.bytes)));
    return row;
  });
  body.replaceChildren(...rows);

  const verify = document.getElementById("verify");
  verify.textContent = "All scored brackets come from these files. ";
  if (DRIVE_URL) {
    const link = el("a", null, "Download them to check for yourself.");
    link.href = DRIVE_URL;
    verify.append(link);
  }
}

function renderUpdated(data) {
  const when = new Date(data.updated_at * 1000);
  document.getElementById("updated").textContent = `Last updated ${when.toLocaleString()}`;
}

function render(data) {
  renderUpdated(data);
  renderStats(data);
  renderProgress(data);
  renderHistogram(data);
  renderTop(data);
  renderFiles(data);
}

async function load() {
  try {
    const response = await fetch(RESULTS_URL, { cache: "no-store" });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    latest = await response.json();
    render(latest);
  } catch (error) {
    const note = latest ? " (showing the last good data)" : "";
    document.getElementById("updated").textContent = `Could not load results${note}; retrying.`;
  }
}

document.getElementById("log-scale").addEventListener("change", () => {
  if (latest) renderHistogram(latest);
});

loadTeams().then(() => {
  load();
  setInterval(load, REFRESH_MS);
});
