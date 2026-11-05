#!/usr/bin/env python3
"""Build winning_bracket.txt from ESPN's public NCAA tournament scoreboard.

Usage: python3 update_winning_bracket.py --year 2027 [--regions East,South,West,Midwest] [--dry-run]

Each key block (1-16, 17-32, 33-48, 49-64) is matched to an ESPN region by finding which region's
1-seed is the team holding that block's 1-seed key in bracket_keys_YEAR.json. Unplayed games are 0.
"""
import argparse
import concurrent.futures
import datetime
import json
import os
import re
import sys
import tempfile
import time
import urllib.error
import urllib.request
from pathlib import Path

SEED_ORDER = [1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15]
START = [16 * block + seed for block in range(4) for seed in SEED_ORDER]
ROUNDS = {
    "1st Round": 1, "Round of 64": 1, "2nd Round": 2, "Round of 32": 2,
    "Sweet 16": 3, "Elite 8": 4, "Elite Eight": 4, "Final Four": 5, "National Championship": 6,
}
ROUND_BASE = {1: 0, 2: 32, 3: 48, 4: 56}
ROUND_SIZE = {1: 8, 2: 4, 3: 2, 4: 1}
URL = "https://site.api.espn.com/apis/site/v2/sports/basketball/mens-college-basketball/scoreboard?dates={date}&limit=300"
HEADLINE_RE = re.compile(r"Basketball Championship - (?:(.+?) Region - )?(.+)$")
NAME_WORDS = {"n": "north", "st": "state"}


def fail(message):
    sys.exit(f"error: {message}")


def fetch(url):
    request = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
    for attempt in range(3):
        try:
            with urllib.request.urlopen(request, timeout=30) as response:
                return json.load(response)
        except (urllib.error.URLError, TimeoutError, ValueError) as error:
            if attempt == 2:
                fail(f"cannot fetch {url}: {error}")
            time.sleep(2 ** attempt)


def parse_game(event):
    competition = event["competitions"][0]
    notes = competition.get("notes") or []
    headline = notes[0].get("headline", "") if notes else ""
    if "Basketball Championship" not in headline:
        return None
    match = HEADLINE_RE.search(headline)
    if not match:
        fail(f"unrecognized tournament headline: {headline!r}")
    region, round_name = match.groups()
    if round_name == "First Four":
        return None  # the winner plays the 1st Round game under the same seed
    if round_name not in ROUNDS:
        fail(f"unrecognized tournament round in {headline!r}")

    teams = []
    for competitor in competition["competitors"]:
        team = competitor["team"]
        seed = (competitor.get("curatedRank") or {}).get("current")
        names = {team.get(key) for key in ("location", "shortDisplayName", "displayName", "abbreviation")}
        teams.append({
            "id": team["id"],
            "seed": seed if isinstance(seed, int) and 1 <= seed <= 16 else None,
            "names": {name for name in names if name},
            "label": team.get("displayName", team["id"]),
            "winner": bool(competitor.get("winner")),
        })
    return {
        "id": event["id"],
        "round": ROUNDS[round_name],
        "region": region,
        "teams": teams,
        "done": bool(event["status"]["type"].get("completed")),
        "headline": headline,
    }


def fetch_games(start, end):
    days = [start + datetime.timedelta(days=n) for n in range((end - start).days + 1)]
    urls = [URL.format(date=day.strftime("%Y%m%d")) for day in days]
    games = {}
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        for data in pool.map(fetch, urls):
            for event in data.get("events", []):
                game = parse_game(event)
                if game:
                    games[game["id"]] = game
    return list(games.values())


def normalize(name):
    words = re.sub(r"[^a-z0-9 ]", " ", name.lower().replace("'", "")).split()
    return " ".join(NAME_WORDS.get(word, word) for word in words)


def regions_from_keys(keys_path, games):
    keys = json.loads(Path(keys_path).read_text())
    regions = []
    for block in range(4):
        key_name = keys[str(16 * block + 1)]
        found = set()
        for game in games:
            if game["round"] != 1:
                continue
            for team in game["teams"]:
                if team["seed"] == 1 and normalize(key_name) in {normalize(n) for n in team["names"]}:
                    found.add(game["region"])
        if len(found) != 1:
            fail(f"cannot match '{key_name}' (key {16 * block + 1}) to exactly one region; pass --regions")
        regions.append(found.pop())
    if len(set(regions)) != 4:
        fail(f"key blocks matched duplicate regions {regions}; pass --regions")
    return regions


