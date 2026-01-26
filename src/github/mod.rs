pub mod client;
pub mod search;
pub mod types;

pub use client::create_client;
pub use search::{search_and_enrich_prs, search_prs};
pub use types::PullRequest;
