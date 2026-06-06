#!/usr/bin/env python3
"""
clean_dataset.py — deterministic cleanup of dataset.v3.tsv → dataset.v4.tsv
plus regeneration of clean train/eval splits.

Rules:
  - Noun-terms → Latin canonical (коммит→commit, etc.)
  - Russified verbs → stay Cyrillic (запушил, закоммитил, etc.)
  - File names → @-prefixed Latin
  - No mixed-alphabet within a word
  - Plurals/cases fully Latin (tests, containers, etc.)
  - Delete rows that are corrupted concatenation artifacts
  - Dedup exact and punctuation-only duplicates
"""

import csv
import json
import re
import shutil
import sys
import unicodedata
from collections import defaultdict
from pathlib import Path

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------
REPO = Path(__file__).resolve().parent.parent
DATA = REPO / "dictation-normalizer" / "data"
V3 = DATA / "dataset.v3.tsv"
V4 = DATA / "dataset.v4.tsv"
TRAIN_OUT = DATA / "train.jsonl"
EVAL_OUT  = DATA / "eval.jsonl"
TRAIN_BAK = DATA / "train.jsonl.v3.bak"
EVAL_BAK  = DATA / "eval.jsonl.v3.bak"

# ---------------------------------------------------------------------------
# Term bank: Cyrillic variants → canonical Latin
# ---------------------------------------------------------------------------
# Each entry: (cyrillic_pattern_lower, canonical_out)
# Patterns are matched as full tokens (word-boundary aware).
# More specific / longer entries should appear first.

