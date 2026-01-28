use serde::{Deserialize, Serialize};

use crate::scoring::ScoringConfig;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    /// Global scoring configuration (applies to all queries unless overridden)
    #[serde(default)]
    pub scoring: Option<ScoringConfig>,

    pub queries: Vec<QueryConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QueryConfig {
    pub name: Option<String>,
    pub query: String,

    /// Per-query scoring configuration (overrides global scoring)
    #[serde(default)]
    pub scoring: Option<ScoringConfig>,
}