def build_bracket(games, regions):
    block_of = {name: index for index, name in enumerate(regions)}
    team_block = {}
    for game in games:
        if game["region"] is None:
            continue
        if game["region"] not in block_of:
            fail(f"region '{game['region']}' is not in --regions {regions}")
        for team in game["teams"]:
            team_block[team["id"]] = block_of[game["region"]]

    bracket = [0] * 63
    for game in games:
        round_number = game["round"]
        if round_number == 1 and all(team["seed"] for team in game["teams"]):
            first, second = (SEED_ORDER.index(team["seed"]) for team in game["teams"])
            if first // 2 != second // 2 or first == second:
                fail(f"unexpected 1st Round matchup (seed data unreliable?): {game['headline']}")
        if round_number == 5:
            blocks = [team_block[t["id"]] for t in game["teams"] if t["id"] in team_block]
            if len(blocks) == 2 and blocks[0] // 2 != blocks[1] // 2:
                fail("the Final Four pairs regions differently than this key sheet's blocks (0+1, 2+3); "
                     f"check --regions {regions}")

        winner = next((t for t in game["teams"] if t["winner"]), None) if game["done"] else None
        if winner is None:
            continue
        if winner["seed"] is None:
            fail(f"no tournament seed for {winner['label']} ({game['headline']})")

        if round_number <= 4:
            block = block_of[game["region"]]
            position = SEED_ORDER.index(winner["seed"]) // 2 ** round_number
            index = ROUND_BASE[round_number] + block * ROUND_SIZE[round_number] + position
        else:
            block = team_block[winner["id"]]
            index = 60 + block // 2 if round_number == 5 else 62
        if bracket[index]:
            fail(f"two results map to game {index}: {game['headline']}")
        bracket[index] = 16 * block + winner["seed"]
    return bracket


def validate(bracket):
    for game, winner in enumerate(bracket):
        if not winner:
            continue
        if game < 32:
            entrants = START[2 * game], START[2 * game + 1]
        else:
            entrants = bracket[2 * (game - 32)], bracket[2 * (game - 32) + 1]
        if winner not in entrants:
            fail(f"game {game}: winner {winner} did not play in that game (a feeder result is missing?)")


def render(bracket):
    positions = [3 * i + i // 2 + 2 * (i // 8) for i in range(32)]
    for game in range(32, 63):
        positions.append((positions[2 * (game - 32)] + positions[2 * (game - 32) + 1]) // 2)
    lines = []
    for start, stop in ((0, 32), (32, 48), (48, 56), (56, 60), (60, 62), (62, 63)):
        row = [" "] * (positions[stop - 1] + 2)
        for game in range(start, stop):
            row[positions[game]:positions[game] + 2] = str(bracket[game]).rjust(2)
        lines.append("".join(row).rstrip())
    return ";\n".join(lines) + "\n"


def write_atomic(path, text):
    fd, temp = tempfile.mkstemp(dir=path.resolve().parent, prefix=".winning-bracket-")
    with os.fdopen(fd, "w") as file:
        file.write(text)
    os.chmod(temp, 0o644)
    os.replace(temp, path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--year", type=int, required=True)
    parser.add_argument("--regions", help="ESPN region names for key blocks 1-16,17-32,33-48,49-64, comma separated")
    parser.add_argument("--keys", type=Path, help="team key file (default: bracket_keys_YEAR.json)")
    parser.add_argument("--output", type=Path, default=Path("winning_bracket.txt"))
    parser.add_argument("--dry-run", action="store_true", help="print the bracket instead of writing it")
    args = parser.parse_args()

    games = fetch_games(datetime.date(args.year, 3, 12), datetime.date(args.year, 4, 12))
    if not games:
        fail(f"no NCAA tournament games found for {args.year}")

    if args.regions:
        regions = [name.strip() for name in args.regions.split(",")]
        if len(regions) != 4:
            fail("--regions needs exactly four names")
    else:
        regions = regions_from_keys(args.keys or Path(f"bracket_keys_{args.year}.json"), games)

    bracket = build_bracket(games, regions)
    validate(bracket)
    text = render(bracket)

    finished = sum(1 for winner in bracket if winner)
    print(f"{finished} of 63 games finished; key blocks 1-16/17-32/33-48/49-64 = {', '.join(regions)}", file=sys.stderr)
    if args.dry_run:
        print(text, end="")
    else:
        write_atomic(args.output, text)
        print(f"wrote {args.output}", file=sys.stderr)


if __name__ == "__main__":
    main()
