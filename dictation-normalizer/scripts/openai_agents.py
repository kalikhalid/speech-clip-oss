#!/usr/bin/env python3
"""Sub-агенты OpenAI: генератор пар IN/OUT и QA-проверяльщик."""

from __future__ import annotations

import json
import os
import random
import re
import urllib.error
import urllib.request
from pathlib import Path

from term_bank import FILE_TERMS, TERMS

API = "https://api.openai.com/v1/chat/completions"
DEFAULT_MODEL = os.environ.get("OPENAI_DATA_MODEL", "gpt-5-mini")

ROOT = Path(__file__).resolve().parent.parent
DISTORTIONS = ROOT / "data" / "parakeet-distortions.tsv"

GENERATOR_SYS = """Ты — агент-генератор обучающих пар для post-ASR нормализатора (dev-диктовка).
Цель: Wispr-flow-like пост-обработка — минимальные правки поверх сырого ASR, без перефразирования.

ФОРМАТ: JSONL, каждая строка {"in":"...","out":"..."}. Без markdown.

ПРАВИЛА:
1. in = сырой вывод ASR (Parakeet): термины кириллицей по звучанию + реальные искажения.
2. out = та же фраза: термины латиницей/как код; русские слова и порядок слов БЕЗ изменений.
3. Не переводи русский: «сделай коммит»→«сделай commit», НЕ «make commit».
4. Файлы в out ВСЕГДА с @: ридми эм дэ→@README.md; если in уже latin file.py→out @file.py.
5. Паттерны «который называется …», «скрипт … .py», «отработал лискрипт» — включай часто.
6. ASR-грязь ~35%: ридми→ритми, гит→гид, запушфорс, докер комполз, лискрипт, отработалли.
7. Русифицированные глаголы (запушил, закоммитил) — не трогать.
8. Существительные-термины → латиница: коммит→commit, докер→docker.
9. Без смешения алфавитов в одном слове (запрещено testы, diffы).
10. Если в in нет термина — out == in.
11. Пунктуация in/out совпадает 1:1, меняется только написание терминов."""

QA_SYS = """Ты — строгий QA-агент датасета post-ASR нормализатора (dev, русская речь + tech terms).

Проверь каждую пару IN→OUT. Верни ТОЛЬКО JSON:
{
  "approved": [{"in":"...","out":"..."}],
  "rejected": [{"in":"...","out":"...","error":"ТИП: пояснение"}]
}

Типы ошибок: ПЕРЕВОД_РУССКОГО, ИЗМЕНЁН_СМЫСЛ, ФАЙЛ_БЕЗ_СОБАЧКИ, ДОЛЖНО_БЫТЬ_БЕЗ_ИЗМЕНЕНИЙ,
СМЕШАННЫЙ_АЛФАВИТ, ТЕРМИН_НЕ_ИСПРАВЛЕН, ПРОЧЕЕ.

Правила:
- OUT меняет только термины; русский не переводится.
- Файлы в OUT: @README.md, @package.json — без code fences.
- IN==OUT если нет терминов.
- Русифицированные глаголы в out остаются кириллицей.
Одобряй только корректные пары."""


