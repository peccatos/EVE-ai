# eva_runtime_with_task_validator example

## Root
`C:\Users\burav\Desktop\brain`

## Binary entrypoint
- `C:\Users\burav\Desktop\brain\src\main.rs`
- `C:\Users\burav\Desktop\brain\Cargo.toml`

## Default local runtime mode

### Input files
- Example input: `C:\Users\burav\Desktop\brain\input.example.json`
- Active input used by `cargo run`: `C:\Users\burav\Desktop\brain\input.json`

### Command
```powershell
Copy-Item C:\Users\burav\Desktop\brain\input.example.json C:\Users\burav\Desktop\brain\input.json
cargo run
```

### Output
- stdout: Russian project phase report JSON
- Phase report logic: `C:\Users\burav\Desktop\brain\src\project_phase_report.rs`

## Repository patch mode

### Command
```powershell
cargo run -- --repo <REPO_URL>
```

### Optional flags
```powershell
cargo run -- --repo <REPO_URL> --branch <BRANCH> --max-changed-files 10 --report-path ./eva_output/report.md --machine-summary-path ./eva_output/summary.json
```

### Logic
- Repo analysis and patch pipeline: `C:\Users\burav\Desktop\brain\src\repo_patch_report.rs`

### Default outputs
- Markdown report: `C:\Users\burav\Desktop\brain\eva_output\report.md`
- Machine summary: `C:\Users\burav\Desktop\brain\eva_output\summary.json`

### Stdout contract
```text
[repo]
<repo_url>

[report]
./eva_output/report.md

[changed_files]
- path/to/file

[status]
ok
```

## Tests
- Repo patch tests: `C:\Users\burav\Desktop\brain\tests\repo_patch_report_tests.rs`
- Phase report tests: `C:\Users\burav\Desktop\brain\tests\project_phase_report_tests.rs`

## Notes
- Default mode reads `input.json`. If that file is missing, the binary falls back to an internal demo input.
- Repo patch mode does not use `input.json`; it is activated only by `--repo`.
- If you want a shareable repo patch result, open:
  - `C:\Users\burav\Desktop\brain\eva_output\report.md`
  - `C:\Users\burav\Desktop\brain\eva_output\summary.json`
