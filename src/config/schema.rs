use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub queries: Vec<QueryConfig>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct QueryConfig {
    pub name: Option<String>,
    pub query: String,
    // scoring field reserved for Phase 2
}