def _chat(key: str, model: str, system: str, user: str, max_tokens: int = 9000) -> tuple[str, dict]:
    body = json.dumps({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user},
        ],
        "max_completion_tokens": max_tokens,
        "reasoning_effort": "minimal",
    }).encode()
    req = urllib.request.Request(
        API,
        data=body,
        headers={"Authorization": f"Bearer {key}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=180) as resp:
        data = json.load(resp)
    return data["choices"][0]["message"]["content"], data.get("usage", {})


def load_history_samples(limit: int = 80) -> list[str]:
    paths = [
        Path.home() / "Library/Application Support/dev.speechclip.oss/history.json",
        Path.home() / "Library/Application Support/com.speechclip.app/history.json",
    ]
    samples: list[str] = []
    for path in paths:
        try:
            payload = json.loads(path.read_text(encoding="utf-8"))
            entries = payload.get("entries", payload) if isinstance(payload, dict) else payload
            for entry in entries:
                raw = str(entry.get("raw_text", "")).strip()
                if raw and re.search(r"[а-яё]{3,}", raw, re.I) and len(raw) > 8:
                    samples.append(raw)
        except OSError:
            continue
    random.shuffle(samples)
    return samples[:limit]


def load_distortion_examples(limit: int = 12) -> list[tuple[str, str, str]]:
    if not DISTORTIONS.exists():
        return []
    rows: list[tuple[str, str, str]] = []
    for line in DISTORTIONS.read_text(encoding="utf-8").splitlines()[1:]:
        parts = line.split("\t")
        if len(parts) >= 2:
            note = parts[2] if len(parts) > 2 else ""
            rows.append((parts[0].strip(), parts[1].strip(), note))
    return rows[:limit]


def parse_jsonl_pairs(text: str) -> list[tuple[str, str]]:
    pairs: list[tuple[str, str]] = []
    for line in text.splitlines():
        line = line.strip().strip(",")
        if not (line.startswith("{") and line.endswith("}")):
            continue
        try:
            item = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(item, dict) and "in" in item and "out" in item:
            a, b = str(item["in"]).strip(), str(item["out"]).strip()
            if a and b:
                pairs.append((a, b))
    return pairs


def parse_qa_json(text: str) -> tuple[list[tuple[str, str]], list[dict]]:
    text = text.strip()
    if text.startswith("```"):
        text = re.sub(r"^```(?:json)?\s*|\s*```$", "", text, flags=re.S).strip()
    try:
        data = json.loads(text)
    except json.JSONDecodeError:
        return [], [{"error": "QA_INVALID_JSON", "raw": text[:500]}]
    approved = []
    for item in data.get("approved", []):
        if isinstance(item, dict) and item.get("in") and item.get("out"):
            approved.append((str(item["in"]).strip(), str(item["out"]).strip()))
    rejected = [r for r in data.get("rejected", []) if isinstance(r, dict)]
    return approved, rejected


class GeneratorAgent:
    def __init__(self, api_key: str, model: str = DEFAULT_MODEL):
        self.api_key = api_key
        self.model = model
        self.history = load_history_samples()
        self.distortions = load_distortion_examples()

    def _user_prompt(self, terms: list[str], files: list[str]) -> str:
        dist_block = ""
        if self.distortions:
            dist_block = "\n\nЭталоны реальных искажений Parakeet (копируй стиль in):\n"
            dist_block += "\n".join(
                f"  IN: {a} → OUT: {b} ({note})" for a, b, note in self.distortions[:8]
            )
        hist_block = ""
        if self.history:
            picked = random.sample(self.history, min(6, len(self.history)))
            hist_block = "\n\nРеальные raw_text из history.json:\n" + "\n".join(f"  «{s}»" for s in picked)

        return f"""Сгенерируй 40 пар IN/OUT для dev post-ASR нормализатора.

ТЕРМИНЫ (озвучивай по-русски в in): {', '.join(terms)}
ФАЙЛЫ (в out только через @): {', '.join(files)}{dist_block}{hist_block}

Состав 40 пар:
- 20: команды с 1-3 терминами (git, docker, frontend, backend, tools).
- 8: чистый русский, in == out.
- 6: длинные цепочки 20-50 слов, несколько терминов.
- 4: ловушки (русское слово рядом с термином).
- 2: «отработал лискрипт / который называется file.py» (latin file → @file в out).

Верни ТОЛЬКО JSONL."""

    def generate_batch(self) -> tuple[list[tuple[str, str]], dict]:
        terms = random.sample(TERMS, 12)
        files = random.sample(FILE_TERMS, 5)
        text, usage = _chat(self.api_key, self.model, GENERATOR_SYS, self._user_prompt(terms, files))
        return parse_jsonl_pairs(text), usage


class QAAgent:
    def __init__(self, api_key: str, model: str = DEFAULT_MODEL):
        self.api_key = api_key
        self.model = model

    def review_batch(self, pairs: list[tuple[str, str]]) -> tuple[list[tuple[str, str]], list[dict], dict]:
        if not pairs:
            return [], [], {}
        lines = "\n".join(json.dumps({"in": a, "out": b}, ensure_ascii=False) for a, b in pairs)
        user = f"Проверь {len(pairs)} пар:\n{lines}"
        text, usage = _chat(self.api_key, self.model, QA_SYS, user, max_tokens=12000)
        approved, rejected = parse_qa_json(text)
        if not approved and not rejected:
            # fallback: keep pairs if QA returned garbage
            return pairs, [{"error": "QA_EMPTY", "count": len(pairs)}], usage
        return approved, rejected, usage


def deterministic_filter(pairs: list[tuple[str, str]]) -> tuple[list[tuple[str, str]], list[str]]:
    """Быстрая локальная проверка без API."""
    ok: list[tuple[str, str]] = []
    issues: list[str] = []
    file_ext = re.compile(
        r"(?<![@/])\b[\w.-]+\.(?:py|ts|tsx|js|jsx|mjs|json|toml|yaml|yml|md|txt|sql|rs|go|sh)\b"
    )
    mixed = re.compile(r"[а-яёА-ЯЁ][a-zA-Z]|[a-zA-Z][а-яёА-ЯЁ]")
    translated = re.compile(
        r"\b(make|check|open|run|create|delete|update|send|build|deploy)\b", re.I
    )

    for a, b in pairs:
        if a == b:
            ok.append((a, b))
            continue
        if mixed.search(b):
            issues.append(f"mixed alphabet OUT: {b[:60]}")
            continue
        for m in file_ext.finditer(b):
            if b[m.start() - 1 : m.start()] != "@":
                issues.append(f"file without @: {m.group()} in {b[:60]}")
                break
        else:
            for m in translated.finditer(b):
                if m.group().lower() not in a.lower():
                    issues.append(f"translated verb: {m.group()} in {b[:60]}")
                    break
            else:
                ok.append((a, b))
    return ok, issues
