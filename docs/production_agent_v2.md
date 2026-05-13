# EVE Production Agent v2

Production Agent v2 extends the governed local loop with repo-aware planning,
structured proposal preview, apply dry-run, task outcome memory, and readiness
checks.

Core loop:

```text
task -> inspect -> repo-map -> plan -> propose -> proposal-show -> apply --dry-run -> approve -> apply -> validate -> report -> pr-summary -> task-outcome
```

OpenAI may be used as the reasoning/proposal layer when `OPENAI_API_KEY` is
configured. Rule-based fallback remains mandatory and sufficient for basic
operation.

Future evolution is based on real task outcomes: tasks, plans, proposals,
approvals, apply results, validations, reports, PR summaries, operator
decisions, failed proposals, and successful patches.
