pub mod config;
pub mod engine;
pub mod factors;
pub mod validation;

pub use config::*;
pub use engine::{calculate_score, FactorContribution, ScoreBreakdown, ScoreResult};
pub use factors::{Effect, RangeOp};
pub use validation::validate_scoring;
