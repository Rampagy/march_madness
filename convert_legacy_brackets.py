#!/usr/bin/env python3
"""Temporary: convert legacy brackets (2024 text, 2025 binary v1) to binary v2.

Writes <folder>_v2/<n>_brackets.bin next to each input folder and never touches the inputs.
Brackets with out-of-range or impossible game picks are omitted and reported.
"""
import argparse
import os
import sys
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

import numpy as np

STARTING_BRACKET = np.array([
    1, 16, 8, 9, 5, 12, 4, 13, 6, 11, 3, 14, 7, 10, 2, 15,
    17, 32, 24, 25, 21, 28, 20, 29, 22, 27, 19, 30, 23, 26, 18, 31,
    33, 48, 40, 41, 37, 44, 36, 45, 38, 43, 35, 46, 39, 42, 34, 47,
    49, 64, 56, 57, 53, 60, 52, 61, 54, 59, 51, 62, 55, 58, 50, 63,
], dtype=np.int16)
V1_RECORD_BYTES = 64  # 63 bytes of (team + 32) then a newline
CHUNK_BYTES = 16 * 1024 * 1024  # multiple of V1_RECORD_BYTES

ALLOWED_TEXT_BYTES = np.zeros(256, dtype=bool)
ALLOWED_TEXT_BYTES[list(b"0123456789 ;\n")] = True


def encode_v2(teams):
    """teams: (n, 63) winners per game -> (n, 8) uint8 of big-endian v2 words."""
    n = teams.shape[0]
    normalized = np.zeros((n, 63), dtype=np.int16)
    bits = np.zeros((n, 64), dtype=np.uint8)  # bit 63 is unused padding
    valid_brackets = np.ones(n, dtype=bool)
    invalid_picks = 0

    for game_start, game_count in ((0, 32), (32, 16), (48, 8), (56, 4), (60, 2), (62, 1)):
        if game_start == 0:
            top = np.broadcast_to(STARTING_BRACKET[0::2], (n, 32))
            bottom = np.broadcast_to(STARTING_BRACKET[1::2], (n, 32))
        else:
            child_start = 2 * (game_start - 32)
            top = normalized[:, child_start:child_start + 2 * game_count:2]
            bottom = normalized[:, child_start + 1:child_start + 2 * game_count:2]

        picks = teams[:, game_start:game_start + game_count]
        second_won = picks == bottom
        valid = (second_won | (picks == top)) & (picks >= 1) & (picks <= 64)
        invalid_picks += int((~valid).sum())
        valid_brackets &= valid.all(axis=1)
        bits[:, game_start:game_start + game_count] = second_won
        normalized[:, game_start:game_start + game_count] = np.where(second_won, bottom, top)

    return np.packbits(bits, axis=1)[valid_brackets], int((~valid_brackets).sum()), invalid_picks


def decode_v1(chunk):
    records = np.frombuffer(chunk, dtype=np.uint8).reshape(-1, V1_RECORD_BYTES)
    if (records[:, 63] != ord("\n")).any():
        raise ValueError("not a version 1 bracket file (record does not end with a newline)")
    return records[:, :63].astype(np.int16) - 32


def decode_text(chunk):
    raw = np.frombuffer(chunk, dtype=np.uint8)
    if not ALLOWED_TEXT_BYTES[raw].all():
        raise ValueError("unexpected character in text bracket file")

    is_digit = (raw >= ord("0")) & (raw <= ord("9"))
    if (is_digit[2:] & is_digit[1:-1] & is_digit[:-2]).any():
        raise ValueError("team number with more than two digits")

    digit = np.where(is_digit, raw.astype(np.int16) - ord("0"), 0).astype(np.int16)
    value = digit.copy()
    value[1:] += 10 * digit[:-1]  # digit is 0 for non-digits, so only a preceding digit contributes
    ends_number = is_digit.copy()
    ends_number[:-1] &= ~is_digit[1:]

    teams = value[ends_number]
    lines = chunk.count(b"\n")
    if teams.size != lines * 63:
        raise ValueError(f"expected 63 teams per line, found {teams.size} teams on {lines} lines")
    return teams.reshape(-1, 63)


def text_chunks(f):
    carry = b""
    while True:
        data = f.read(CHUNK_BYTES)
        if not data:
            break
        data = carry + data
        cut = data.rfind(b"\n") + 1
        carry = data[cut:]
        if cut:
            yield data[:cut]
    if carry.strip():
        yield carry + b"\n"


def v1_chunks(f):
    while chunk := f.read(CHUNK_BYTES):
        yield chunk


def convert_file(src, dst):
    is_text = src.suffix == ".txt"
    if not is_text and src.stat().st_size % V1_RECORD_BYTES:
        raise ValueError(f"{src}: size is not a multiple of {V1_RECORD_BYTES} bytes")

    partial = dst.with_name(dst.name + ".part")
    count = 0
    discarded_brackets = 0
    invalid_picks = 0
    try:
        with open(src, "rb") as fin, open(partial, "wb") as fout:
            for chunk in (text_chunks(fin) if is_text else v1_chunks(fin)):
                teams = decode_text(chunk) if is_text else decode_v1(chunk)
                encoded, discarded, invalid = encode_v2(teams)
                fout.write(encoded.tobytes())
                discarded_brackets += discarded
                invalid_picks += invalid
                count += teams.shape[0]
        os.replace(partial, dst)
    except Exception as error:
        partial.unlink(missing_ok=True)
        raise ValueError(f"{src}: {error}") from error
    return src.name, count, discarded_brackets, invalid_picks


def find_inputs(folder):
    def number(path):
        prefix = path.name.split("_", 1)[0]
        return (int(prefix) if prefix.isdigit() else 1 << 60, path.name)

    with os.scandir(folder) as entries:
        files = [Path(e.path) for e in entries
                 if e.is_file() and "_brackets" in e.name and e.name.endswith((".txt", ".bin"))]
    return sorted(files, key=number)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("folders", nargs="*", type=Path, default=[Path("2024_brackets"), Path("2025_brackets")])
    parser.add_argument("--jobs", type=int, default=min(8, os.cpu_count() or 1))
    parser.add_argument("--force", action="store_true", help="rebuild outputs even if they already exist")
    args = parser.parse_args()

    work = []
    for folder in args.folders:
        out_dir = folder.with_name(folder.name + "_v2")
        out_dir.mkdir(exist_ok=True)
        for src in find_inputs(folder):
            dst = out_dir / (src.stem + ".bin")
            if dst.exists() and not args.force:
                print(f"skip (already converted): {dst}")
            else:
                work.append((src, dst))

    failed = 0
    with ProcessPoolExecutor(max_workers=args.jobs) as pool:
        futures = {pool.submit(convert_file, src, dst): src for src, dst in work}
        for done, future in enumerate(as_completed(futures), 1):
            try:
                name, count, discarded, invalid_picks = future.result()
                note = f"; discarded {discarded:,} brackets with {invalid_picks:,} invalid picks" if discarded else ""
                print(f"[{done}/{len(work)}] {futures[future].parent.name}/{name}: kept {count - discarded:,}/{count:,} brackets{note}")
            except Exception as error:
                failed += 1
                print(f"[{done}/{len(work)}] FAILED {error}", file=sys.stderr)

    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