TERM_MAP_RAW = [
    # git
    ("коммит",          "commit"),
    ("комит",           "commit"),
    ("коммиты",         "commits"),
    ("комиты",          "commits"),
    ("коммита",         "commit"),   # genitive
    ("коммитов",        "commits"),
    ("комита",          "commit"),
    ("пуш",             "push"),
    ("пушфорс",         "force push"),
    ("запушфорс",       "force push"),  # erroneous concatenation but clean
    ("пулл",            "pull"),
    ("пул",             "pull"),
    ("мёрдж",           "merge"),
    ("мёрджа",          "merge"),
    ("мердж",           "merge"),
    ("мёрджи",          "merges"),
    ("ребейс",          "rebase"),
    ("ребейса",         "rebase"),
    ("чекаут",          "checkout"),
    ("чек-аут",         "checkout"),
    ("бранч",           "branch"),
    ("бранчи",          "branches"),
    ("бранча",          "branch"),
    ("ветки",           None),  # keep (Russian)
    ("ветку",           None),
    ("клон",            "clone"),
    ("дифф",            "diff"),
    ("диф",             "diff"),
    ("диффы",           "diffs"),
    ("стэш",            "stash"),
    ("стеш",            "stash"),
    ("фетч",            "fetch"),
    ("фетча",           "fetch"),
    ("форс пуш",        "force push"),
    ("черри-пик",       "cherry-pick"),
    ("мастер",          "master"),
    ("мейн",            "main"),
    ("ориджин",         "origin"),
    ("ремот",           "remote"),
    ("римоут",          "remote"),
    ("ремоут",          "remote"),
    ("пулл реквест",    "pull request"),
    ("пул реквест",     "pull request"),
    ("пуллреквест",     "pull request"),
    ("пр",              "PR"),   # careful: only standalone
    # docker / devops
    ("докер",           "docker"),
    ("докера",          "docker"),
    ("докере",          "docker"),
    ("докер компоуз",   "docker compose"),
    ("докер-компоуз",   "docker compose"),
    ("докомпоуз",       "docker compose"),
    ("контейнер",       "container"),
    ("контейнеры",      "containers"),
    ("контейнера",      "container"),
    ("контейнеров",     "containers"),
    ("контейнере",      "container"),
    ("имидж",           "image"),
    ("имейдж",          "image"),
    ("имиджи",          "images"),
    ("имейджи",         "images"),
    ("деплой",          "deploy"),
    ("деплоя",          "deploy"),
    ("деплое",          "deploy"),
    ("деплои",          "deploys"),
    ("деплоев",         "deploys"),
    ("кубернетис",      "kubernetes"),
    ("кубернетес",      "kubernetes"),
    ("под",             None),    # ambiguous (Russian preposition too) — skip
    ("поды",            "pods"),
    ("пода",            "pod"),
    ("подов",           "pods"),
    ("нгинкс",          "nginx"),
    ("нджинкс",         "nginx"),
    ("прод",            "prod"),
    ("прода",           "prod"),
    ("проде",           "prod"),
    ("продов",          "prod"),
    ("стейджинг",       "staging"),
    ("стэйджинг",       "staging"),
    ("стейдж",          "staging"),
    ("роллбэк",         "rollback"),
    ("откат",           None),   # keep Russian
    ("терраформ",       "Terraform"),
    ("ансибл",          "Ansible"),
    ("сиайсиди",        "CI/CD"),
    ("си-ай-си-ди",     "CI/CD"),
    # backend / api
    ("бэкенд",          "backend"),
    ("бекенд",          "backend"),
    ("бэк",             "backend"),   # standalone only — may be short
    ("фронтенд",        "frontend"),
    ("фронт",           "frontend"),  # standalone
    ("апи",             "API"),
    ("эндпоинт",        "endpoint"),
    ("эндпоинты",       "endpoints"),
    ("эндпоинта",       "endpoint"),
    ("эндпоинтов",      "endpoints"),
    ("реквест",         "request"),
    ("реквесты",        "requests"),
    ("реквеста",        "request"),
    ("респонс",         "response"),
    ("джейсон",         "JSON"),
    ("мигра",           None),   # prefix ambiguous
    ("миграция",        "migration"),
    ("миграции",        "migrations"),
    ("миграцию",        "migration"),
    ("кэш",             "cache"),
    ("кеш",             "cache"),
    ("кэша",            "cache"),
    ("кэше",            "cache"),
    ("кэширование",     "caching"),
    ("токен",           "token"),
    ("токены",          "tokens"),
    ("токена",          "token"),
    ("токенов",         "tokens"),
    ("вебхук",          "webhook"),
    ("хук",             "hook"),
    ("хуки",            "hooks"),
    ("хука",            "hook"),
    ("хуков",           "hooks"),
    ("крон",            "cron"),
    ("очередь",         None),   # Russian
    ("кафка",           "Kafka"),
    ("редис",           "Redis"),
    ("постгрес",        "Postgres"),
    ("монгодб",         "MongoDB"),
    ("мёрдж конфликт",  "merge conflict"),
    ("мерж конфликт",   "merge conflict"),
    ("конфликт мёрджа", "merge conflict"),
    ("конфликт мерджа", "merge conflict"),
    # languages / runtimes
    ("пайтон",          "Python"),
    ("питон",           "Python"),
    ("джаваскрипт",     "JavaScript"),
    ("тайпскрипт",      "TypeScript"),
    ("тайпскрипта",     "TypeScript"),
    ("раст",            "Rust"),
    ("карго",           "cargo"),
    ("ноде",            "Node"),
    ("нпм",             "npm"),
    ("ярн",             "yarn"),
    ("пип",             "pip"),
    ("джава",           "Java"),
    ("котлин",          "Kotlin"),
    ("свифт",           "Swift"),
    # frontend
    ("реакт",           "React"),
    ("реакта",          "React"),
    ("компонент",       "component"),
    ("компоненты",      "components"),
    ("компонента",      "component"),
    ("компонентов",     "components"),
    ("компоненте",      "component"),
    ("пропсы",          "props"),
    ("пропс",           "props"),
    ("стейт",           "state"),
    ("тейлвинд",        "Tailwind"),
    ("вайт",            "Vite"),
    ("вебпак",          "Webpack"),
    ("бандл",           "bundle"),
    ("линт",            "lint"),
    ("линтер",          "linter"),
    ("биллд",           "build"),
    ("билд",            "build"),
    ("билды",           "builds"),
    ("билда",           "build"),
    ("билде",           "build"),
    # test / process
    ("тест",            "test"),
    ("тесты",           "tests"),
    ("теста",           "test"),
    ("тестов",          "tests"),
    ("тестировать",     None),  # verb, keep
    ("мок",             "mock"),
    ("моки",            "mocks"),
    ("моков",           "mocks"),
    ("дебаг",           "debug"),
    ("лог",             "log"),
    ("логи",            "logs"),
    ("лога",            "log"),
    ("логов",           "logs"),
    ("логе",            "log"),
    ("стек трейс",      "stack trace"),
    ("стектрейс",       "stack trace"),
    ("код ревью",       "code review"),
    ("кодревью",        "code review"),
    ("ревью",           "review"),
    ("рефактор",        "refactor"),
    ("рефакторинг",     "refactoring"),
    ("релиз",           "release"),
    ("пайплайн",        "pipeline"),
    ("пайплайны",       "pipelines"),
    ("пайплайна",       "pipeline"),
    # tools / services
    ("слак",            "Slack"),
    ("слака",           "Slack"),
    ("слаке",           "Slack"),
    ("ноушн",           "Notion"),
    ("ноушне",          "Notion"),
    ("зум",             "Zoom"),
    ("зума",            "Zoom"),
    ("зуме",            "Zoom"),
    ("фигма",           "Figma"),
    ("фигме",           "Figma"),
    ("джира",           "Jira"),
    ("джире",           "Jira"),
    ("гитхаб",          "GitHub"),
    ("гитлаб",          "GitLab"),
    ("телеграм",        "Telegram"),
    ("вэс код",         "VS Code"),
    ("вес код",         "VS Code"),
    ("вс код",          "VS Code"),
    ("курсор",          None),   # ambiguous (cursor position vs Cursor IDE)
    ("линеар",          "Linear"),
    ("постман",         "Postman"),
    ("эксэль",          "Excel"),
    ("гугл докс",       "Google Docs"),
    ("гугл-докс",       "Google Docs"),
    # abbreviations
    ("хттпс",           "HTTPS"),
    ("хттп",            "HTTP"),
    ("урл",             "URL"),
    ("ипишник",         "IP"),
    ("сдк",             "SDK"),
    ("кли",             "CLI"),
    ("юай",             "UI"),
    ("юикс",            "UX"),
    ("пдф",             "PDF"),
    ("цсв",             "CSV"),
    ("рам",             "RAM"),
    ("цпу",             "CPU"),
    ("мвп",             "MVP"),
    ("куа",             "QA"),
    ("пр",              "PR"),
    # file names (handled separately via @-prefix)
    ("ридми",           "@README.md"),
    ("пакейдж джейсон", "@package.json"),
    ("пакейдж.джейсон", "@package.json"),
    ("карго.томл",      "@Cargo.toml"),
    ("дотэнв",          "@.env"),
    ("тсконфиг",        "@tsconfig.json"),
    ("докерфайл",       "@Dockerfile"),
    ("гитигнор",        "@.gitignore"),
    ("мейкфайл",        "@Makefile"),
    ("сеттингс джейсон","@settings.json"),
]

# Build a sorted-by-length (longest first) lookup list
TERM_MAP: list[tuple[str, str | None]] = sorted(
    [(k, v) for k, v in TERM_MAP_RAW if v is not None],
    key=lambda x: len(x[0]),
    reverse=True
)

# Cyrillic chars pattern
CYR = re.compile(r'[а-яёА-ЯЁ]')
LAT = re.compile(r'[a-zA-Z]')

