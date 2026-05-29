#!/usr/bin/env python3
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
            else:
                # no data, just pick a some random numbers
                #if i < j:
                #    win_percent[i, j] = 0.99
                #elif i > j:
                #    win_percent[i, j] = 0.01
                #else:
                #    win_percent[i, j] = 0.5
                pass

    # Avoid exact 0 or 1 values by nudging them to 0.01 and 0.99.
    win_percent = np.where(win_percent == 0.0, 0.01, win_percent)
    win_percent = np.where(win_percent == 1.0, 0.99, win_percent)

    return win_counts, win_percent


def format_percent_table(win_percent: np.ndarray) -> str:
    """Create a readable text table for the seed win percent matrix."""
    header = ["Seed"] + [f"{s:2d}" for s in range(1, 17)]
    lines = ["  ".join(header)]
    for seed in range(16):
        row = [f"{seed+1:2d}"]
        for opponent in range(16):
            pct = win_percent[seed, opponent]
            if math.isnan(pct):
                row.append("  -")
            else:
                row.append(f"{pct*100:4.1f}")
        lines.append("  ".join(row))
    return "\n".join(lines)


def plot_win_percent_matrix(win_percent: np.ndarray) -> None:
    """Plot the seed-vs-seed win percent matrix using matplotlib."""
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
    ax.set_title("Win Percentage by Seed Against Each Opponent Seed")

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

    win_counts, win_percent = summarize_seed_win_percent(files)
    plot_win_percent_matrix(win_percent)
    plt.show()
