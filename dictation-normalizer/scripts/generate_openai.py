#!/usr/bin/env python3
"""Генерация датасета через OpenAI gpt-5-mini (бесплатные data-sharing токены).
Стратегия: узкий банк терминов, каждый встречается МНОГО раз (recall > широта).
Реалистичные искажения Parakeet + пунктуация + @-файлы + no-change + ловушки + длинные.

Запуск:
  export OPENAI_API_KEY=...
  python3 scripts/generate_openai.py --batches 160 --out data/dataset.v3.tsv
"""
import os, sys, json, time, random, argparse, urllib.request, collections, re
sys.path.insert(0, os.path.dirname(__file__))
from term_bank import TERMS, FILE_TERMS

def load_real_distortions():
    """Вытаскивает реальные искажения ASR из истории обоих приложений."""
    paths = [
        os.path.expanduser("~/Library/Application Support/dev.speechclip.oss/history.json"),
        os.path.expanduser("~/Library/Application Support/com.speechclip.app/history.json"),
    ]
    samples = []
    for p in paths:
        try:
            d = json.load(open(p, encoding="utf-8"))
            entries = d.get("entries", d) if isinstance(d, dict) else d
            for e in entries:
                raw = e.get("raw_text", "").strip()
                # берём только фразы где есть кириллица И похоже на термин
                if raw and re.search(r'[а-яё]{3,}', raw, re.I) and len(raw) > 8:
                    samples.append(raw)
        except Exception:
            pass
    random.shuffle(samples)
    return samples[:80]  # не более 80 примеров чтобы не раздувать промпт

MODEL = "gpt-5-mini"
API = "https://api.openai.com/v1/chat/completions"

SYS = """Ты генерируешь обучающие пары для модели-нормализатора русской голосовой диктовки разработчика.
ASR (Parakeet) слышит английские термины и пишет их русскими буквами «как звучит». Модель учится возвращать правильное написание.

ФОРМАТ ОТВЕТА: JSONL — каждая пара на ОТДЕЛЬНОЙ строке как {"in": "...", "out": "..."}. Без markdown, без массивов, без обёрток — просто строки JSON одна под другой.

ПРАВИЛА (железно):
1. in = фраза, где термины записаны РУССКИМИ буквами по звучанию (коммит, докер, мастер, джейсон, реакт). Русские слова обычные.
2. out = та же фраза, термины записаны правильно (латиницей/как код), русские слова и порядок слов БЕЗ ИЗМЕНЕНИЙ.
3. НИКОГДА не переводи русские слова на английский: «сделай коммит» -> «сделай commit», НЕ «make commit».
4. ИМЕНА ФАЙЛОВ в out ВСЕГДА через @ без code-форматирования: ридми точка эм дэ -> @README.md, карго томл -> @Cargo.toml.
   Если in УЖЕ содержит латинское имя файла (clean_dataset.py, main.rs, config.yaml) — в out добавь @: @clean_dataset.py.
   Паттерны «который называется …», «скрипт … .py», «файл … .json» — включай часто.
5. Пунктуация и заглавные как у реального ASR: точки, запятые, заглавная в начале. В in и out пунктуация СОВПАДАЕТ 1:1, меняется только написание терминов.
6. Воспроизводи РЕАЛЬНЫЕ искажения ASR: глотает/меняет буквы (ридми->ритми, энв->энф, гит->гид, слак->слаг), склеивает/рвёт слова (запушфорс, докер комполз, лискрипт от «ли скрипт», отработалли), путает похожее. ~35% строк делай «грязными».
7. Один термин озвучивай ПО-РАЗНОМУ в разных строках (commit -> коммит/комит, remote -> ремот/римоут).
8. АНТИ-ГАЛЛЮЦИНАЦИЯ: если в in нет термина — out полностью равен in.
9. ГЛАГОЛЫ-РУСИЦИЗМЫ — ОСТАВЛЯТЬ КИРИЛЛИЦЕЙ. Русифицированные глаголы типа «запушил», «закоммитил», «задеплоил», «смёрджил», «зафетчил» — НЕ ТРОГАТЬ, оставлять как есть. Это глаголы в русской грамматике, они уже русские. Только чистые существительные-команды конвертируются: «сделай пуш» -> «сделай push», «сделай коммит» -> «сделай commit».
10. ВСЕ СУЩЕСТВИТЕЛЬНЫЕ-ТЕРМИНЫ — В ЛАТИНИЦУ. Термины как существительные конвертируются: докер->docker, коммит->commit, мастер->master, контейнер->container, ремот->remote, реакт->React, деплой->deploy (когда это существительное: «делай деплой» -> «делай deploy»).
11. НИКАКОГО смешения алфавитов в одном слове. Запрещено «testы», «diffы», «buildы». Если термин во множественном/падежной — пиши ПОЛНОСТЬЮ по-английски (тесты->tests, диффы->diffs) или не используй такую форму. НИКОГДА латинский корень + русское окончание.
12. Русские слова (глаголы не от терминов, предлоги, союзы, обычные существительные) остаются кириллицей без изменений."""

