use anyhow::{anyhow, Context, Result};
use futures::stream::{FuturesUnordered, StreamExt};
use octocrab::Octocrab;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_retry::{strategy::ExponentialBackoff, Retry};

use crate::github::types::PullRequest;

/// Search GitHub for pull requests matching the given query
pub async fn search_prs(client: &Octocrab, query: &str) -> Result<Vec<PullRequest>> {
    // Retry strategy: exponential backoff with 3 attempts
    let retry_strategy = ExponentialBackoff::from_millis(100)
        .max_delay(std::time::Duration::from_secs(5))
        .take(3);

    let results = Retry::spawn(retry_strategy, || async {
        client
            .search()
            .issues_and_pull_requests(query)
            .send()
            .await
            .map_err(|e| {
                // Extract useful error info from octocrab error
                let error_str = format!("{:?}", e);
                if error_str.contains("do not have permission") || error_str.contains("resources do not exist") {
                    anyhow!("Repository not found or no access. Check repo name and token permissions (needs 'repo' scope for private repos).")
                } else if error_str.contains("401") || error_str.contains("Bad credentials") {
                    anyhow!("Authentication failed. Your GitHub token may be invalid or expired.")
                } else if error_str.contains("rate limit") || error_str.contains("403") {
                    anyhow!("GitHub API rate limit exceeded. Wait a few minutes and try again.")
                } else {
                    anyhow!("GitHub API error: {}", e)
                }
            })
    })
    .await?;

    let prs: Vec<PullRequest> = results
        .items
        .into_iter()
        .filter(|issue| issue.pull_request.is_some()) // Only PRs, not issues
        .filter_map(|issue| {
            // Extract owner/repo from html_url
            // Format: "https://github.com/owner/repo/pull/123"
            let path = issue.html_url.path();
            let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            let repo = if parts.len() >= 2 {
                format!("{}/{}", parts[0], parts[1])
            } else {
                "unknown/unknown".to_string()
            };

            Some(PullRequest {
                title: issue.title,
                number: issue.number,
                author: issue.user.login.clone(),
                repo,
                url: issue.html_url.to_string(),
                created_at: issue.created_at,
                updated_at: issue.updated_at,
                additions: 0,  // Search API doesn't include these
                deletions: 0,  // Will be populated by enrichment
                approvals: 0,  // Requires separate API call
                draft: false,  // Search API doesn't expose draft status reliably
            })
        })
        .collect();

    Ok(prs)
}

/// Fetch PR details (additions, deletions) from the GitHub API
async fn fetch_pr_details(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<(u64, u64)> {
    let pr = client
        .pulls(owner, repo)
        .get(number)
        .await
        .context("Failed to fetch PR details")?;

    let additions = pr.additions.unwrap_or(0) as u64;
    let deletions = pr.deletions.unwrap_or(0) as u64;

    Ok((additions, deletions))
}

/// Fetch PR review count (approved reviews)
async fn fetch_pr_reviews(
    client: &Octocrab,
    owner: &str,
    repo: &str,
    number: u64,
) -> Result<u32> {
    let reviews = client
        .pulls(owner, repo)
        .list_reviews(number)
        .send()
        .await
        .context("Failed to fetch PR reviews")?;

    let approved_count = reviews
        .items
        .iter()
        .filter(|review| {
            matches!(review.state, Some(octocrab::models::pulls::ReviewState::Approved))
        })
        .count() as u32;

    Ok(approved_count)
}

/// Enrich a PR with detailed information (size and approvals)
async fn enrich_pr(client: &Octocrab, pr: &mut PullRequest) -> Result<()> {
    // Parse owner/repo from pr.repo field
    let parts: Vec<&str> = pr.repo.split('/').collect();
    if parts.len() != 2 {
        return Err(anyhow!("Invalid repo format: {}", pr.repo));
    }
    let owner = parts[0];
    let repo_name = parts[1];

    // Fetch details and reviews in parallel
    let details_fut = fetch_pr_details(client, owner, repo_name, pr.number);
    let reviews_fut = fetch_pr_reviews(client, owner, repo_name, pr.number);

    match tokio::try_join!(details_fut, reviews_fut) {
        Ok(((additions, deletions), approvals)) => {
            pr.additions = additions;
            pr.deletions = deletions;
            pr.approvals = approvals;
            Ok(())
        }
        Err(e) => {
            // If enrichment fails, log but don't fail the whole operation
            eprintln!("Warning: Failed to enrich PR {}: {}", pr.number, e);
            Ok(())
        }
    }
}

/// Helper function for concurrent PR enrichment
async fn enrich_pr_with_rate_limit_check(
    client: Octocrab,
    mut pr: PullRequest,
    rate_limited: Arc<AtomicBool>,
) -> PullRequest {
    if rate_limited.load(Ordering::Relaxed) {
        return pr; // Skip enrichment if rate limited
    }

    match enrich_pr(&client, &mut pr).await {
        Ok(_) => {}
        Err(e) => {
            let err_str = e.to_string();
            if err_str.contains("rate limit") || err_str.contains("403") {
                eprintln!("Warning: Rate limit hit during enrichment. Returning partial results.");
                rate_limited.store(true, Ordering::Relaxed);
            } else {
                eprintln!("Warning: Failed to enrich PR {}: {}", pr.number, e);
            }
        }
    }
    pr
}

/// Search and enrich PRs with full details
pub async fn search_and_enrich_prs(client: &Octocrab, query: &str) -> Result<Vec<PullRequest>> {
    let prs = search_prs(client, query).await?;

    // Enrich PRs with bounded concurrency
    const MAX_CONCURRENT_ENRICHMENTS: usize = 10;

    // Rate limit flag shared across concurrent tasks
    let rate_limited = Arc::new(AtomicBool::new(false));

    let mut futures = FuturesUnordered::new();
    let mut prs_iter = prs.into_iter();
    let mut enriched_prs = Vec::new();

    // Fill initial batch
    for _ in 0..MAX_CONCURRENT_ENRICHMENTS {
        if let Some(pr) = prs_iter.next() {
            futures.push(enrich_pr_with_rate_limit_check(
                client.clone(),
                pr,
                rate_limited.clone(),
            ));
        }
    }

    // Process results and feed new tasks
    while let Some(pr) = futures.next().await {
        enriched_prs.push(pr);

        // Add next PR if not rate limited
        if !rate_limited.load(Ordering::Relaxed) {
            if let Some(next_pr) = prs_iter.next() {
                futures.push(enrich_pr_with_rate_limit_check(
                    client.clone(),
                    next_pr,
                    rate_limited.clone(),
                ));
            }
        }
    }

    // Add any remaining unenriched PRs (if rate limited, remaining weren't submitted)
    enriched_prs.extend(prs_iter);

    Ok(enriched_prs)
}