# ---------------------------------------------------------------------------
# Russified verb recognition — these must NOT be converted
# ---------------------------------------------------------------------------
# Full forms that appear verbatim and should stay Cyrillic
RUSSIFIED_VERB_FORMS = {
    # past tense за- prefix
    "запушил", "запушила", "запушили",
    "закоммитил", "закоммитила", "закоммитили",
    "задеплоил", "задеплоила", "задеплоили",
    "смёрджил", "смёрджила", "смёрджили",
    "смерджил", "смерджила", "смерджили",
    "зафетчил", "зафетчила", "зафетчили",
    "зарефакторил", "зарефакторила", "зарефакторили",
    "зачекаутил", "зачекаутила", "зачекаутили",
    "зарибейснул", "зарибейснула",
    "заребейснул", "заребейснула",
    "зарезолвил", "зарезолвила",
    "задебажил", "задебажила",
    "залинтил", "залинтила",
    "заформатил", "заформатила",
    # non-past / infinitive forms
    "запушить", "закоммитить", "задеплоить",
    "смёрджить", "смерджить", "зафетчить",
    "зарефакторить", "зачекаутить", "зарибейснуть", "заребейснуть",
    # short forms / slang
    "запушилвремот",  # corrupted concatenation — keep as-is? Actually delete
    # other common verbs
    "мёрджить", "мерджить",
    "рефакторить", "рефакторил", "рефакторила", "рефакторили",
    "дебажить", "дебажил", "дебажила",
    "лочить", "дефолтить",
    "ребейснуть", "ребейснул", "ребейснула",
    "ребейснули",
    "смёрджь", "смерджь",
    "запушь",
    "зафетчить", "зафетчь",
    "закоммить",
    "зафиксить", "зафиксил",
    "задебажить",
    # check-out verb forms
    "чекаутнуть", "чекаутнул", "чекаутнула",
    "зачекаутнуть",
    # refactor verb forms
    "рефакторнуть", "рефакторнул", "рефакторнула",
    "зарефакторнуть",
}

# Patterns for russified-verb detection via regex (prefix + latin-root + cyrillic suffix)
# These are the mixed-alphabet class (b): Latin stem + Cyrillic suffix → should be all-Cyrillic
MIXED_VERB_REPAIR: list[tuple[re.Pattern, str]] = [
    # refactorить → рефакторить
    (re.compile(r'\brefactor(ить|ил[аи]?|нуть|нул[аи]?|инг)\b', re.IGNORECASE),
     lambda m: "рефактор" + m.group(1)),
    (re.compile(r'\bmerge(ить|ил[аи]?|нуть|нул[аи]?)\b', re.IGNORECASE),
     lambda m: "мёрдж" + m.group(1)),
    (re.compile(r'\bdebug(ить|ил[аи]?|нуть|нул[аи]?)\b', re.IGNORECASE),
     lambda m: "дебаг" + m.group(1)),
    (re.compile(r'\bfetch(ить|ил[аи]?|нуть|нул[аи]?)\b', re.IGNORECASE),
     lambda m: "фетч" + m.group(1)),
    (re.compile(r'\brebase(ить|ил[аи]?|нуть|нул[аи]?)\b', re.IGNORECASE),
     lambda m: "ребейс" + m.group(1)),
    (re.compile(r'\bcheckout(ить|ил[аи]?|нуть|нул[аи]?)\b', re.IGNORECASE),
     lambda m: "чекаут" + m.group(1)),
    (re.compile(r'\btest(ировать|ировал[аи]?)\b', re.IGNORECASE),
     lambda m: "тестирова" + ("ть" if "ть" in m.group(1) else "л" + m.group(1)[len("ировал"):])),
    (re.compile(r'\btest(ируется)\b', re.IGNORECASE),
     lambda m: "тестируется"),
    (re.compile(r'\bcache(ирование|ировать|ировал[аи]?)\b', re.IGNORECASE),
     lambda m: "кэшир" + m.group(1)[len("ирова"):] if m.group(1).startswith("ирова") else "кэш" + m.group(1)),
]

# Repair for mixed-alphabet class (c): Latin root + Cyrillic plural/case suffix → full Latin
# e.g. podы → pods, testы → tests, containerы → containers
MIXED_NOUN_REPAIR: list[tuple[re.Pattern, str]] = [
    (re.compile(r'\bpod[ыа-я]+\b', re.IGNORECASE),  "pods"),
    (re.compile(r'\btest[ыа-я]+\b', re.IGNORECASE),  "tests"),
    (re.compile(r'\bcontainer[ыа-я]+\b', re.IGNORECASE), "containers"),
    (re.compile(r'\brequest[ыа-я]+\b', re.IGNORECASE),  "requests"),
    (re.compile(r'\bresponse[ыа-я]+\b', re.IGNORECASE), "responses"),
    (re.compile(r'\bcommit[ыа-я]+\b', re.IGNORECASE),   "commits"),
    (re.compile(r'\btoken[ыа-я]+\b', re.IGNORECASE),    "tokens"),
    (re.compile(r'\bbranch[ыа-я]+\b', re.IGNORECASE),   "branches"),
    (re.compile(r'\bendpoint[ыа-я]+\b', re.IGNORECASE), "endpoints"),
    (re.compile(r'\blog[ыа-яа-я]+\b', re.IGNORECASE),   "logs"),
    (re.compile(r'\bcache[а-я]+\b', re.IGNORECASE),     "cache"),
    (re.compile(r'\bimage[а-я]+\b', re.IGNORECASE),     "images"),
    (re.compile(r'\bbuild[ыа-я]+\b', re.IGNORECASE),    "builds"),
    (re.compile(r'\bdeployment[а-я]+\b', re.IGNORECASE), "deployments"),
    (re.compile(r'\bdeploy[а-я]+\b', re.IGNORECASE),    "deploys"),
    (re.compile(r'\bhook[ыа-я]+\b', re.IGNORECASE),     "hooks"),
    (re.compile(r'\bpipeline[ыа-я]+\b', re.IGNORECASE), "pipelines"),
    (re.compile(r'\bmigration[ыа-я]+\b', re.IGNORECASE), "migrations"),
    (re.compile(r'\bcomponent[ыа-я]+\b', re.IGNORECASE), "components"),
    (re.compile(r'\bfeature[ыа-я]+\b', re.IGNORECASE),  "features"),
    (re.compile(r'\bservice[ыа-я]+\b', re.IGNORECASE),  "services"),
    (re.compile(r'\bmodule[ыа-я]+\b', re.IGNORECASE),   "modules"),
]

