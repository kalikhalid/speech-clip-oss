# Dictation Normalizer

Мини-проект: маленькая модель, которая чинит русскую голосовую диктовку —
возвращает английским тех-терминам правильное написание, не трогая русское.

Пример: `сделай коммит и запуш в мастер` → `сделай commit и push в master`

## Зачем
Parakeet (ASR в основном приложении) пишет английские слова «как слышит»
русскими буквами. Эта модель — пост-обработка поверх его вывода.

## Термины
- **IN**  — то, что выдал ASR (термины кириллицей по звучанию).
- **OUT** — желаемый текст (термины записаны правильно, русское без изменений).
- Модель учится IN → OUT.

## Файлы
- `01-generate-dataset.md` — промпт для ИИ, генерирует пары IN/OUT (таблица → CSV).
- `02-check-dataset.md` — промпт-проверяльщик: ловит плохие пары перед обучением.
- `scripts/pipeline_openai.py` — OpenAI promo pipeline: генератор + QA sub-агенты.
- `scripts/openai_agents.py` — логика sub-агентов (GeneratorAgent, QAAgent).
- `scripts/backup_dataset.py` — архив текущего датасета перед перегенерацией.

## OpenAI pipeline (data-sharing promo)

```bash
export OPENAI_API_KEY=sk-...   # не коммитить в git
python3 dictation-normalizer/scripts/backup_dataset.py
python3 dictation-normalizer/scripts/pipeline_openai.py --batches 80
```

Пайплайн:
1. **GeneratorAgent** — gpt-5-mini, батчи по ~40 пар (dev post-ASR, Parakeet-грязь, @-файлы).
2. **QAAgent** — проверяет каждый батч, отклоняет плохие пары.
3. Локальный `deterministic_filter` — файлы без @, mixed alphabet.
4. Golden-пары из `parakeet-distortions.tsv` + `raw/agent_parakeet_gaps.tsv`.
5. `clean_dataset.py` → `dataset.v5.tsv` + `train.jsonl` + `eval.jsonl`.

Архив старого датасета: `data/archive/pre-openai-*`.

## План
1. Генерируешь датасет промптом №1 (Notion AI → экспорт в CSV). Цель: 3–5к пар.
2. Проверяешь его промптом №2, чистишь проблемные пары.
3. Откладываешь ~200 пар в held-out (проверка точности после обучения).
4. LoRA fine-tune (Unsloth):
   - `gemma3_270m_dictation.ipynb` — Gemma 3 270M
   - `qwen35_08b_dictation.ipynb` — **Qwen3.5-0.8B** (text-only, рекомендуется)
5. Конвертируешь в MLX и запускаешь локально на Mac.

## Состав датасета (важно)
- ~55% — команды с терминами
- ~25% — чистый русский (IN == OUT) — защита от «исправления всего»
- ~15% — длинные склеенные цепочки
- ~5%  — ловушки (русское слово вплотную к термину)
