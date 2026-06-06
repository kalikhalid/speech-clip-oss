#!/usr/bin/env python3
"""Замер exact-match нормализатора по eval.jsonl, разбивка по категориям."""
import json, subprocess, sys, os

MODEL = os.path.expanduser("~/Downloads/gemma3_270m_norm_v3_mlx")
EVAL = os.path.join(os.path.dirname(__file__), "..", "data", "eval.jsonl")
SYSTEM = ("Ты нормализуешь русскую голосовую диктовку разработчика. Английские "
          "технические термины, имена файлов, бренды и аббревиатуры, записанные "
          "русскими буквами по звучанию, замени на правильное написание "
          "(коммит -> commit, карго томл -> @Cargo.toml, зум -> Zoom). Имена файлов "
          "пиши через собачку @. Русские слова не переводи и не меняй. Меняй только "
          "термины, всё остальное оставляй как есть.")

def cat(a, b):
    if a == b: return "nochange"
    if len(a.split()) >= 15: return "long"
    return "short"

def infer(text):
    prompt = (f"<start_of_turn>user\n{SYSTEM}\n\n{text}<end_of_turn>\n"
              f"<start_of_turn>model\n")
    out = subprocess.run(
        ["/opt/homebrew/bin/mlx_lm.generate", "--model", MODEL,
         "--prompt", prompt, "--temp", "0", "--max-tokens", "400"],
        capture_output=True, text=True)
    txt = out.stdout
    # mlx печатает ответ между ========== маркерами
    if "==========" in txt:
        parts = txt.split("==========")
        if len(parts) >= 2:
            return parts[1].strip()
    return txt.strip()

rows = [json.loads(l) for l in open(EVAL, encoding="utf-8") if l.strip()]
stats = {}; fails = []
for i, o in enumerate(rows, 1):
    pred = infer(o["in"])
    c = cat(o["in"], o["out"])
    ok = pred.strip() == o["out"].strip()
    stats.setdefault(c, [0, 0])
    stats[c][0] += ok; stats[c][1] += 1
    if not ok and len(fails) < 30:
        fails.append((c, o["in"], o["out"], pred))
    print(f"\r{i}/{len(rows)}", end="", flush=True)

print("\n\n=== EXACT-MATCH по категориям ===")
tot_ok = tot = 0
for c, (ok, n) in sorted(stats.items()):
    print(f"  {c:10s}: {ok}/{n} = {100*ok/n:.0f}%")
    tot_ok += ok; tot += n
print(f"  {'ИТОГО':10s}: {tot_ok}/{tot} = {100*tot_ok/tot:.0f}%")

print("\n=== ПРИМЕРЫ ОШИБОК (до 30) ===")
for c, i, o, p in fails:
    print(f"[{c}]\n  IN  : {i}\n  WANT: {o}\n  GOT : {p}\n")
