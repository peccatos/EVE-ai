pub mod generator;
pub mod hypothesis;
pub mod memory;
pub mod metrics;
pub mod mutator;
pub mod rollback;
pub mod scorer;
pub mod validator;

pub use generator::{generate_from_plan, generate_safe_mutation};
pub use hypothesis::{rank_plans, EvolutionHypothesis};
pub use memory::{record_evolution, CandidateSummary, ReplayResult};
pub use metrics::{load_metrics, update_metrics_after_log, EvolutionMetrics};
pub use mutator::apply_mutation;
pub use rollback::rollback_sandbox;
pub use scorer::{score_cycle, EvolutionScore};
pub use validator::validate_mutation;
