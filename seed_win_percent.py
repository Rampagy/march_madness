#!/usr/bin/env python3
import argparse
import math
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

ROUND_SIZES = [32, 16, 8, 4, 2, 1]
FIRST_ROUND_PAIRS = [
    (1, 16),
    (8, 9),
    (5, 12),
    (4, 13),
    (6, 11),
    (3, 14),
    (7, 10),
    (2, 15),
]


def team_seed(team_id: int) -> int:
    """Return the seed number 1-16 for a given team id 1-64."""
    if not 1 <= team_id <= 64:
        raise ValueError(f"Invalid team id: {team_id}")
    return ((team_id - 1) % 16) + 1


def build_first_round_participants() -> list[int]:
    """Build the 64-team initial bracket ordering for first-round matchups."""
    participants: list[int] = []
    for region_offset in (0, 16, 32, 48):
        for a, b in FIRST_ROUND_PAIRS:
            participants.append(a + region_offset)
            participants.append(b + region_offset)
    return participants


def parse_bracket_file(path: Path) -> list[list[int]]:
    """Parse a winning bracket text file into a list of round-by-round winner ids.

    The stored file contains only winners for each round, starting with 32
    round-1 winners followed by 16, 8, 4, 2, and 1 winners.
    """
    rounds: list[list[int]] = []
    with path.open("r", encoding="utf-8") as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            if ";" in line:
                line = line.split(";", 1)[0].strip()
            if not line:
                continue
            winners = [int(tok) for tok in line.split() if tok]
            rounds.append(winners)

    sizes = [len(r) for r in rounds]
    if sizes != ROUND_SIZES:
        raise ValueError(
            f"Unexpected round sizes in {path.name}: {sizes}. "
            f"Expected {ROUND_SIZES} for winner-only bracket files"
        )

    if any(value <= 0 for winner in rounds for value in winner):
        raise ValueError(f"Invalid winner ID found in {path.name}: all IDs must be positive")

    return rounds


def extract_match_results(rounds: list[list[int]]) -> list[tuple[int, int]]:
    """Return a list of (winner_id, loser_id) tuples for every game in the bracket.

    The stored winner-only files include only winners, starting with round 1.
    """
    matches: list[tuple[int, int]] = []
    participants = build_first_round_participants()

    for round_index, winners in enumerate(rounds, start=1):
        expected = len(participants) // 2
        if len(winners) != expected:
            raise ValueError(
                f"Round {round_index} expected {expected} winners, got {len(winners)}"
            )

        next_participants: list[int] = []
        for i in range(expected):
            a = participants[2 * i]
            b = participants[2 * i + 1]
            winner = winners[i]
            if winner not in (a, b):
                raise ValueError(
                    f"Winner {winner} is not in the matchup ({a}, {b}) "
                    f"for round {round_index}, game {i + 1}"
                )
            loser = b if winner == a else a
            matches.append((winner, loser))
            next_participants.append(winner)

        participants = next_participants

    return matches


def summarize_seed_win_percent(files: list[Path]) -> tuple[np.ndarray, np.ndarray]:
    """Compute win counts and win percentage matrix for seeds 1-16."""
    win_counts = np.zeros((16, 16), dtype=int)
    games_played = np.zeros((16, 16), dtype=int)

    for path in files:
        rounds = parse_bracket_file(path)
        results = extract_match_results(rounds)
        for winner_id, loser_id in results:
            winner_seed = team_seed(winner_id) - 1
            loser_seed = team_seed(loser_id) - 1
            win_counts[winner_seed, loser_seed] += 1
            games_played[winner_seed, loser_seed] += 1
            games_played[loser_seed, winner_seed] += 1

    win_percent = np.full((16, 16), np.nan, dtype=float)
    for i in range(16):
        for j in range(16):
            total = games_played[i, j]
            if total:
                win_percent[i, j] = win_counts[i, j] / total

    return win_counts, win_percent


# Logistic steepness used only for seed pairs that never met in the data.
FALLBACK_STEEPNESS = 0.15


def build_probability_table(win_percent: np.ndarray) -> np.ndarray:
    """Use observed win rates directly; fill unseen pairs with a logistic model."""
    table = build_seed_prior()
    observed = ~np.isnan(win_percent)
    table[observed] = win_percent[observed]
    return table


def build_seed_prior() -> np.ndarray:
    """Build the logistic seed-difference prior for every seed matchup."""
    table = np.empty((16, 16), dtype=float)
    for i in range(16):
        for j in range(16):
            table[i, j] = 1.0 / (1.0 + math.exp(-FALLBACK_STEEPNESS * (j - i)))
    return table


def build_smoothed_probability_table(
    win_counts: np.ndarray,
    prior_strength: float = 4.0,
) -> np.ndarray:
    """Blend observed matchup rates with the logistic seed-difference prior."""
    if prior_strength < 0:
        raise ValueError("prior_strength must be non-negative")

    prior = build_seed_prior()
    games_played = win_counts + win_counts.T
    denominator = games_played + prior_strength
    numerator = win_counts + prior_strength * prior
    return np.divide(numerator, denominator, out=prior.copy(), where=denominator > 0)


