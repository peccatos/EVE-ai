use crate::contracts::{CommandResult, MutationContract, MutationObjective, SandboxResult};
use crate::evolution::{generator, memory, metrics, mutator, scorer, validator};
use crate::sandbox::{manager, runner, snapshot};

#[derive(Debug, Clone)]
struct PlanContext {
    plan_id: Option<String>,
    hypothesis_id: Option<String>,
    objective: Option<MutationObjective>,
    graph_evidence: Vec<String>,
}

pub fn run_evolution_cycle(project_root: &str) -> Result<(), String> {
    run_evolution_cycle_with_memory(project_root, "memory")
}

pub fn run_evolution_cycle_with_memory(
    project_root: &str,
    memory_root: &str,
) -> Result<(), String> {
    run_evolution_cycle_with_mutation(
        project_root,
        memory_root,
        generator::generate_safe_mutation(),
        None,
    )
}

pub fn run_planned_evolution_cycle(project_root: &str, memory_root: &str) -> Result<(), String> {
    let plans = crate::graph::analyzer::propose_mutation_plans(memory_root)?;
    let hypotheses = crate::evolution::rank_plans(&plans);
    let Some(hypothesis) = hypotheses.first() else {
        return Err("no graph-guided plans available".to_string());
    };
    let plan = plans
        .iter()
        .find(|plan| plan.id == hypothesis.plan_id)
        .ok_or_else(|| "ranked hypothesis points to missing plan".to_string())?;
    let mutation = generator::generate_from_plan(plan);
    validator::validate_mutation(&mutation)?;
    run_evolution_cycle_with_mutation(
        project_root,
        memory_root,
        mutation,
        Some(PlanContext {
            plan_id: Some(plan.id.clone()),
            hypothesis_id: Some(hypothesis.id.clone()),
            objective: Some(plan.objective),
            graph_evidence: plan.graph_evidence.clone(),
        }),
    )
}

fn run_evolution_cycle_with_mutation(
    project_root: &str,
    memory_root: &str,
    mutation: MutationContract,
    plan_context: Option<PlanContext>,
) -> Result<(), String> {
    let run_id = memory::new_run_id();
    let sandbox_path = manager::create_sandbox_path();
    snapshot::copy_project(project_root, &sandbox_path)?;

    let result = run_cycle_in_sandbox(&sandbox_path, mutation);
    let cleanup = manager::destroy_sandbox(&sandbox_path);
    if let Ok((mutation, score, sandbox)) = &result {
        let entry = if let Some(context) = &plan_context {
            memory::build_log_entry_with_plan(
                run_id,
                context.plan_id.clone(),
                context.hypothesis_id.clone(),
                context.objective.map(|objective| format!("{objective:?}")),
                context.graph_evidence.clone(),
                mutation,
                score,
                sandbox,
                false,
                cleanup.is_ok(),
            )
        } else {
            memory::build_log_entry(run_id, mutation, score, sandbox, false, cleanup.is_ok())
        };
        memory::append_jsonl(
            std::path::Path::new(memory_root).join("evolution.jsonl"),
            &entry,
        )?;
        memory::maybe_store_candidate(memory_root, &entry, mutation)?;
        crate::graph::update_graph_for_evolution(memory_root, &entry)?;
        metrics::update_metrics_after_log(memory_root, &entry)?;
    }

    match (result, cleanup) {
        (Ok((_, score, _)), Ok(())) if score.accepted => Ok(()),
        (Ok((_, score, sandbox)), Ok(())) => Err(format!(
            "evolution validation failed: check={} test={} run={}",
            sandbox.check.success, score.test_passed, score.run_passed
        )),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(cleanup_error)) => Err(cleanup_error),
        (Err(error), Err(cleanup_error)) => {
            Err(format!("{error}; cleanup failed: {cleanup_error}"))
        }
    }
}

fn run_cycle_in_sandbox(
    sandbox_path: &str,
    mutation: MutationContract,
) -> Result<
    (
        MutationContract,
        crate::evolution::EvolutionScore,
        SandboxResult,
    ),
    String,
> {
    validator::validate_mutation(&mutation)?;
    mutator::apply_mutation(sandbox_path, &mutation)?;

    let check = runner::run_cargo_check(sandbox_path);
    let test = if check.success {
        runner::run_cargo_test(sandbox_path)
    } else {
        failed_command("cargo test skipped because cargo check failed")
    };
    let run = if test.success {
        Some(runner::run_cargo_run(sandbox_path))
    } else {
        None
    };

    let score = scorer::score_cycle(&check, &test, run.as_ref());
    let sandbox = SandboxResult {
        sandbox_path: sandbox_path.to_string(),
        check,
        test: Some(test),
        run,
    };
    Ok((mutation, score, sandbox))
}

fn failed_command(stderr: &str) -> CommandResult {
    CommandResult {
        success: false,
        stdout: String::new(),
        stderr: stderr.to_string(),
        duration_ms: 0,
    }
}
