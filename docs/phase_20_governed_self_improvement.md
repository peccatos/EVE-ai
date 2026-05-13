# Phase 20G Governed Self-Improvement

Command:

```bash
cargo run -- self-improve propose
```

Self-improvement is proposal-only. It reads task outcomes, patterns, and fitness
signals, then proposes a normal operator-gated task.

It cannot auto-approve, auto-apply, weaken validation, weaken approval gates,
change CI to skip tests, mutate `.git/`, mutate `memory/`, or bypass safe path
policy.
