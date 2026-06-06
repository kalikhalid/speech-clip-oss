#!/usr/bin/env python3
"""Merge raw agent TSV files into a single deduplicated dataset."""

import csv
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RAW_DIR = ROOT / "data" / "raw"
OUT_FILE = ROOT / "data" / "dataset.tsv"


def load_rows(path: Path) -> list[tuple[str, str]]:
    rows = []
    with path.open(encoding="utf-8") as f:
        reader = csv.reader(f, delimiter="\t")
        header = next(reader, None)
        for line in reader:
            if len(line) < 2:
                continue
            in_text, out_text = line[0].strip(), line[1].strip()
            if not in_text:
                continue
            rows.append((in_text, out_text))
    return rows


def main() -> None:
    raw_files = sorted(RAW_DIR.glob("agent_*.tsv"))
    if not raw_files:
        print(f"No agent files in {RAW_DIR}", file=sys.stderr)
        sys.exit(1)

    seen: set[str] = set()
    merged: list[tuple[str, str]] = []
    stats: dict[str, int] = {}

    for path in raw_files:
        count = 0
        for row in load_rows(path):
            key = row[0]
            if key in seen:
                continue
            seen.add(key)
            merged.append(row)
            count += 1
        stats[path.name] = count

    OUT_FILE.parent.mkdir(parents=True, exist_ok=True)
    with OUT_FILE.open("w", encoding="utf-8", newline="") as f:
        writer = csv.writer(f, delimiter="\t", lineterminator="\n")
        writer.writerow(["IN", "OUT"])
        writer.writerows(merged)

    pure_ru = sum(1 for i, o in merged if i == o)
    changed = len(merged) - pure_ru

    print(f"Merged {len(merged)} unique rows → {OUT_FILE}")
    print(f"  Changed: {changed} ({100*changed/len(merged):.1f}%)")
    print(f"  Pure Russian (IN==OUT): {pure_ru} ({100*pure_ru/len(merged):.1f}%)")
    print("Per file:")
    for name, n in stats.items():
        print(f"  {name}: {n} rows accepted")


if __name__ == "__main__":
    main()
