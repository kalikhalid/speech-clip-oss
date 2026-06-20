#!/usr/bin/env python3
"""Generate profile-specific spoken-normalization rule candidates with OpenAI.

The API key is read only from OPENAI_API_KEY. Do not pass it as an argument.

Examples:
  export OPENAI_API_KEY=...
  python3 dictation-normalizer/scripts/generate_profile_rules.py --all --token-budget 200000
  python3 dictation-normalizer/scripts/generate_profile_rules.py --profile developer --batches 4
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
import time
import urllib.request
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parents[2]
PROFILES_PATH = REPO / "src-tauri" / "resources" / "spoken_normalization_profiles.json"
DEFAULT_OUT_DIR = REPO / "dictation-normalizer" / "data" / "generated_profile_rules"

API = "https://api.openai.com/v1/chat/completions"
DEFAULT_MODEL = os.environ.get("OPENAI_PROFILE_MODEL", "gpt-5.4")

PROFILE_FOCUS: dict[str, str] = {
    "general": (
        "Generate conservative general dictation rules. Avoid developer, legal, and medical "
        "jargon. Focus on safe formatting words, common ASR variants, and no-change eval cases."
    ),
    "developer": (
        "Generate AI-coding prompt rules: agent instructions, constraints, tests, build commands, "
        "frontend/backend/devops terms, and dangerous negation eval cases. Avoid overfitting to filenames."
    ),
    "writing": (
        "Generate writing/editing dictation rules: headings, lists, summaries, tone instructions, "
        "drafting markers, and long-form text structure. Avoid code terms unless explicitly common."
    ),
    "support_sales": (
        "Generate support/sales rules: CRM notes, follow-ups, call summaries, leads, pipeline, "
        "SLA, invoice, renewal, ticket updates, and customer communication."
    ),
    "legal_lite": (
        "Generate conservative legal-note rules: section/clause/comment/redline markers and eval "
        "cases that preserve exact wording, dates, numbers, defined terms, and negations."
    ),
    "medical_lite": (
        "Generate conservative clinical-note rules: note sections, allergies, dosage, symptoms, "
        "plan, recommendations, and eval cases that preserve medication names, numbers, and negations."
    ),
}

SYSTEM_PROMPT = """You generate candidate rules for local spoken dictation normalization.

Return ONLY valid JSON. No markdown.

The product receives Russian ASR text. Users may speak English/domain terms phonetically in Russian.
Rules are deterministic candidates, so they must be high precision.

Output schema:
{
  "profile_id": "string",
  "rules": {
    "terms": [{"from": ["token", "..."], "to": "canonical"}],
    "protected_phrases": ["phrase that should stay unchanged"],
    "avoid_replacements": ["bad replacement description"]
  },
  "eval_cases": [
    {"in": "raw ASR-like text", "out": "expected text", "bucket": "short_name"}
  ]
}