# Repair -ни suffix (imperative hybrid): Rebaseни → ребейсни
MIXED_IMPERATIVE_REPAIR: list[tuple[re.Pattern, str]] = [
    (re.compile(r'\bRebase(ни|нись)\b', re.IGNORECASE), lambda m: "ребейс" + m.group(1)),
    (re.compile(r'\bMerge(ни|нись)\b', re.IGNORECASE), lambda m: "мёрдж" + m.group(1)),
    (re.compile(r'\bRefactor(ни|нись)\b', re.IGNORECASE), lambda m: "рефактор" + m.group(1)),
    (re.compile(r'\bCheckout(ни|нись)\b', re.IGNORECASE), lambda m: "чекаут" + m.group(1)),
    (re.compile(r'\bFetch(ни|нись)\b', re.IGNORECASE), lambda m: "фетч" + m.group(1)),
    (re.compile(r'\bDebug(ни|нись)\b', re.IGNORECASE), lambda m: "дебаг" + m.group(1)),
]

# Erroneous verb→English repairs: заfetch → зафетчил (approximate)
VERB_ENGLISH_REPAIR: list[tuple[re.Pattern, str]] = [
    (re.compile(r'\bза(fetch|фетч)(?:ил[аи]?|ить|ь)?\b', re.IGNORECASE), "зафетчил"),
    (re.compile(r'\bзаfetch\b', re.IGNORECASE), "зафетчил"),
]

# Corrupted concatenation patterns (class a) — rows to DELETE
# These are strings where a word and a latin term are fused without space
CORRUPT_PATTERN = re.compile(
    r'[а-яёА-ЯЁ][a-zA-Z]|[a-zA-Z][а-яёА-ЯЁ]'
)

# Additional explicit corrupt tokens (exact matches to delete)
CORRUPT_TOKEN_PATTERNS = [
    re.compile(r'Запушил[вВ]r?emot', re.IGNORECASE),
    re.compile(r'remote,закоммитил', re.IGNORECASE),
    re.compile(r'remote,закоммитил', re.IGNORECASE),
    re.compile(r'запушил[вВ]remote', re.IGNORECASE),
    re.compile(r'запушил[вВ]vremot', re.IGNORECASE),
    re.compile(r'\bвremote\b', re.IGNORECASE),
    re.compile(r'\bremoteна\b', re.IGNORECASE),
    re.compile(r'\bremoteзапушил\b', re.IGNORECASE),
    re.compile(r'Запушил[вВ]ремот\b', re.IGNORECASE),
]

# <unk> rows — delete
UNK_PATTERN = re.compile(r'<unk>')

# ---------------------------------------------------------------------------
# Normalization helpers
# ---------------------------------------------------------------------------

def is_russified_verb(token: str) -> bool:
    """Return True if token is a known russified verb that must stay Cyrillic."""
    t = token.lower().strip('.,!?;:')
    return t in RUSSIFIED_VERB_FORMS


# Pre-compile term map patterns once at module level for speed
_TERM_MAP_COMPILED: list[tuple[re.Pattern, str]] = [
    (re.compile(r'(?<![а-яёА-ЯЁa-zA-Z])' + re.escape(cyr_form) + r'(?![а-яёА-ЯЁa-zA-Z])',
                re.IGNORECASE), canonical)
    for cyr_form, canonical in TERM_MAP
]

# Pre-compile noun-cyrillic detection pattern (OR of all terms)
_NOUN_CYR_PATTERN = re.compile(
    '(?<![а-яёА-ЯЁa-zA-Z])(' +
    '|'.join(re.escape(cyr_form) for cyr_form, _ in TERM_MAP) +
    ')(?![а-яёА-ЯЁa-zA-Z])',
    re.IGNORECASE
)