def brier_score(table: np.ndarray, matches: list[tuple[int, int]]) -> float:
    """Score held-out game predictions using the lower seed as canonical first team."""
    squared_errors = []
    for winner_id, loser_id in matches:
        winner_seed = team_seed(winner_id)
        loser_seed = team_seed(loser_id)
        if winner_seed < loser_seed:
            probability = table[winner_seed - 1, loser_seed - 1]
            outcome = 1.0
        else:
            probability = table[loser_seed - 1, winner_seed - 1]
            outcome = 0.0
        squared_errors.append((probability - outcome) ** 2)
    if not squared_errors:
        raise ValueError("holdout contains no match results")
    return float(np.mean(squared_errors))


def write_rust_table(table: np.ndarray, path: Path) -> None:
    """Write the probability table as a Rust source file included by main.rs."""
    width = 8
    lines = [
        "// Generated by seed_win_percent.py; do not edit.",
        "// Rows: winning seed. Columns: opponent seed.",
        "const SEED_WIN_PROBABILITY: [[f64; 16]; 16] = [",
        "    // opp: " + " ".join(f"{s:<{width + 1}}" for s in range(1, 17)).rstrip(),
    ]
    for seed, row in enumerate(table, start=1):
        cells = (f"{v:.6f}" for v in row)
        body = " ".join(f"{c + ',':<{width + 1}}" for c in cells).rstrip()
        lines.append(f"    [{body}], // seed {seed}")
    lines.append("];")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def plot_win_percent_matrix(win_percent: np.ndarray, title: str) -> None:
    """Plot a seed-vs-seed probability matrix using matplotlib."""
    fig, ax = plt.subplots(figsize=(10, 9))
    cmap = plt.cm.viridis
    im = ax.imshow(win_percent, cmap=cmap, vmin=0.0, vmax=1.0)

    seed_labels = [str(s) for s in range(1, 17)]
    ax.set_xticks(np.arange(16))
    ax.set_yticks(np.arange(16))
    ax.set_xticklabels(seed_labels)
    ax.set_yticklabels(seed_labels)
    ax.set_xlabel("Opponent Seed")
    ax.set_ylabel("Winning Seed")
    ax.set_title(title)

    plt.setp(ax.get_xticklabels(), rotation=45, ha="right", rotation_mode="anchor")

    for i in range(16):
        for j in range(16):
            value = win_percent[i, j]
            if not math.isnan(value):
                text = f"{value*100:4.1f}%"
            else:
                text = "-"
            ax.text(j, i, text, ha="center", va="center", color="white" if value >= 0.5 else "black")

    cbar = fig.colorbar(im, ax=ax)
    cbar.set_label("Win Percentage")


if __name__ == "__main__":
    root = Path(__file__).resolve().parent
    files = sorted(root.glob("winning_bracket_[0-9][0-9][0-9][0-9].txt"))
    if not files:
        raise SystemExit("No valid winning_bracket_[0-9][0-9][0-9][0-9].txt files found")

    parser = argparse.ArgumentParser()
    parser.add_argument("--plot", action="store_true", help="show the win percent matrix")
    parser.add_argument("--compare-holdout", type=Path, help="compare raw and smoothed tables on this bracket without training on it")
    parser.add_argument("--prior-strength", type=float, default=4.0, help="pseudo-game weight for the logistic prior (default: 4)")
    parser.add_argument("--output", type=Path, help="Rust table output path (default: src/seed_win_probabilities.rs)")
    args = parser.parse_args()

    win_counts, win_percent = summarize_seed_win_percent(files)
    probabilities = build_probability_table(win_percent)

    if args.compare_holdout:
        holdout_rounds = parse_bracket_file(args.compare_holdout)
        holdout_matches = extract_match_results(holdout_rounds)
        smoothed = build_smoothed_probability_table(win_counts, args.prior_strength)
        raw_brier = brier_score(probabilities, holdout_matches)
        smoothed_brier = brier_score(smoothed, holdout_matches)
        print(f"Training years: {', '.join(path.stem.removeprefix('winning_bracket_') for path in files)}")
        print(f"Holdout: {args.compare_holdout} ({len(holdout_matches)} games)")
        print(f"Raw table Brier score: {raw_brier:.6f}")
        print(f"Smoothed table Brier score (prior strength {args.prior_strength:g}): {smoothed_brier:.6f}")
        print(f"Brier change (smoothed - raw): {smoothed_brier - raw_brier:+.6f}")

    output_table = build_smoothed_probability_table(win_counts, args.prior_strength)
    out_path = args.output or (root / "src" / "seed_win_probabilities.rs")
    write_rust_table(output_table, out_path)
    table_name = f"smoothed (k={args.prior_strength:g})"
    print(f"Wrote {table_name} probability table to {out_path}")

    if args.plot:
        plot_win_percent_matrix(win_percent, "Observed Win Rates by Seed Matchup")
        plot_win_percent_matrix(output_table, f"Probabilities Written to Rust ({table_name})")
        plt.show()
