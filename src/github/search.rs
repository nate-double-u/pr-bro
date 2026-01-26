use anyhow::{Context, Result};
use octocrab::Octocrab;
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
            .context("Failed to search pull requests")
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