def user_prompt(terms, files, real_samples):
    samples_block = ""
    if real_samples:
        picked = random.sample(real_samples, min(6, len(real_samples)))
        samples_block = "\n\nРЕАЛЬНЫЕ примеры из истории ASR пользователя (вот КАК реально коверкает слова его микрофон+модель — воспроизводи ТАКОЙ стиль искажений в in):\n"
        samples_block += "\n".join(f"  «{s}»" for s in picked)
    return f"""Сгенерируй 40 пар, используя ПРЕИМУЩЕСТВЕННО эти термины (озвучивай их по-русски в in):
ТЕРМИНЫ: {', '.join(terms)}
ФАЙЛЫ (в out строго через @): {', '.join(files)}{samples_block}

Состав 40 пар:
- 22 пары: команды/реплики с 1-3 терминами из списка выше (фронтенд, бэкенд, devops, инструменты — разные сферы dev).
- 8 пар: ЧИСТАЯ русская речь без терминов, in == out (рабочая болтовня, вопросы, бытовое).
- 6 пар: ДЛИННЫЕ склейки 3-6 предложений подряд с терминами и пунктуацией.
- 4 пары: ловушки — русское слово вплотную к термину (запушил в ремот -> запушил в remote).
Разнообразь глаголы, длину, тон. Верни ТОЛЬКО JSONL."""

def call(terms, files, key, real_samples=None):
    body = json.dumps({
        "model": MODEL,
        "messages": [{"role": "system", "content": SYS},
                     {"role": "user", "content": user_prompt(terms, files, real_samples or [])}],
        "max_completion_tokens": 9000,
        "reasoning_effort": "minimal",
    }).encode()
    req = urllib.request.Request(API, data=body, headers={
        "Authorization": f"Bearer {key}", "Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=120) as r:
        d = json.load(r)
    return d["choices"][0]["message"]["content"], d.get("usage", {})

def parse(txt):
    out = []
    for line in txt.splitlines():
        line = line.strip().strip(",")
        if not (line.startswith("{") and line.endswith("}")):
            continue
        try:
            it = json.loads(line)
        except Exception:
            continue
        if isinstance(it, dict) and "in" in it and "out" in it:
            a, b = str(it["in"]).strip(), str(it["out"]).strip()
            if a and b:
                out.append((a, b))
    return out

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--batches", type=int, default=160)
    ap.add_argument("--out", default="data/dataset.v3.tsv")
    args = ap.parse_args()
    key = os.environ["OPENAI_API_KEY"]

    random.seed(7)
    real_samples = load_real_distortions()
    print(f"Загружено реальных искажений из истории: {len(real_samples)}")
    seen = set(); pairs = []; tok = 0
    with open(args.out, "w", encoding="utf-8") as f:
        f.write("IN\tOUT\n")
        for i in range(1, args.batches + 1):
            terms = random.sample(TERMS, 12)
            files = random.sample(FILE_TERMS, 4)
            try:
                txt, usage = call(terms, files, key, real_samples)
            except Exception as e:
                print(f"\n[batch {i}] ошибка: {e}; пауза 5с"); time.sleep(5); continue
            tok += usage.get("total_tokens", 0)
            new = 0
            for a, b in parse(txt):
                k = a.lower()
                if k in seen:
                    continue
                seen.add(k); pairs.append((a, b))
                f.write(a.replace("\t", " ") + "\t" + b.replace("\t", " ") + "\n")
                new += 1
            f.flush()
            print(f"\rбатч {i}/{args.batches} | +{new} | всего {len(pairs)} | токенов {tok}", end="", flush=True)
            time.sleep(0.3)
    print(f"\n\nГотово: {len(pairs)} пар -> {args.out}")
    nc = sum(1 for a, b in pairs if a == b)
    print(f"no-change: {nc} ({100*nc/max(1,len(pairs)):.0f}%) | токенов всего: {tok}")

if __name__ == "__main__":
    main()
