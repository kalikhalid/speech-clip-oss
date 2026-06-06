"""Банк терминов для генерации датасета. Узкий, но покрывает dev-широту.
Каждый термин будет встречаться МНОГО раз (frequency = recall).
Файлы помечаем через @. Длинный хвост/личные бренды — в словарь приложения, не сюда.
"""

# Команды и инструменты (латиница без @)
TERMS = [
    # git
    "commit", "push", "pull", "merge", "rebase", "checkout", "branch", "clone",
    "stash", "fetch", "git", "master", "main", "origin", "remote", "pull request",
    "force push", "cherry-pick", "diff", "log",
    # docker / devops
    "docker", "docker compose", "container", "image", "build", "deploy", "kubernetes",
    "kubectl", "pod", "nginx", "systemctl", "ssh", "VPN", "CI/CD", "GitHub Actions",
    "Terraform", "Ansible", "prod", "staging", "rollback",
    # backend / языки
    "backend", "frontend", "API", "endpoint", "request", "response", "JSON", "SQL",
    "query", "migration", "cache", "Redis", "Postgres", "MongoDB", "token", "JWT",
    "webhook", "cron", "queue", "Kafka", "gRPC", "REST", "GraphQL",
    # языки/рантаймы
    "Python", "JavaScript", "TypeScript", "Rust", "cargo", "Go", "Node", "npm",
    "yarn", "pip", "Java", "Kotlin", "Swift",
    # frontend
    "React", "Vue", "component", "props", "state", "hook", "CSS", "HTML", "DOM",
    "Tailwind", "Vite", "Webpack", "bundle", "Next.js", "build", "lint", "Prettier",
    # тесты / процесс
    "test", "unit test", "mock", "coverage", "debug", "log", "stack trace",
    "merge conflict", "code review", "refactor", "release",
    # инструменты / сервисы
    "Slack", "Notion", "Zoom", "Figma", "Jira", "GitHub", "GitLab", "Telegram",
    "VS Code", "Cursor", "Linear", "Postman", "Excel", "Google Docs",
    # аббревиатуры
    "UI", "UX", "URL", "IP", "DNS", "HTTP", "HTTPS", "PDF", "CSV", "RAM", "CPU",
    "MVP", "PR", "QA", "SDK", "CLI", "env",
]

# Имена файлов — в OUT ВСЕГДА через @
FILE_TERMS = [
    "@README.md", "@package.json", "@Cargo.toml", "@.env", "@tsconfig.json",
    "@docker-compose.yml", "@Dockerfile", "@.gitignore", "@vite.config.ts",
    "@next.config.js", "@go.mod", "@requirements.txt", "@index.html", "@main.rs",
    "@App.tsx", "@styles.css", "@config.yaml", "@Makefile", "@settings.json",
    "@pom.xml", "@webpack.config.js", "@.eslintrc", "@schema.sql",
    "@clean_dataset.py", "@merge_dataset.py", "@eval_model.py", "@generate_openai.py",
    "@integrate_model.sh", "@normalizer.rs", "@commands.rs", "@history.json",
]
