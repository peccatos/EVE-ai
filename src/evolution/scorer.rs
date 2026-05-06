use serde::{Deserialize, Serialize};

use crate::contracts::sandbox_result::CommandResult;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvolutionScore {
    pub accepted: bool,
    pub score: f32,
    pub check_passed: bool,
    pub test_passed: bool,
    pub run_passed: bool,
    pub total_duration_ms: u128,
}

pub fn score_cycle(
    check: &CommandResult,
    test: &CommandResult,
    run: Option<&CommandResult>,
) -> EvolutionScore {
    let run_passed = run.map(|result| result.success).unwrap_or(false);
    let total_duration_ms =
        check.duration_ms + test.duration_ms + run.map(|result| result.duration_ms).unwrap_or(0);
    let mut score = 0.0;
    if check.success {
        score += 3.0;
    }
    if test.success {
        score += 4.0;
    }
    if run_passed {
        score += 3.0;
    }

    EvolutionScore {
        accepted: check.success && test.success,
        score,
        check_passed: check.success,
        test_passed: test.success,
        run_passed,
        total_duration_ms,
    }
}
