use crate::evolution::task_strategy_memory::{
    build_task_strategy_memory, classify_goal, recommended_strategy,
};

pub fn select_strategy(memory_root: &str, goal: &str) -> Result<(String, f32, String), String> {
    let kind = classify_goal(goal);
    if kind == "unsafe" {
        return Ok((
            "refuse_unsafe".to_string(),
            1.0,
            "goal mentions forbidden path or unsafe operation".to_string(),
        ));
    }
    let memory = build_task_strategy_memory(memory_root)?;
    if let Some(pattern) = memory
        .strategies
        .iter()
        .find(|pattern| pattern.task_kind == kind)
    {
        return Ok((
            pattern.recommended_strategy.clone(),
            pattern.confidence,
            pattern.reason.clone(),
        ));
    }
    Ok((
        recommended_strategy(&kind, 0, 1),
        0.25,
        "no direct outcome history; conservative fallback".to_string(),
    ))
}

pub fn print_strategy_select(memory_root: &str, goal: &str) -> Result<String, String> {
    let (strategy, confidence, reason) = select_strategy(memory_root, goal)?;
    Ok(format!(
        "EVA Strategy Selection\ngoal={}\nrecommended_strategy={}\nconfidence={:.2}\nreason={}",
        goal, strategy, confidence, reason
    ))
}
