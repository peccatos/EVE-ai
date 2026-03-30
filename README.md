# eva_runtime_with_task_validator demo

Компактная демонстрационная версия EVA для локального запуска.

## Сценарии

1. `cargo run` — локальный runtime cycle с русским фазовым отчётом.
2. `cargo run -- --repo <REPO_URL>` — анализ репозитория и patch report по файлам.
3. `cargo run --bin github_repo_discover`, `github_repo_prepare`, `benchmark_batch` — benchmark pipeline.

## Быстрый старт

### Локальный runtime

```powershell
Copy-Item .\input.example.json .\input.json
cargo run
```

### Repo patch mode

```powershell
cargo run -- --repo <REPO_URL>
```

Результат:
- `./eva_output/report.md`
- `./eva_output/summary.json`

### Offline benchmark demo

```powershell
cargo run --bin github_repo_discover -- --fixture fixtures/github_search_fixture.json
cargo run --bin github_repo_prepare
cargo run --bin benchmark_batch
```

Результат:
- `benchmarks/rust_cases.json`
- `benchmarks/rust_cases_prepared.json`
- `benchmarks/rust_cases_ready.json`
- `benchmarks/rust_batch_report.json`

## Git bootstrap

Будущий origin:
- `https://github.com/peccatos/eva-brain-repo`