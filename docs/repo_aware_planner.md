# Repo-Aware Planner

`cargo run -- repo-map` builds a metadata-only map from:

```text
Cargo.toml
src/main.rs
src/lib.rs
src/**/*.rs
tests/**/*.rs
docs/**/*.md
README.md
```

It ignores `.git/`, `target/`, `memory/`, `releases/`, `sandboxes/`, and test
artifact roots.

The planner uses this map to prefer docs files for documentation tasks, tests
for test tasks, and `src/main.rs` plus agent modules for CLI command tasks.
