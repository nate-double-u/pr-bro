use anyhow::Result;
use crate::config::Config;
use crate::github::cache::CacheConfig;
use crate::github::types::PullRequest;
use crate::scoring::{ScoringConfig, ScoreResult, calculate_score};
use crate::snooze::{SnoozeState, filter_active_prs, filter_snoozed_prs};
use futures::stream::{FuturesUnordered, StreamExt};
use std::collections::HashSet;

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
    cache_config: &CacheConfig,
    verbose: bool,
) -> Result<(Vec<(PullRequest, ScoreResult)>, Vec<(PullRequest, ScoreResult)>)> {
    if verbose {
        let cache_status = if cache_config.enabled {
            "enabled"
        } else {
            "disabled (--no-cache)"
        };
        eprintln!("Cache: {}", cache_status);
    }

    // Search PRs for each query in parallel
    let mut all_prs = Vec::new();
    let mut any_succeeded = false;

    let mut futures = FuturesUnordered::new();
    for query_config in &config.queries {
        let client = client.clone();
        let query = query_config.query.clone();
        let query_name = query_config.name.clone();
        futures.push(async move {
            let result = crate::github::search_and_enrich_prs(&client, &query).await;
            (query_name, query, result)
        });
    }

    while let Some((name, query, result)) = futures.next().await {
        match result {
            Ok(prs) => {
                if verbose {
                    eprintln!("  Found {} PRs for {}", prs.len(),
                        name.as_deref().unwrap_or(&query));
                }
                all_prs.extend(prs);
                any_succeeded = true;
            }
            Err(e) => {
                eprintln!("Query failed: {} - {}",
                    name.as_deref().unwrap_or(&query), e);
            }
        }
    }

    // If all queries failed, return error
    if !any_succeeded && !config.queries.is_empty() {
        anyhow::bail!("All queries failed. Check your network connection and GitHub token.");
    }

    // Deduplicate PRs by URL (same PR may appear in multiple queries)
    let mut seen_urls = HashSet::new();
    let unique_prs: Vec<_> = all_prs
        .into_iter()
        .filter(|pr| seen_urls.insert(pr.url.clone()))
        .collect();

    if verbose {
        eprintln!("After deduplication: {} unique PRs", unique_prs.len());
    }

    // Split into active and snoozed
    let active_prs = filter_active_prs(unique_prs.clone(), snooze_state);
    let snoozed_prs = filter_snoozed_prs(unique_prs, snooze_state);

    if verbose {
        eprintln!("After filter: {} active, {} snoozed", active_prs.len(), snoozed_prs.len());
    }

    // Score active PRs
    let mut active_scored: Vec<_> = active_prs
        .into_iter()
        .map(|pr| {
            let result = calculate_score(&pr, scoring);
            (pr, result)
        })
        .collect();

    // Score snoozed PRs
    let mut snoozed_scored: Vec<_> = snoozed_prs
        .into_iter()
        .map(|pr| {
            let result = calculate_score(&pr, scoring);
            (pr, result)
        })
        .collect();

    // Sort both lists by score descending, then by age ascending (older first for ties)
    let sort_fn = |a: &(PullRequest, ScoreResult), b: &(PullRequest, ScoreResult)| {
        // Primary: score descending
        let score_cmp = b.1.score.partial_cmp(&a.1.score).unwrap_or(std::cmp::Ordering::Equal);
        if score_cmp != std::cmp::Ordering::Equal {
            return score_cmp;
        }
        // Tie-breaker: age ascending (older first = smaller created_at)
        a.0.created_at.cmp(&b.0.created_at)
    };

    active_scored.sort_by(sort_fn);
    snoozed_scored.sort_by(sort_fn);

    Ok((active_scored, snoozed_scored))
}