# Pre-compile ALL mixed-repair patterns at module level
_APOS_HYPHEN_SUFFIX: list[tuple[re.Pattern, str | re.Match]] = [
    (re.compile(r"\bcontainer['''\-][а-яё]+\b", re.IGNORECASE), "containers"),
    (re.compile(r"\bbranch['''\-][а-яё]+\b", re.IGNORECASE), "branches"),
    (re.compile(r"\bcommit['''\-][а-яё]+\b", re.IGNORECASE), "commits"),
    (re.compile(r"\bpod['''\-][а-яё]+\b", re.IGNORECASE), "pods"),
    (re.compile(r"\bwebhook['''\-][а-яё]+\b", re.IGNORECASE), "webhooks"),
    (re.compile(r"\bstash['''\-][а-яё]+\b", re.IGNORECASE), "stashes"),
    (re.compile(r"\blog['''\-][а-яё]+\b", re.IGNORECASE), "logs"),
    (re.compile(r"\bclone['''\-][а-яё]+\b", re.IGNORECASE), "clone"),
    (re.compile(r"\bcheckout['''\-][а-яё]+\b", re.IGNORECASE), "checkout"),
    (re.compile(r"\brelease-нотс\b", re.IGNORECASE), "release notes"),
    (re.compile(r"\brelease-арт[а-яё]+\b", re.IGNORECASE), "release artifacts"),
    (re.compile(r"\btrace[а-яё]+\b", re.IGNORECASE), "trace"),
    (re.compile(r"\bstack trace[а-яё]+\b", re.IGNORECASE), "stack trace"),
    (re.compile(r"\bюнит-tests\b", re.IGNORECASE), "unit tests"),
    (re.compile(r"\bunit-тест[а-яё]*\b", re.IGNORECASE), "unit tests"),
    (re.compile(r"\bфич-request\b", re.IGNORECASE), "feature request"),
    (re.compile(r"\btest-кейс[а-яё]*\b", re.IGNORECASE), "test cases"),
    (re.compile(r"[Сс]лэш-endpoint\b"), "slash-endpoint"),
    (re.compile(r"\btyping-[а-яё]+\b", re.IGNORECASE), "typings"),
    (re.compile(r"\bPython-[а-яё]+\b"), "Python"),
    (re.compile(r"\bimage-[а-яё]+\b", re.IGNORECASE), "image"),
    (re.compile(r"\bbranch-[а-яё]+\b", re.IGNORECASE), "branch"),
    (re.compile(r"\bremote,([а-яёА-ЯЁ])", re.IGNORECASE), r"remote, \1"),
    (re.compile(r"\bremote-[а-яёА-ЯЁ][а-яё]+\b", re.IGNORECASE), "remote"),
    (re.compile(r"\bSlack-[а-яёА-ЯЁ][а-яё]+\b"), "Slack"),
    (re.compile(r"\bTelegram-[а-яёА-ЯЁ][а-яё]+\b"), "Telegram"),
    (re.compile(r"\bSQL-[а-яёА-ЯЁ][а-яё]+\b"), "SQL queries"),
]

_EXTRA_APOS: list[tuple[re.Pattern, str]] = [
    (re.compile(r"\bmigration['''\-][а-яё]+\b", re.IGNORECASE), "migrations"),
    (re.compile(r"\btoken['''\-][а-яё]+\b", re.IGNORECASE), "tokens"),
    (re.compile(r"\bbackend['''\-][а-яё]+\b", re.IGNORECASE), "backend"),
    (re.compile(r"\bWebpack['''\-][а-яё]+\b", re.IGNORECASE), "Webpack"),
    (re.compile(r"\bbuild['''\-][а-яё]+\b", re.IGNORECASE), "build"),
    (re.compile(r"\bdeploy['''\-][а-яё]+\b", re.IGNORECASE), "deploy"),
    (re.compile(r"\bpush['''\-][а-яё]+\b", re.IGNORECASE), "pushes"),
    (re.compile(r"\bendpoint['''\-][а-яё]+\b", re.IGNORECASE), "endpoints"),
    (re.compile(r"\bmerge['''']-?ить\b", re.IGNORECASE), "мёрджить"),
    (re.compile(r"\bfetch['''']-?ни\b", re.IGNORECASE), "фетчни"),
    (re.compile(r"\bdebug['''']-?нуть\b", re.IGNORECASE), "дебажнуть"),
    (re.compile(r"\bdebug-процесс\b", re.IGNORECASE), "debug process"),
    (re.compile(r"\brefactor['''']-?ь\b", re.IGNORECASE), "рефакторь"),
    (re.compile(r"\bпре-push\b", re.IGNORECASE), "pre-push"),
    (re.compile(r"\bпре-commit\b", re.IGNORECASE), "pre-commit"),
    (re.compile(r"\bе2[еe]\b", re.IGNORECASE), "e2e"),
    (re.compile(r"\bmerge-стратег[а-яё]+\b", re.IGNORECASE), "merge strategy"),
    (re.compile(r"\bконтент-review\b", re.IGNORECASE), "content review"),
    (re.compile(r"\bSDK-интеграц[а-яё]+\b", re.IGNORECASE), "SDK integration"),
    (re.compile(r"\bbuild-тайм[а-яё]*\b", re.IGNORECASE), "build time"),
    (re.compile(r"\bdocker-образ\b", re.IGNORECASE), "docker image"),
    (re.compile(r"\bdocker['''\-][а-яё]+\b", re.IGNORECASE), "docker"),
    (re.compile(r"\bReact-стайл\b", re.IGNORECASE), "React style"),
    (re.compile(r"\bSwift-[а-яё]+\b", re.IGNORECASE), "Swift"),
    (re.compile(r"[Сс]лэш-endpoint\b", re.IGNORECASE), "slash-endpoint"),
    (re.compile(r"\bфич-request\b", re.IGNORECASE), "feature request"),
    (re.compile(r"\btest-кейс[а-яё]*\b", re.IGNORECASE), "test cases"),
    (re.compile(r"\bюнит-tests\b", re.IGNORECASE), "unit tests"),
    (re.compile(r"\brelease-арт[а-яё]+\b", re.IGNORECASE), "release artifacts"),
]

_REMOTE_AT = re.compile(r'\bremote@[а-яё]+\b')
_LATIN_FILE = re.compile(
    r'(?<![@/])\b([a-zA-Z][\w.-]*\.(?:py|ts|tsx|js|jsx|mjs|json|toml|yaml|yml|md|txt|sql|csv|rs|go|sh|env|ini|cfg|lock))\b'
)
_NOT_FILES = frozenset({"Next.js", "Node.js", "Vue.js"})


def prefix_at_filenames(text: str) -> str:
    """Parakeet often emits latin filenames without @; OUT should always use @."""
    def repl(m: re.Match[str]) -> str:
        name = m.group(1)
        if name in _NOT_FILES:
            return name
        return "@" + name

    return _LATIN_FILE.sub(repl, text)