Rules:
- `from` is an array of lower-case spoken tokens, already split by spaces.
- `to` is the exact canonical output.
- Do not add ambiguous replacements that would corrupt normal speech.
- For general/writing/legal/medical profiles, prefer protected phrases over aggressive term replacements.
- Include many no-change and negation cases in eval_cases.
- Keep Russian words Russian unless the domain term should be canonicalized.
- Never include secrets, real people data, or private customer data.
"""


def load_profiles() -> dict[str, dict[str, Any]]:
    payload = json.loads(PROFILES_PATH.read_text(encoding="utf-8"))
    return {p["id"]: p for p in payload["profiles"]}


def chat(
    api_key: str,
    model: str,
    user_prompt: str,
    max_completion_tokens: int,
) -> tuple[dict[str, Any], dict[str, Any]]:
    body = json.dumps(
        {
            "model": model,
            "messages": [
                {"role": "system", "content": SYSTEM_PROMPT},
                {"role": "user", "content": user_prompt},
            ],
            "max_completion_tokens": max_completion_tokens,
            "reasoning_effort": "low",
        }
    ).encode()
    req = urllib.request.Request(
        API,
        data=body,
        headers={
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
        },
    )
    with urllib.request.urlopen(req, timeout=180) as resp:
        data = json.load(resp)
    text = data["choices"][0]["message"]["content"].strip()
    if text.startswith("```"):
        text = re.sub(r"^```(?:json)?\s*|\s*```$", "", text, flags=re.S).strip()
    return json.loads(text), data.get("usage", {})


def build_user_prompt(profile: dict[str, Any], batch: int, per_batch: int) -> str:
    profile_id = profile["id"]
    focus = PROFILE_FOCUS.get(profile_id, "Generate conservative high-precision rules.")
    return json.dumps(
        {
            "task": "Generate profile-specific spoken-normalization candidates.",
            "profile": profile,
            "focus": focus,
            "batch": batch,
            "target_counts": {
                "terms": per_batch,
                "protected_phrases": max(10, per_batch // 3),
                "avoid_replacements": max(8, per_batch // 4),
                "eval_cases": per_batch,
            },
            "diversity_instruction": (
                "Use varied ASR-like Russian phonetic spellings, inflections, and long prompt "
                "phrases. Avoid duplicating obvious examples from the profile description."
            ),
        },
        ensure_ascii=False,
    )


def normalize_token(token: str) -> str:
    token = token.strip().lower().replace("ё", "е")
    token = re.sub(r"\s+", " ", token)
    return token


def normalize_rule(rule: dict[str, Any]) -> dict[str, Any] | None:
    raw_from = rule.get("from")
    to = str(rule.get("to", "")).strip()
    if isinstance(raw_from, str):
        from_tokens = raw_from.split()
    elif isinstance(raw_from, list):
        from_tokens = [str(t) for t in raw_from]
    else:
        return None
    from_tokens = [normalize_token(t) for t in from_tokens]
    from_tokens = [t for t in from_tokens if t]
    if not from_tokens or not to:
        return None
    return {"from": from_tokens, "to": to}


def merge_payloads(profile_id: str, model: str, payloads: list[dict[str, Any]]) -> dict[str, Any]:
    terms: list[dict[str, Any]] = []
    protected: list[str] = []
    avoid: list[str] = []
    eval_cases: list[dict[str, str]] = []
    seen_terms: set[tuple[tuple[str, ...], str]] = set()
    seen_text: set[str] = set()

    for payload in payloads:
        rules = payload.get("rules", {})
        for raw_rule in rules.get("terms", []):
            if not isinstance(raw_rule, dict):
                continue
            rule = normalize_rule(raw_rule)
            if not rule:
                continue
            key = (tuple(rule["from"]), rule["to"])
            if key not in seen_terms:
                seen_terms.add(key)
                terms.append(rule)

        for item in rules.get("protected_phrases", []):
            phrase = str(item).strip()
            key = f"p:{phrase.lower()}"
            if phrase and key not in seen_text:
                seen_text.add(key)
                protected.append(phrase)

        for item in rules.get("avoid_replacements", []):
            phrase = str(item).strip()
            key = f"a:{phrase.lower()}"
            if phrase and key not in seen_text:
                seen_text.add(key)
                avoid.append(phrase)

        for item in payload.get("eval_cases", []):
            if not isinstance(item, dict):
                continue
            raw_in = str(item.get("in", "")).strip()
            out = str(item.get("out", "")).strip()
            bucket = str(item.get("bucket", "general")).strip() or "general"
            key = f"e:{raw_in.lower()}->{out.lower()}"
            if raw_in and out and key not in seen_text:
                seen_text.add(key)
                eval_cases.append({"in": raw_in, "out": out, "bucket": bucket})

    return {
        "profile_id": profile_id,
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "model": model,
        "rules": {
            "terms": terms,
            "protected_phrases": protected,
            "avoid_replacements": avoid,
        },
        "eval_cases": eval_cases,
    }


def generate_profile(
    profile: dict[str, Any],
    api_key: str,
    model: str,
    batches: int,
    per_batch: int,
    max_completion_tokens: int,
    token_budget: int | None,
    spent_tokens: int,
) -> tuple[dict[str, Any], int]:
    payloads = []
    total_tokens = 0
    for batch in range(1, batches + 1):
        if token_budget is not None and spent_tokens + total_tokens >= token_budget:
            print("  token budget reached; stopping this profile")
            break
        prompt = build_user_prompt(profile, batch, per_batch)
        payload, usage = chat(api_key, model, prompt, max_completion_tokens)
        payloads.append(payload)
        total_tokens += int(usage.get("total_tokens", 0) or 0)
        print(
            f"  batch {batch}/{batches}: "
            f"{len(payload.get('rules', {}).get('terms', []))} terms, "
            f"{len(payload.get('eval_cases', []))} eval cases"
        )
        time.sleep(0.25)
    return merge_payloads(profile["id"], model, payloads), total_tokens


def main() -> None:
    ap = argparse.ArgumentParser(description="Generate profile-specific normalization candidates")
    group = ap.add_mutually_exclusive_group(required=True)
    group.add_argument("--profile", help="Profile id from spoken_normalization_profiles.json")
    group.add_argument("--all", action="store_true", help="Generate all profiles")
    ap.add_argument("--batches", type=int, default=2)
    ap.add_argument("--per-batch", type=int, default=30)
    ap.add_argument("--max-completion-tokens", type=int, default=7000)
    ap.add_argument(
        "--token-budget",
        type=int,
        default=200000,
        help="Stop before spending more total tokens. Use 0 to disable.",
    )
    ap.add_argument("--model", default=DEFAULT_MODEL)
    ap.add_argument("--out-dir", default=str(DEFAULT_OUT_DIR))
    args = ap.parse_args()

    api_key = os.environ.get("OPENAI_API_KEY")
    if not api_key:
        print("OPENAI_API_KEY is required in the environment", file=sys.stderr)
        sys.exit(2)

    profiles = load_profiles()
    selected = list(profiles) if args.all else [args.profile]
    unknown = [profile_id for profile_id in selected if profile_id not in profiles]
    if unknown:
        print(f"Unknown profile(s): {', '.join(unknown)}", file=sys.stderr)
        sys.exit(2)

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    token_budget = None if args.token_budget == 0 else args.token_budget
    grand_total = 0
    for profile_id in selected:
        if token_budget is not None and grand_total >= token_budget:
            print(f"token budget reached before {profile_id}; stopping")
            break
        print(f"=== {profile_id} ===")
        result, tokens = generate_profile(
            profiles[profile_id],
            api_key=api_key,
            model=args.model,
            batches=args.batches,
            per_batch=args.per_batch,
            max_completion_tokens=args.max_completion_tokens,
            token_budget=token_budget,
            spent_tokens=grand_total,
        )
        grand_total += tokens
        out_path = out_dir / f"{profile_id}.json"
        out_path.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(
            f"  wrote {out_path} | "
            f"{len(result['rules']['terms'])} terms, "
            f"{len(result['eval_cases'])} eval cases, "
            f"tokens {tokens}"
        )

    print(f"done | total tokens {grand_total}")


if __name__ == "__main__":
    main()
