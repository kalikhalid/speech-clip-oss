#!/usr/bin/env python3
"""Пайплайн: backup → sub-агент генератор → sub-агент QA → dataset.v5.raw.tsv

Модель по умолчанию: gpt-5-mini (OpenAI data-sharing promo).

Запуск:
  export OPENAI_API_KEY=sk-...
  python3 dictation-normalizer/scripts/pipeline_openai.py --batches 80
"""

from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
ROOT = SCRIPT_DIR.parent
DATA = ROOT / "data"
sys.path.insert(0, str(SCRIPT_DIR))

from openai_agents import (  # noqa: E402
    DEFAULT_MODEL,
    GeneratorAgent,
    QAAgent,
    deterministic_filter,
)


def run_backup() -> None:
    subprocess.run([sys.executable, str(SCRIPT_DIR / "backup_dataset.py")], check=True)


def load_golden_pairs() -> list[tuple[str, str]]:
    """Эталонные пары: реальные искажения Parakeet + ручные gap-кейсы."""
    pairs: list[tuple[str, str]] = []
    distortions = DATA / "parakeet-distortions.tsv"
    gaps = DATA / "raw" / "agent_parakeet_gaps.tsv"
    for path in (distortions, gaps):
        if not path.exists():
            continue
        for line in path.read_text(encoding="utf-8").splitlines()[1:]:
            cols = line.split("\t")
            if len(cols) >= 2 and cols[0].strip() and cols[1].strip():
                pairs.append((cols[0].strip(), cols[1].strip()))
    return pairs


def write_tsv(path: Path, pairs: list[tuple[str, str]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as f:
        f.write("IN\tOUT\n")
        for a, b in pairs:
            f.write(a.replace("\t", " ") + "\t" + b.replace("\t", " ") + "\n")


def run_clean_dataset(raw_path: Path) -> None:
    clean_script = ROOT.parent / "scripts" / "clean_dataset.py"
    if not clean_script.exists():
        print("clean_dataset.py не найден, пропуск пост-обработки")
        return
    subprocess.run(
        [
            sys.executable,
            str(clean_script),
            "--input",
            str(raw_path),
            "--output",
            str(DATA / "dataset.v5.tsv"),
            "--train",
            str(DATA / "train.jsonl"),
            "--eval",
            str(DATA / "eval.jsonl"),
        ],
        check=True,
    )


def main() -> None:
    ap = argparse.ArgumentParser(description="OpenAI pipeline: generator + QA agents")
    ap.add_argument("--batches", type=int, default=80, help="число API-батчей (~40 пар каждый)")
    ap.add_argument("--model", default=DEFAULT_MODEL)
    ap.add_argument("--out-raw", default=str(DATA / "dataset.v5.raw.tsv"))
    ap.add_argument("--skip-backup", action="store_true")
    ap.add_argument("--skip-clean", action="store_true")
    ap.add_argument("--no-qa", action="store_true", help="только генератор + локальный фильтр")
    args = ap.parse_args()

    key = os.environ.get("OPENAI_API_KEY")
    if not key:
        print("Нужен OPENAI_API_KEY в окружении", file=sys.stderr)
        sys.exit(1)

    if not args.skip_backup:
        print("=== Backup ===")
        run_backup()

    out_raw = Path(args.out_raw)
    generator = GeneratorAgent(key, args.model)
    qa = QAAgent(key, args.model)

    seen: set[str] = set()
    all_pairs: list[tuple[str, str]] = []
    stats = {
        "generated": 0,
        "qa_approved": 0,
        "qa_rejected": 0,
        "local_rejected": 0,
        "tokens": 0,
    }
    qa_log: list[dict] = []

    print(f"=== Генерация ({args.batches} батчей, model={args.model}) ===")
    write_tsv(out_raw, [])

    with out_raw.open("w", encoding="utf-8") as f:
        f.write("IN\tOUT\n")
        for i in range(1, args.batches + 1):
            try:
                raw_pairs, gen_usage = generator.generate_batch()
            except Exception as exc:
                print(f"\n[batch {i}] generator error: {exc}; sleep 8s")
                time.sleep(8)
                continue

            stats["generated"] += len(raw_pairs)
            stats["tokens"] += gen_usage.get("total_tokens", 0)

            if args.no_qa:
                approved = raw_pairs
                rejected: list[dict] = []
                qa_usage = {}
            else:
                try:
                    approved, rejected, qa_usage = qa.review_batch(raw_pairs)
                except Exception as exc:
                    print(f"\n[batch {i}] QA error: {exc}; local filter only")
                    approved, _ = deterministic_filter(raw_pairs)
                    rejected = []
                    qa_usage = {}
                stats["tokens"] += qa_usage.get("total_tokens", 0)
                stats["qa_rejected"] += len(rejected)
                if rejected:
                    qa_log.append({"batch": i, "rejected": rejected[:10]})

            filtered, local_issues = deterministic_filter(approved)
            stats["local_rejected"] += len(approved) - len(filtered)
            if local_issues:
                qa_log.append({"batch": i, "local": local_issues[:5]})

            new = 0
            for a, b in filtered:
                k = a.lower()
                if k in seen:
                    continue
                seen.add(k)
                all_pairs.append((a, b))
                f.write(a.replace("\t", " ") + "\t" + b.replace("\t", " ") + "\n")
                new += 1
            f.flush()

            stats["qa_approved"] += new
            nc = sum(1 for a, b in all_pairs if a == b)
            print(
                f"\rбатч {i}/{args.batches} | +{new} | всего {len(all_pairs)} | "
                f"no-change {100*nc/max(1,len(all_pairs)):.0f}% | tokens {stats['tokens']}",
                end="",
                flush=True,
            )
            time.sleep(0.4)

    golden = load_golden_pairs()
    added_golden = 0
    with out_raw.open("a", encoding="utf-8") as f:
        for a, b in golden:
            k = a.lower()
            if k in seen:
                continue
            seen.add(k)
            all_pairs.append((a, b))
            f.write(a.replace("\t", " ") + "\t" + b.replace("\t", " ") + "\n")
            added_golden += 1
    print(f"\n\nСырой датасет: {out_raw} ({len(all_pairs)} пар, +{added_golden} golden)")
    log_path = DATA / "pipeline_qa_log.json"
    log_path.write_text(json.dumps({"stats": stats, "issues": qa_log}, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"QA log: {log_path}")
    print(json.dumps(stats, ensure_ascii=False, indent=2))

    if not args.skip_clean and len(all_pairs) > 100:
        print("\n=== clean_dataset.py ===")
        run_clean_dataset(out_raw)
        print("Готово: dataset.v5.tsv + train.jsonl + eval.jsonl")


if __name__ == "__main__":
    main()