def apply_term_map(text: str) -> str:
    """Replace Cyrillic tech-term spans with canonical Latin, multi-word first."""
    result = text
    for pattern, canonical in _TERM_MAP_COMPILED:
        result = pattern.sub(canonical, result)
    return result


def apply_mixed_repairs(text: str) -> str:
    """Fix mixed-alphabet tokens in text."""
    # Class (b): Latin verb stem + Cyrillic suffix → full Cyrillic
    for pattern, repl in MIXED_VERB_REPAIR:
        text = pattern.sub(repl, text)
    # Class (c): Latin noun + Cyrillic suffix → full Latin
    for pattern, repl in MIXED_NOUN_REPAIR:
        text = pattern.sub(repl, text)
    # Imperative hybrids
    for pattern, repl in MIXED_IMPERATIVE_REPAIR:
        text = pattern.sub(repl, text)
    # Erroneous заfetch etc
    for pattern, repl in VERB_ENGLISH_REPAIR:
        text = pattern.sub(repl, text)
    # Apostrophe/hyphen noun suffixes
    for pattern, repl in _APOS_HYPHEN_SUFFIX:
        text = pattern.sub(repl, text)
    # remote@и → remote (stray @)
    text = _REMOTE_AT.sub('remote', text)
    # Extra compound repairs
    for pattern, repl in _EXTRA_APOS:
        text = pattern.sub(repl, text)
    return text


def is_corrupted_concat(text: str) -> bool:
    """
    Return True if the row looks like a corrupted concatenation artifact that
    cannot be cleanly fixed: Cyrillic and Latin fused within a word without
    separator, e.g. 'remoteзапушил', 'Запушилvremote'.
    """
    for pat in CORRUPT_TOKEN_PATTERNS:
        if pat.search(text):
            return True
    if UNK_PATTERN.search(text):
        return True
    # Detect fused patterns: letter boundary Cyr↔Lat within a non-@ token
    for token in text.split():
        if token.startswith('@'):
            continue
        # Remove surrounding punctuation
        t = token.strip('.,!?;:()')
        if not t:
            continue
        has_cyr = bool(CYR.search(t))
        has_lat = bool(LAT.search(t))
        if has_cyr and has_lat:
            # Check if it's a known "ok" pattern: e.g. @file, CI/CD, numbers
            if '/' in t or t.upper() == t:
                continue
            # If it's a known russified verb form, it's fine (but shouldn't have lat)
            if is_russified_verb(t):
                continue
            # Otherwise it's mixed — check if it looks like a fusion (no separator)
            # Detect alternating scripts
            if re.search(r'[а-яёА-ЯЁ][a-zA-Z]|[a-zA-Z][а-яёА-ЯЁ]', t):
                # Try to classify: if it matches a known repairable pattern, not corrupt
                repaired = apply_mixed_repairs(token)
                if repaired != token:
                    return False  # repairable
                # Check if term map would fix it
                # If still mixed after repairs, flag as corrupt
                return True
    return False


def normalize_out(out: str) -> str:
    """Apply all normalization rules to an OUT string."""
    text = out

    # 1. Fix mixed-alphabet repairs first
    text = apply_mixed_repairs(text)

    # 2. Apply term map (Cyrillic → Latin)
    text = apply_term_map(text)

    # 3. @-prefix for latin filenames (incl. when ASR already wrote them in latin)
    text = prefix_at_filenames(text)

    # 4. Clean up any residual spacing issues from replacements
    text = re.sub(r'  +', ' ', text).strip()

    return text


def normalize_key(text: str) -> str:
    """Produce a normalized key for dedup (lowercase, strip punctuation)."""
    t = text.lower()
    t = re.sub(r'[^\w\s]', '', t)
    t = re.sub(r'\s+', ' ', t).strip()
    return t


def has_noun_cyrillic(text: str) -> bool:
    """Check if text has Cyrillic tech terms that should be Latin."""
    return bool(_NOUN_CYR_PATTERN.search(text))


def has_mixed_alphabet_token(text: str) -> bool:
    """Check if text has mixed-alphabet tokens (excluding @-files and all-caps abbreviations)."""
    for token in text.split():
        if token.startswith('@'):
            continue
        t = token.strip('.,!?;:()')
        if not t:
            continue
        if '/' in t:
            continue
        has_cyr = bool(CYR.search(t))
        has_lat = bool(LAT.search(t))
        if has_cyr and has_lat:
            return True
    return False


def has_erroneous_nochange(in_text: str, out_text: str) -> bool:
    """Return True if IN==OUT and IN has a term that should have been converted."""
    if in_text != out_text:
        return False
    return has_noun_cyrillic(in_text)


def count_russified_verbs(rows: list[tuple[str, str]]) -> int:
    """Count rows where OUT contains at least one russified verb form."""
    count = 0
    for _, out in rows:
        for form in RUSSIFIED_VERB_FORMS:
            if form in out.lower():
                count += 1
                break
    return count


# ---------------------------------------------------------------------------
# Main processing
# ---------------------------------------------------------------------------

