pub mod config;
pub mod factors;
pub mod engine;
pub mod validation;

pub use config::*;
pub use factors::{RangeOp, Effect};
pub use engine::{calculate_score, ScoreResult};
pub use validation::validate_scoring;
