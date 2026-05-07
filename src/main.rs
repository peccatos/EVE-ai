use eva_runtime_with_task_validator::{
    adjust_task_from_campaign, autonomy_status, build_project_phase_runtime_output, candidate_diff,
    default_corpus_contract, distill_patterns, fix_generated_test_names, ingest_corpus,
    ingest_repo_patterns, latest_corpus_id, learning_summary, list_adjusted_tasks,
    list_bounded_runs, list_candidates, list_corpora, list_suggested_tasks, load_corpus_summary,
    load_metrics, preview_campaign_recombination, print_benchmark, print_bounded_run_report,
    print_campaign, print_campaign_report, print_evolution_policy, print_hygiene_plan,
    print_hygiene_report, print_last_bounded_run, print_last_campaign_report, print_last_report,
    print_last_task_adjustment, print_portfolio, print_quality_report, print_report,
    print_strategy_portfolio, promote_candidate, refresh_metrics, refresh_portfolio,
    refresh_report, refresh_strategy_portfolio, render_plans, render_recombined_hypotheses,
    replay_candidate, review_candidate, run_benchmark, run_bounded_evolution, run_evolution_cycle,
    run_planned_cycles, run_planned_evolution_cycle, run_recombined_evolution_cycle,
    run_repo_patch_report, run_stored_campaign, run_task_from_path, serve_runtime_daemon,
    should_run_repo_patch_mode, suggest_strategy_tasks, CycleInput, RepoPatchCliConfig,
    RuntimeCliCommand, RuntimeCycleRunner, RUNTIME_CLI_HELP,
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
        Ok(RuntimeCliCommand::Evolve) => {
            if let Err(err) = run_evolution_cycle(".") {
                eprintln!("evolution_cycle_error: {err}");
                std::process::exit(1);
            }
            println!("evolution_cycle_status: ok");
            return;
        }
        Ok(RuntimeCliCommand::PlanEvolution) => {
            match render_plans("memory") {
                Ok(output) => println!("{output}"),
                Err(err) => {
                    eprintln!("plan_evolution_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::EvolvePlanned) => {
            if let Err(err) = run_planned_evolution_cycle(".", "memory") {
                eprintln!("planned_evolution_error: {err}");
                std::process::exit(1);
            }
            println!("planned_evolution_status: ok");
            return;
        }
        Ok(RuntimeCliCommand::EvolvePlannedN(count)) => {
            match run_planned_cycles(".", "memory", count) {
                Ok(run_ids) => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&run_ids).expect("serialize run ids")
                    )
                }
                Err(err) => {
                    eprintln!("planned_evolution_n_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::EvolutionBenchmark(count)) => {
            match run_benchmark(".", "memory", count) {
                Ok(benchmark) => println!("{}", print_benchmark(&benchmark)),
                Err(err) => {
                    eprintln!("evolution_benchmark_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::AutonomyStatus) => {
            match autonomy_status(".", "memory") {
                Ok(status) => println!(
                    "{}",
                    serde_json::to_string_pretty(&status).expect("serialize autonomy status")
                ),
                Err(err) => {
                    eprintln!("autonomy_status_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::Metrics) => {
            match load_metrics("memory") {
                Ok(metrics) => println!(
                    "{}",
                    serde_json::to_string_pretty(&metrics).expect("serialize metrics")
                ),
                Err(err) => {
                    eprintln!("metrics_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::MetricsRefresh) => {
            match refresh_metrics("memory") {
                Ok(metrics) => println!(
                    "{}",
                    serde_json::to_string_pretty(&metrics).expect("serialize metrics")
                ),
                Err(err) => {
                    eprintln!("metrics_refresh_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::Portfolio) => {
            match print_portfolio("memory") {
                Ok(summary) => println!("{summary}"),
                Err(err) => {
                    eprintln!("portfolio_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::PortfolioRefresh) => {
            match refresh_portfolio("memory") {
                Ok(portfolio) => println!(
                    "{}",
                    serde_json::to_string_pretty(&portfolio).expect("serialize portfolio")
                ),
                Err(err) => {
                    eprintln!("portfolio_refresh_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::StrategyPortfolio) => {
            match print_strategy_portfolio("memory") {
                Ok(summary) => println!("{summary}"),
                Err(err) => {
                    eprintln!("strategy_portfolio_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::StrategyPortfolioRefresh) => {
            match refresh_strategy_portfolio("memory") {
                Ok(portfolio) => println!(
                    "{}",
                    serde_json::to_string_pretty(&portfolio).expect("serialize strategy portfolio")
                ),
                Err(err) => {
                    eprintln!("strategy_portfolio_refresh_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::EvolutionPolicy) => {
            match print_evolution_policy(".", "memory", None) {
                Ok(policy) => println!("{policy}"),
                Err(err) => {
                    eprintln!("evolution_policy_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::QualityReport(run_id)) => {
            match print_quality_report("memory", &run_id) {
                Ok(report) => println!("{report}"),
                Err(err) => {
                    eprintln!("quality_report_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::EvolutionHygiene) => {
            match print_hygiene_report(".", "memory") {
                Ok(report) => println!("{report}"),
                Err(err) => {
                    eprintln!("evolution_hygiene_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::HygienePlan) => {
            match print_hygiene_plan(".", "memory") {
                Ok(plan) => println!("{plan}"),
                Err(err) => {
                    eprintln!("hygiene_plan_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::HygieneFixGeneratedTests) => {
            match fix_generated_test_names(".") {
                Ok(status) => println!("{status}"),
                Err(err) => {
                    eprintln!("hygiene_fix_generated_tests_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::IngestCorpus(path)) => {
            match ingest_corpus("memory", &default_corpus_contract(&path)) {
                Ok(summary) => println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).expect("serialize corpus summary")
                ),
                Err(err) => {
                    eprintln!("ingest_corpus_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::IngestCorpusContract(path)) => {
            let contract = fs::read_to_string(&path)
                .map_err(|error| format!("failed to read corpus contract: {error}"))
                .and_then(|contents| {
                    serde_json::from_str::<eva_runtime_with_task_validator::CorpusIngestContract>(
                        &contents,
                    )
                    .map_err(|error| format!("failed to parse corpus contract: {error}"))
                });
            match contract.and_then(|contract| ingest_corpus("memory", &contract)) {
                Ok(summary) => println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).expect("serialize corpus summary")
                ),
                Err(err) => {
                    eprintln!("ingest_corpus_contract_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::CorpusSummary(corpus_id)) => {
            let resolved_id = resolve_corpus_alias("memory", &corpus_id);
            match resolved_id.and_then(|resolved| load_corpus_summary("memory", &resolved)) {
                Ok(summary) => println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).expect("serialize corpus summary")
                ),
                Err(err) => {
                    eprintln!("corpus_summary_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::ListCorpora) => {
            match list_corpora("memory") {
                Ok(corpora) => println!("{}", render_corpora_listing("memory", &corpora)),
                Err(err) => {
                    eprintln!("list_corpora_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::SuggestStrategyTasks(corpus_id)) => {
            let resolved_id = resolve_corpus_alias("memory", &corpus_id);
            match resolved_id.and_then(|resolved| suggest_strategy_tasks("memory", &resolved)) {
                Ok(tasks) => println!(
                    "{}",
                    serde_json::to_string_pretty(&tasks).expect("serialize suggested tasks")
                ),
                Err(err) => {
                    eprintln!("suggest_strategy_tasks_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::ListSuggestedTasks) => {
            match list_suggested_tasks("memory") {
                Ok(tasks) => println!("{}", tasks.join("\n")),
                Err(err) => {
                    eprintln!("list_suggested_tasks_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::LearningSummary) => {
            match learning_summary("memory") {
                Ok(summary) => println!("{summary}"),
                Err(err) => {
                    eprintln!("learning_summary_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::LastReport) => {
            match print_last_report("memory") {
                Ok(report) => println!("{report}"),
                Err(err) => {
                    eprintln!("last_report_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::Report(run_id)) => {
            match print_report("memory", &run_id) {
                Ok(report) => println!("{report}"),
                Err(err) => {
                    eprintln!("report_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::ReportRefresh(run_id)) => {
            match refresh_report("memory", &run_id) {
                Ok(report) => println!(
                    "{}",
                    serde_json::to_string_pretty(&report).expect("serialize refreshed report")
                ),
                Err(err) => {
                    eprintln!("report_refresh_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::ReviewCandidate(run_id)) => {
            match review_candidate(".", "memory", &run_id) {
                Ok(review) => println!(
                    "{}",
                    serde_json::to_string_pretty(&review).expect("serialize candidate review")
                ),
                Err(err) => {
                    eprintln!("review_candidate_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::CandidateDiff(run_id)) => {
            match candidate_diff("memory", &run_id) {
                Ok(diff) => println!("{diff}"),
                Err(err) => {
                    eprintln!("candidate_diff_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::ListCandidates) => {
            match list_candidates("memory") {
                Ok(output) => println!("{output}"),
                Err(err) => {
                    eprintln!("list_candidates_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::RunTask(path)) => {
            match run_task_from_path(".", "memory", &path) {
                Ok(campaign) => println!("{}", print_campaign(&campaign)),
                Err(err) => {
                    eprintln!("run_task_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::Campaign(task_id)) => {
            match run_stored_campaign(".", "memory", &task_id) {
                Ok(campaign) => println!("{}", print_campaign(&campaign)),
                Err(err) => {
                    eprintln!("campaign_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::LastCampaignReport) => {
            match print_last_campaign_report("memory") {
                Ok(report) => println!("{report}"),
                Err(err) => {
                    eprintln!("last_campaign_report_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::CampaignReport(campaign_id)) => {
            match print_campaign_report("memory", &campaign_id) {
                Ok(report) => println!("{report}"),
                Err(err) => {
                    eprintln!("campaign_report_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::AdjustTaskFromCampaign(campaign_id)) => {
            match adjust_task_from_campaign("memory", &campaign_id) {
                Ok(adjustment) => println!(
                    "{}",
                    serde_json::to_string_pretty(&adjustment).expect("serialize task adjustment")
                ),
                Err(err) => {
                    eprintln!("adjust_task_from_campaign_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::LastTaskAdjustment) => {
            match print_last_task_adjustment("memory") {
                Ok(report) => println!("{report}"),
                Err(err) => {
                    eprintln!("last_task_adjustment_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::ListAdjustedTasks) => {
            match list_adjusted_tasks("memory") {
                Ok(tasks) => println!("{}", tasks.join("\n")),
                Err(err) => {
                    eprintln!("list_adjusted_tasks_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::CampaignRecombinePreview(task_path)) => {
            match eva_runtime_with_task_validator::evolution::load_task_contract(Path::new(
                &task_path,
            ))
            .and_then(|task| preview_campaign_recombination("memory", &task))
            {
                Ok(preview) => println!(
                    "{}",
                    serde_json::to_string_pretty(&preview)
                        .expect("serialize campaign recombine preview")
                ),
                Err(err) => {
                    eprintln!("campaign_recombine_preview_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::EvolveBounded { task_path, cycles }) => {
            match run_bounded_evolution(".", "memory", &task_path, cycles) {
                Ok(summary) => println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).expect("serialize bounded summary")
                ),
                Err(err) => {
                    eprintln!("evolve_bounded_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::LastBoundedRun) => {
            match print_last_bounded_run("memory") {
                Ok(report) => println!("{report}"),
                Err(err) => {
                    eprintln!("last_bounded_run_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::BoundedRunReport(bounded_run_id)) => {
            match print_bounded_run_report("memory", &bounded_run_id) {
                Ok(report) => println!("{report}"),
                Err(err) => {
                    eprintln!("bounded_run_report_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::ListBoundedRuns) => {
            match list_bounded_runs("memory") {
                Ok(runs) => println!("{}", runs.join("\n")),
                Err(err) => {
                    eprintln!("list_bounded_runs_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::DistillPatterns) => {
            match distill_patterns("memory") {
                Ok(summary) => println!(
                    "{}",
                    serde_json::to_string_pretty(&summary).expect("serialize pattern summary")
                ),
                Err(err) => {
                    eprintln!("distill_patterns_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::RecombinePatterns) => {
            match render_recombined_hypotheses("memory") {
                Ok(output) => println!("{output}"),
                Err(err) => {
                    eprintln!("recombine_patterns_error: {err}");
                    std::process::exit(1);
                }
            }
            return;
        }
        Ok(RuntimeCliCommand::EvolveRecombined) => {
            if let Err(err) = run_recombined_evolution_cycle(".", "memory") {
                eprintln!("evolve_recombined_error: {err}");
                std::process::exit(1);
            }
            println!("evolve_recombined_status: ok");
            return;
        }
        Ok(RuntimeCliCommand::Replay(run_id)) => {
            if let Err(err) = replay_candidate(".", "memory", &run_id) {
                eprintln!("replay_error: {err}");
                std::process::exit(1);
            }
            println!("replay_status: ok");
            return;
        }
        Ok(RuntimeCliCommand::Promote(run_id)) => {
            if let Err(err) = promote_candidate(".", "memory", &run_id) {
                eprintln!("promotion_error: {err}");
                std::process::exit(1);
            }
            println!("promotion_status: ok");
            return;
        }
        Ok(RuntimeCliCommand::IngestRepo(path)) => {
            if let Err(err) = ingest_repo_patterns(&path, "memory") {
                eprintln!("ingest_repo_error: {err}");
                std::process::exit(1);
            }
            println!("ingest_repo_status: ok");
            return;
        }
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

fn resolve_corpus_alias(memory_root: &str, corpus_id: &str) -> Result<String, String> {
    if corpus_id == "latest" {
        latest_corpus_id(memory_root)
    } else {
        Ok(corpus_id.to_string())
    }
}

fn render_corpora_listing(memory_root: &str, corpora: &[String]) -> String {
    let mut lines = Vec::new();
    for corpus_id in corpora {
        if let Ok(summary) = load_corpus_summary(memory_root, corpus_id) {
            lines.push(format!(
                "{} root_path={} scanned_files={} detected_strategy_count={}",
                summary.corpus_id,
                summary.root_path,
                summary.scanned_files,
                summary.suggested_strategies.len()
            ));
        } else {
            lines.push(corpus_id.clone());
        }
    }
    lines.join("\n")
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