def process_v3(input_path: Path | None = None) -> tuple[list[tuple[str, str]], dict]:
    src = input_path or V3
    stats = {
        "in_rows": 0,
        "deleted_corrupt": 0,
        "deleted_unk": 0,
        "fixed_noun_cyr": 0,
        "fixed_mixed": 0,
        "fixed_nochange": 0,
        "dedup_exact": 0,
        "dedup_punct": 0,
        "contradiction_resolved": 0,
    }

    raw_rows: list[tuple[str, str]] = []
    with open(src, encoding="utf-8") as f:
        reader = csv.reader(f, delimiter='\t')
        header = next(reader)
        for row in reader:
            if len(row) == 2:
                raw_rows.append((row[0].strip(), row[1].strip()))
    stats["in_rows"] = len(raw_rows)

    # Pass 1: Delete corrupted / UNK rows
    clean_rows: list[tuple[str, str]] = []
    for in_, out in raw_rows:
        if UNK_PATTERN.search(in_) or UNK_PATTERN.search(out):
            stats["deleted_unk"] += 1
            continue
        if is_corrupted_concat(out) or is_corrupted_concat(in_):
            stats["deleted_corrupt"] += 1
            continue
        clean_rows.append((in_, out))

    # Pass 2: Normalize OUT
    normalized: list[tuple[str, str]] = []
    for in_, out in clean_rows:
        new_out = normalize_out(out)
        if new_out != out:
            stats["fixed_mixed"] += 1
        # Also check if noun-cyrillic remains after general normalization
        if has_noun_cyrillic(new_out):
            stats["fixed_noun_cyr"] += 1
            # Apply one more pass (shouldn't be needed but defensive)
            new_out = apply_term_map(new_out)

        # Fix erroneous no-change: if IN==OUT and IN contains a term
        if in_ == out and has_noun_cyrillic(in_):
            new_out = normalize_out(in_)
            stats["fixed_nochange"] += 1

        normalized.append((in_, new_out))

    # Pass 3: Dedup
    # Exact dedup
    seen_exact: dict[tuple[str, str], bool] = {}
    after_exact: list[tuple[str, str]] = []
    for row in normalized:
        if row not in seen_exact:
            seen_exact[row] = True
            after_exact.append(row)
        else:
            stats["dedup_exact"] += 1

    # Punctuation-only dedup: same normalized IN → keep rule-consistent OUT
    # Group by normalized IN key
    key_to_rows: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for row in after_exact:
        key = normalize_key(row[0])
        key_to_rows[key].append(row)

    after_dedup: list[tuple[str, str]] = []
    for key, group in key_to_rows.items():
        if len(group) == 1:
            after_dedup.append(group[0])
            continue
        # Multiple rows with same normalized IN — check for contradictions
        # Group by normalized OUT
        out_variants: dict[str, list[tuple[str, str]]] = defaultdict(list)
        for row in group:
            out_variants[normalize_key(row[1])].append(row)

        if len(out_variants) == 1:
            # Same OUT (modulo punct) — keep first, dedup rest
            stats["dedup_punct"] += len(group) - 1
            after_dedup.append(group[0])
        else:
            # Contradiction: pick the rule-consistent OUT
            # Score each variant: prefer the one with fewest noun-Cyrillic and no mixed
            def score(row: tuple[str, str]) -> int:
                _, out = row
                s = 0
                if has_noun_cyrillic(out):
                    s += 10
                if has_mixed_alphabet_token(out):
                    s += 5
                return s

            best = min(group, key=score)
            discarded = [r for r in group if r != best]
            stats["contradiction_resolved"] += len(discarded)
            after_dedup.append(best)

    return after_dedup, stats


def compute_defect_counts(rows: list[tuple[str, str]]) -> dict:
    noun_cyr = sum(1 for _, out in rows if has_noun_cyrillic(out))
    mixed = sum(1 for _, out in rows if has_mixed_alphabet_token(out))
    no_change = sum(1 for in_, out in rows if in_ == out and has_noun_cyrillic(in_))
    verbs_cyr = count_russified_verbs(rows)
    return {
        "noun_cyrillic_out": noun_cyr,
        "mixed_alphabet": mixed,
        "erroneous_nochange": no_change,
        "verbs_cyrillic": verbs_cyr,
        "total": len(rows),
    }


def write_tsv(path: Path, rows: list[tuple[str, str]]) -> None:
    with open(path, 'w', encoding='utf-8', newline='') as f:
        writer = csv.writer(f, delimiter='\t')
        writer.writerow(["IN", "OUT"])
        writer.writerows(rows)


def write_jsonl(path: Path, rows: list[tuple[str, str]]) -> None:
    with open(path, 'w', encoding='utf-8') as f:
        for in_, out in rows:
            f.write(json.dumps({"in": in_, "out": out}, ensure_ascii=False) + '\n')


def categorize(in_text: str, out_text: str) -> str:
    """Assign row to a category for stratified eval split."""
    if in_text == out_text:
        return "nochange"
    words = in_text.split()
    if len(words) >= 15:
        return "long"
    return "short"


def make_splits(rows: list[tuple[str, str]], eval_n: int = 230) -> tuple[list, list]:
    """
    Stratified split: pick ~eval_n rows for eval with zero normalized-IN overlap
    with train, stratified by category (nochange / long / short).
    """
    # Categorize
    cats: dict[str, list[tuple[str, str]]] = defaultdict(list)
    for row in rows:
        cats[categorize(*row)].append(row)

    total = len(rows)
    # Target proportions
    targets = {
        "nochange": int(eval_n * len(cats["nochange"]) / total),
        "long":     int(eval_n * len(cats["long"]) / total),
        "short":    int(eval_n * len(cats["short"]) / total),
    }
    # Make up rounding difference in 'short'
    diff = eval_n - sum(targets.values())
    targets["short"] += diff

    import random
    random.seed(42)

    eval_rows: list[tuple[str, str]] = []
    train_rows: list[tuple[str, str]] = []
    eval_in_keys: set[str] = set()

    for cat, target in targets.items():
        pool = list(cats[cat])
        random.shuffle(pool)
        picked = 0
        cat_eval: list[tuple[str, str]] = []
        cat_train: list[tuple[str, str]] = []
        for row in pool:
            key = normalize_key(row[0])
            if picked < target and key not in eval_in_keys:
                # Extra quality filter for eval: no defects
                _, out = row
                if has_noun_cyrillic(out):
                    cat_train.append(row)
                    continue
                if has_mixed_alphabet_token(out):
                    cat_train.append(row)
                    continue
                if row[0] == row[1] and has_noun_cyrillic(row[0]):
                    cat_train.append(row)
                    continue
                cat_eval.append(row)
                eval_in_keys.add(key)
                picked += 1
            else:
                cat_train.append(row)
        eval_rows.extend(cat_eval)
        train_rows.extend(cat_train)

    # Verify train doesn't share keys with eval
    train_keys = {normalize_key(r[0]) for r in train_rows}
    assert not (eval_in_keys & train_keys), "Overlap detected!"

    return train_rows, eval_rows


