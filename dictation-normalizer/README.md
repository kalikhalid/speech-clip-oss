# Dictation Normalizer

Мини-проект: маленькая модель, которая чинит русскую голосовую диктовку —
возвращает английским тех-терминам правильное написание, не трогая русское.

Пример: `сделай коммит и запуш в мастер` → `сделай commit и push в master`

## Зачем
Parakeet (ASR в основном приложении) пишет английские слова «как слышит»
русскими буквами. Эта модель — пост-обработка поверх его вывода.

## Архитектура: два слоя (правила + LLM)

Нормализация в приложении — **двухступенчатая** (см. `src-tauri/src/commands.rs::finalize_text`):

```
сырой ASR
  └─▶ spoken_normalization::normalize_text   (детерминированные правила, Rust)
        └─▶ LLM-нормализатор (Qwen 0.8B)      (только сложный хвост)
              └─▶ spoken_normalization снова  (safety pass)
                    └─▶ guard_model_output    (откат, если модель «перевела» русский)
                          └─▶ apply_dictionary + strip_filler
```

**Разделение труда:**
- **Правила** (`src-tauri/src/spoken_normalization.rs`) берут предсказуемое: известные термины
  (`докер комполз → docker compose`) и имена файлов по шаблону `<корень> точка <ext> → @main.rs`.
  100% точность, защита от unknown-файлов (`Вася точка мди` остаётся как есть).
- **LLM** дочищает только то, что правила не взяли: незнакомые/искажённые термины, склейки и
  **split-слова** (`мы им → main`), контекстную неоднозначность. Русские слова не трогает.

**Важно для обучения — train/serve consistency:** модель в проде видит уже пост-правило текст,
поэтому и обучать её надо на `normalize_text(сырой ASR) → gold`, иначе skew. Учим минимальным
правкам + ~30% passthrough-пар (вход = выход), чтобы не переусердствовала.

## Над чем сейчас работаем
1. **Метрики** вместо голого exact-match: Term-F1 / Filename-F1 / Guard + страты сложности.
2. **Детерминированный слой** в Rust (готов первый инкремент `spoken_normalization.rs`).
3. **Синтетика**: forward-distortion движок (чистый OUT → реалистичный Parakeet-мусор) для 50k+ пар.
4. **Дистилляция + обучение Qwen 0.8B** на пост-правило входе.
5. **Маховик данных** из приложения (реальные правки пользователя → train).

## Термины
- **IN**  — то, что выдал ASR (термины кириллицей по звучанию).
- **OUT** — желаемый текст (термины записаны правильно, русское без изменений).
- Модель учится IN → OUT.

## Где лежит хорошая инфа
- `RAW_PARAKEET_ANALYSIS.md` — **главный документ**: анализ 290 реальных записей, паттерны
  искажений Parakeet, что чинится правилами, а что только LLM.
- `HOW_FINETUNING_WORKS.md` — как устроено дообучение Qwen (LoRA) для не-ML-человека.
- `INTEGRATE.md` — как взять чекпойнт из Colab и поставить в приложение (GGUF-пайплайн).
- `data/parakeet-distortions.tsv` — реальные искажения ASR (IN/OUT/NOTE), golden-сид.
- `data/llm-hard-cases.tsv` — **копилка кейсов «только для LLM»**: split/merge слов
  (`мы им → main`), контекстная неоднозначность, garble мимо банка терминов. Правила это
  не ловят. Идёт целиком в train с повышенным весом и в hard-band эвала. Сюда же падают
  трудные примеры из реального использования.

## Файлы (скрипты/промпты)
- `01-generate-dataset.md` — промпт для ИИ, генерирует пары IN/OUT (таблица → CSV).
- `02-check-dataset.md` — промпт-проверяльщик: ловит плохие пары перед обучением.
- `scripts/pipeline_openai.py` — OpenAI promo pipeline: генератор + QA sub-агенты.
- `scripts/openai_agents.py` — логика sub-агентов (GeneratorAgent, QAAgent).
- `scripts/backup_dataset.py` — архив текущего датасета перед перегенерацией.
- `scripts/term_bank.py` — банк терминов + имён файлов (канонические написания).
- `scripts/eval_model.py` — замер качества на eval.jsonl.

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
