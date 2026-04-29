use eva_runtime_with_task_validator::{
    build_project_phase_runtime_output, run_repo_patch_report, serve_runtime_daemon,
    should_run_repo_patch_mode, CycleInput, RepoPatchCliConfig, RuntimeCliCommand,
    RuntimeCycleRunner, RUNTIME_CLI_HELP,
};
use serde::Deserialize;
use std::fs;
use std::path::Path;

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if should_run_repo_patch_mode(args.iter().map(String::as_str)) {
        match RepoPatchCliConfig::parse_from_iter(args) {
            Ok(config) => match run_repo_patch_report(&config) {
                Ok(execution) => println!("{}", execution.stdout_output()),
                Err(err) => {
                    eprintln!("repo_patch_error: {err}");
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("repo_patch_cli_error: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    match RuntimeCliCommand::parse_from_iter(args) {
        Ok(RuntimeCliCommand::Help) => {
            println!("{RUNTIME_CLI_HELP}");
            return;
        }
        Ok(RuntimeCliCommand::Once) => {}
        Ok(RuntimeCliCommand::Serve(config)) => {
            if let Err(err) = serve_runtime_daemon(config) {
                eprintln!("runtime_daemon_error: {err}");
                std::process::exit(1);
            }
            return;
        }
        Err(err) => {
            eprintln!("runtime_cli_error: {err}");
            eprintln!("run `cargo run` for available commands");
            std::process::exit(1);
        }
    }

    let input = load_input("input.json").unwrap_or_else(|_| CycleInput {
        goal: "получить фазовый отчёт EVA по локальному runtime циклу".to_string(),
        external_state: "локальный demo режим без внешних сервисов".to_string(),
    });

    let mut runner = RuntimeCycleRunner::new();
    match runner.run_cycle_report(input) {
        Ok(report) => {
            let output = build_project_phase_runtime_output(&report);
            println!(
                "{}",
                serde_json::to_string_pretty(&output).expect("serialize runtime phase output")
            );
        }
        Err(err) => {
            eprintln!("runtime_cycle_error: {err}");
            std::process::exit(1);
        }
    }
}

#[derive(Debug, Deserialize)]
struct InputFile {
    goal: String,
    context: String,
}

fn load_input(path: impl AsRef<Path>) -> Result<CycleInput, String> {
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let input: InputFile = serde_json::from_str(&contents).map_err(|err| err.to_string())?;
    Ok(CycleInput {
        goal: input.goal,
        external_state: input.context,
    })
}
