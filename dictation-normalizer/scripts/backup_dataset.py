#!/usr/bin/env python3
"""Сохранить текущий датасет в data/archive/ перед перегенерацией."""

from __future__ import annotations

import shutil
from datetime import datetime
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DATA = ROOT / "data"
ARCHIVE = DATA / "archive"

FILES = [
    "dataset.tsv",
    "dataset.v3.tsv",
    "dataset.v4.tsv",
    "train.jsonl",
    "eval.jsonl",
    "train.jsonl.v3.bak",
    "eval.jsonl.v3.bak",
    "parakeet-distortions.tsv",
    "test-phrases.tsv",
]


def main() -> None:
    stamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    dest = ARCHIVE / f"pre-openai-{stamp}"
    dest.mkdir(parents=True, exist_ok=True)

    copied = 0
    for name in FILES:
        src = DATA / name
        if src.exists():
            shutil.copy2(src, dest / name)
            copied += 1

    raw_src = DATA / "raw"
    if raw_src.exists():
        shutil.copytree(raw_src, dest / "raw", dirs_exist_ok=True)
        copied += 1

    print(f"Архив: {dest}")
    print(f"Скопировано элементов: {copied}")


if __name__ == "__main__":
    main()
