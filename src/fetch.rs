use anyhow::Result;
use crate::config::Config;
use crate::github::types::PullRequest;
use crate::scoring::{ScoringConfig, ScoreResult, calculate_score};
use crate::snooze::{SnoozeState, filter_active_prs, filter_snoozed_prs};

/// Fetch PRs from all configured queries, deduplicate, score, and split into
/// active and snoozed lists. Both lists are sorted by score descending.
///
/// This function is called from main.rs for initial load and from the TUI
/// event loop for manual/auto refresh.
pub async fn fetch_and_score_prs(
    client: &octocrab::Octocrab,
    config: &Config,
    scoring: &ScoringConfig,
    snooze_state: &SnoozeState,
    verbose: bool,
) -> Result<(Vec<(PullRequest, ScoreResult)>, Vec<(PullRequest, ScoreResult)>)> {
    // Implementation: extract the fetch-all-queries, deduplicate, score, sort
    // logic from main.rs into this function.
    // Return (active_scored_sorted, snoozed_scored_sorted).
    todo!("Extract from main.rs in Task 2")
}
