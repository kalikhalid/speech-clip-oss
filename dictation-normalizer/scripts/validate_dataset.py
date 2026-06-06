#!/usr/bin/env python3
"""Quick automated QA for IN/OUT dataset pairs."""

import csv
import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATASET = ROOT / "data" / "dataset.tsv"

# Common wrongly translated Russian→English in OUT
TRANSLATED_VERBS = re.compile(
    r"\b(make|check|open|close|run|start|stop|create|delete|update|send|get|set|add|remove|fix|build|deploy|push|pull|merge)\b",
    re.I,
)

# File-like patterns in OUT without @
FILE_EXT = re.compile(
    r"(?<![@/\w])(README\.md|package\.json|Cargo\.toml|Cargo\.lock|\.env|\.gitignore|docker-compose\.yml|tsconfig\.json|\.yaml|\.yml|\.json|\.md|\.toml|\.lock)\b",
    re.I,
)

# Latin letters in IN (terms should be Cyrillic phonetic)
LATIN_IN = re.compile(r"[a-zA-Z]{2,}")


def load() -> list[tuple[str, str]]:
    rows = []
    with DATASET.open(encoding="utf-8") as f:
        reader = csv.reader(f, delimiter="\t")
        next(reader)
        for line in reader:
            if len(line) >= 2:
                rows.append((line[0], line[1]))
    return rows


def main() -> None:
    rows = load()
    issues: list[tuple[int, str, str, str]] = []

    for i, (inp, out) in enumerate(rows, 1):
        if inp == out:
            continue

        if TRANSLATED_VERBS.search(out):
            # only flag if verb not in IN as latin
            for m in TRANSLATED_VERBS.finditer(out):
                if m.group().lower() not in inp.lower():
                    issues.append((i, inp, out, f"ПЕРЕВОД_РУССКОГО? verb '{m.group()}' in OUT"))
                    break

        for m in FILE_EXT.finditer(out):
            start = m.start()
            if start == 0 or out[start - 1] != "@":
                issues.append((i, inp, out, f"ФАЙЛ_БЕЗ_СОБАЧКИ: {m.group()}"))
                break

        if LATIN_IN.search(inp):
            issues.append((i, inp, out, "ТЕРМИН_НЕ_ОЗВУЧЕН_В_IN (latin in IN)"))

    pure = sum(1 for i, o in rows if i == o)
    print(f"Total: {len(rows)}, IN==OUT: {pure} ({100*pure/len(rows):.1f}%)")
    print(f"Issues found: {len(issues)}\n")
    for num, inp, out, kind in issues[:40]:
        print(f"#{num} [{kind}]")
        print(f"  IN:  {inp[:100]}")
        print(f"  OUT: {out[:100]}")
        print()
    if len(issues) > 40:
        print(f"... and {len(issues)-40} more")


if __name__ == "__main__":
    main()