def verify_eval(eval_rows: list[tuple[str, str]]) -> dict:
    noun_cyr = sum(1 for _, out in eval_rows if has_noun_cyrillic(out))
    mixed = sum(1 for _, out in eval_rows if has_mixed_alphabet_token(out))
    no_change = sum(1 for in_, out in eval_rows if in_ == out and has_noun_cyrillic(in_))
    return {
        "noun_cyrillic_out": noun_cyr,
        "mixed_alphabet": mixed,
        "erroneous_nochange": no_change,
        "total": len(eval_rows),
    }


def main():
    import argparse

    ap = argparse.ArgumentParser(description="Clean IN/OUT dataset and rebuild splits")
    ap.add_argument("--input", type=Path, default=V3, help="input TSV (default: dataset.v3.tsv)")
    ap.add_argument("--output", type=Path, default=V4, help="output TSV (default: dataset.v4.tsv)")
    ap.add_argument("--train", type=Path, default=TRAIN_OUT, help="train.jsonl output")
    ap.add_argument("--eval", type=Path, default=EVAL_OUT, help="eval.jsonl output")
    args = ap.parse_args()

    print("=" * 60)
    print(f"clean_dataset.py  {args.input.name} → {args.output.name}")
    print("=" * 60)

    rows, stats = process_v3(args.input)

    print(f"\nProcessing stats:")
    print(f"  Input rows:               {stats['in_rows']}")
    print(f"  Deleted (<unk>):           {stats['deleted_unk']}")
    print(f"  Deleted (corrupt concat): {stats['deleted_corrupt']}")
    print(f"  Fixed mixed/noun-cyr:     {stats['fixed_mixed']}")
    print(f"  Fixed nochange:           {stats['fixed_nochange']}")
    print(f"  Dedup exact:              {stats['dedup_exact']}")
    print(f"  Dedup punct-only:         {stats['dedup_punct']}")
    print(f"  Contradictions resolved:  {stats['contradiction_resolved']}")

    # Defect counts before (on input)
    print(f"\n--- {args.input.name} defect counts ---")
    v3_rows = []
    with open(args.input, encoding="utf-8") as f:
        reader = csv.reader(f, delimiter='\t')
        next(reader)
        for row in reader:
            if len(row) == 2:
                v3_rows.append((row[0].strip(), row[1].strip()))
    v3_defects = compute_defect_counts(v3_rows)
    for k, v in v3_defects.items():
        print(f"  {k}: {v}")

    # Defect counts after (on output)
    print(f"\n--- {args.output.name} defect counts ---")
    v4_defects = compute_defect_counts(rows)
    for k, v in v4_defects.items():
        print(f"  {k}: {v}")

    write_tsv(args.output, rows)
    print(f"\nWrote {args.output}  ({len(rows)} rows)")

    # Backup existing splits
    if args.train.exists():
        bak = args.train.with_suffix(args.train.suffix + ".bak")
        shutil.copy(args.train, bak)
        print(f"Backed up {args.train} → {bak}")
    if args.eval.exists():
        bak = args.eval.with_suffix(args.eval.suffix + ".bak")
        shutil.copy(args.eval, bak)
        print(f"Backed up {args.eval} → {bak}")

    # Generate splits
    train_rows, eval_rows = make_splits(rows, eval_n=230)

    # Verify eval quality
    print("\n--- Eval quality check ---")
    eval_check = verify_eval(eval_rows)
    for k, v in eval_check.items():
        print(f"  {k}: {v}")

    # Check overlap
    train_keys = {normalize_key(r[0]) for r in train_rows}
    eval_keys = {normalize_key(r[0]) for r in eval_rows}
    overlap = train_keys & eval_keys
    print(f"  train/eval IN overlap:    {len(overlap)} (should be 0)")

    write_jsonl(args.train, train_rows)
    write_jsonl(args.eval, eval_rows)
    print(f"\nWrote {args.train}  ({len(train_rows)} rows)")
    print(f"Wrote {args.eval}  ({len(eval_rows)} rows)")

    if eval_check["noun_cyrillic_out"] == 0 and eval_check["mixed_alphabet"] == 0 \
            and eval_check["erroneous_nochange"] == 0 and len(overlap) == 0:
        print("\n✓ Eval is CLEAN: 0 noun-Cyrillic, 0 mixed-alphabet, 0 erroneous-nochange, 0 overlap")
    else:
        print("\n⚠ Eval has remaining defects — inspect manually")
        if eval_check["noun_cyrillic_out"] > 0:
            print("  Sample noun-Cyrillic in eval OUT:")
            for in_, out in eval_rows:
                if has_noun_cyrillic(out):
                    print(f"    {in_!r} → {out!r}")
                    break
        if eval_check["mixed_alphabet"] > 0:
            print("  Sample mixed-alphabet in eval OUT:")
            for in_, out in eval_rows:
                if has_mixed_alphabet_token(out):
                    print(f"    {in_!r} → {out!r}")
                    break

    print("\nDone.")


if __name__ == "__main__":
    main()
