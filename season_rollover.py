#!/usr/bin/env python3
"""Archive the live season into site/YYYY/ and reset the root page for the next season.

Usage: python3 season_rollover.py --year 2027 [--teams bracket_keys_2027.json] [--dry-run]
"""
import argparse
import json
import os
import re
import shutil
import sys
import tempfile
import time
from pathlib import Path

TEAM_KEYS = {str(i) for i in range(1, 65)}
BODY_RE = re.compile(r'<body data-year="(\d{4})" data-current="(?:true|false)">')
NAV_RE = re.compile(r'(<nav class="year-nav"[^>]*>)(.*?)(</nav>)', re.DOTALL)
SCORE_BINS = 193


def fail(message):
    sys.exit(f"error: {message}")


def read_json(path):
    try:
        return json.loads(Path(path).read_text())
    except (OSError, ValueError) as error:
        fail(f"cannot read {path}: {error}")


def check_teams(path):
    teams = read_json(path)
    valid = isinstance(teams, dict) and set(teams) == TEAM_KEYS and all(
        isinstance(name, str) and name.strip() for name in teams.values())
    if not valid:
        fail(f"{path} must map team keys 1-64 to non-empty names")


def write_atomic(path, text):
    fd, temp = tempfile.mkstemp(dir=path.parent, prefix=".rollover-")
    with os.fdopen(fd, "w") as file:
        file.write(text)
    os.chmod(temp, 0o644)  # mkstemp creates 0600, which the web server cannot read
    os.replace(temp, path)


def archived_years(site):
    return {int(p.name) for p in site.iterdir()
            if p.is_dir() and re.fullmatch(r"\d{4}", p.name) and (p / "index.html").is_file()}


def update_page(html, year, is_live, years, live_year):
    if not BODY_RE.search(html) or not NAV_RE.search(html):
        fail("page is missing the data-year body tag or year-nav block")
    html = BODY_RE.sub(f'<body data-year="{year}" data-current="{str(is_live).lower()}">', html, count=1)

    def nav(match):
        links = ""
        for y in sorted(years, reverse=True):
            href = "/" if y == live_year else f"/{y}/"
            links += f'\n    <a data-year-link="{y}" href="{href}">{y}</a>'
        return f"{match.group(1)}{links}\n  {match.group(3)}"

    return NAV_RE.sub(nav, html, count=1)


def empty_results():
    return {
        "updated_at": int(time.time()),
        "total_brackets": 0,
        "perfect_brackets": 0,
        "max_bracket_score": 0,
        "average_score": 0,
        "most_common_score": 0,
        "most_common_count": 0,
        "games_finished": 0,
        "winning_bracket": "",
        "score_distribution": [0] * SCORE_BINS,
        "top_brackets": [],
        "files": [],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--year", type=int, required=True, help="the new season to make live")
    parser.add_argument("--teams", type=Path, help="team keys for the new year (default: bracket_keys_YEAR.json)")
    parser.add_argument("--site", type=Path, default=Path("site"))
    parser.add_argument("--allow-incomplete", action="store_true", help="archive even if fewer than 63 games finished")
    parser.add_argument("--dry-run", action="store_true", help="validate and print the plan without writing")
    args = parser.parse_args()

    site = args.site
    root_page, root_results, root_teams = site / "index.html", site / "results.json", site / "teams.json"
    for required in (root_page, root_results, root_teams):
        if not required.is_file():
            fail(f"missing {required}")

    root_html = root_page.read_text()
    body = BODY_RE.search(root_html)
    if not body:
        fail(f"{root_page} has no data-year body tag")
    old_year = int(body.group(1))
    if args.year <= old_year:
        fail(f"new year {args.year} must be after the live year {old_year}")

    archive_dir = site / str(old_year)
    if archive_dir.exists():
        fail(f"{archive_dir} already exists; refusing to overwrite an archive")

    new_teams = args.teams or Path(f"bracket_keys_{args.year}.json")
    check_teams(root_teams)
    check_teams(new_teams)

    results = read_json(root_results)
    if results.get("total_brackets", 0) <= 0:
        fail(f"{root_results} has no scored brackets")
    if results.get("games_finished") != 63 and not args.allow_incomplete:
        fail(f"{old_year} has {results.get('games_finished')} of 63 games finished (use --allow-incomplete)")

    years = archived_years(site) | {old_year, args.year}
    print(f"archive {old_year}: {root_page}, {root_results}, {root_teams} -> {archive_dir}/")
    print(f"new live year {args.year}: teams from {new_teams}, empty results, navigation {sorted(years, reverse=True)}")
    if args.dry_run:
        return

    archive_dir.mkdir()
    try:
        write_atomic(archive_dir / "index.html", update_page(root_html, old_year, False, years, args.year))
        write_atomic(archive_dir / "results.json", root_results.read_text())
        write_atomic(archive_dir / "teams.json", root_teams.read_text())
    except BaseException:
        shutil.rmtree(archive_dir, ignore_errors=True)
        raise

    for year in archived_years(site) - {old_year}:
        page = site / str(year) / "index.html"
        write_atomic(page, update_page(page.read_text(), year, False, years, args.year))

    write_atomic(root_page, update_page(root_html, args.year, True, years, args.year))
    write_atomic(root_teams, new_teams.read_text())
    write_atomic(root_results, json.dumps(empty_results(), indent=2) + "\n")

    print(f"done. Next: create winning_bracket_{args.year}.txt, then score and export to {root_results}:")
    print(f"  ./target/release/march_madness --winning-bracket winning_bracket_{args.year}.txt "
          f"--bracket-dir {args.year}_brackets --export {root_results}")


if __name__ == "__main__":
    main()
