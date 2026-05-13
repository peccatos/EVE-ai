# Proposal Preview And Dry Run

Preview:

```bash
cargo run -- proposal-show <PROPOSAL_ID>
```

Dry run:

```bash
cargo run -- apply --dry-run <PROPOSAL_ID>
cargo run -- apply <PROPOSAL_ID> --dry-run
```

Dry-run is read-only. It does not mutate files, create snapshots, create
rollback manifests, approve proposals, update task status, or call OpenAI.
