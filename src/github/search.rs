use anyhow::{anyhow, Context, Result};
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

/// Search and enrich PRs with full details
pub async fn search_and_enrich_prs(client: &Octocrab, query: &str) -> Result<Vec<PullRequest>> {
    let mut prs = search_prs(client, query).await?;

    // Enrich each PR with details
    // If we hit rate limits, we'll stop enriching but keep the PRs we have
    for pr in &mut prs {
        match enrich_pr(client, pr).await {
            Ok(_) => {}
            Err(e) => {
                // Check if it's a rate limit error
                if e.to_string().contains("rate limit") || e.to_string().contains("403") {
                    eprintln!("Warning: Rate limit hit during enrichment. Returning partial results.");
                    break;
                }
                // For other errors, continue trying the rest
                eprintln!("Warning: Failed to enrich PR {}: {}", pr.number, e);
            }
        }
    }

    Ok(prs)
}
