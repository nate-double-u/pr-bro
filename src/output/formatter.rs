use std::io::IsTerminal;
use chrono::Duration;
use owo_colors::OwoColorize;

use crate::github::types::PullRequest;

/// Format a list of PRs as one line per PR
/// Format: "{title} | {repo} | {author} | {url}"
pub fn format_pr_list(prs: &[PullRequest], use_colors: bool) -> String {
    if prs.is_empty() {
        return "No pull requests found.".to_string();
    }

    prs.iter()
        .map(|pr| format_pr_line(pr, use_colors))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format a single PR as one line
fn format_pr_line(pr: &PullRequest, use_colors: bool) -> String {
    if use_colors {
        format!(
            "{} | {} | {} | {}",
            pr.title.bold(),
            pr.repo.cyan(),
            pr.author.yellow(),
            pr.url.underline()
        )
    } else {
        format!(
            "{} | {} | {} | {}",
            pr.title,
            pr.repo,
            pr.author,
            pr.url
        )
    }
}

/// Format a single PR with detailed multi-line output (for verbose mode)
pub fn format_pr_detail(pr: &PullRequest, use_colors: bool) -> String {
    let age = format_age(pr.age());
    let total_size = pr.size();

    if use_colors {
        format!(
            "{}\n  Repo: {}\n  Author: {}\n  Age: {}\n  Size: +{}/{} ({} lines)\n  Approvals: {}\n  URL: {}",
            pr.title.bold(),
            pr.repo.cyan(),
            pr.author.yellow(),
            age,
            pr.additions.green(),
            pr.deletions.red(),
            total_size,
            pr.approvals,
            pr.url.underline()
        )
    } else {
        format!(
            "{}\n  Repo: {}\n  Author: {}\n  Age: {}\n  Size: +{}/{} ({} lines)\n  Approvals: {}\n  URL: {}",
            pr.title,
            pr.repo,
            pr.author,
            age,
            pr.additions,
            pr.deletions,
            total_size,
            pr.approvals,
            pr.url
        )
    }
}

/// Check if stdout is a TTY (for auto-detecting color support)
pub fn should_use_colors() -> bool {
    std::io::stdout().is_terminal()
}

/// Format a duration into a human-readable age string
/// "2h" for hours, "3d" for days, "1w" for weeks
pub fn format_age(duration: Duration) -> String {
    let hours = duration.num_hours();
    let days = duration.num_days();
    let weeks = days / 7;

    if weeks >= 1 {
        format!("{}w", weeks)
    } else if days >= 1 {
        format!("{}d", days)
    } else if hours >= 1 {
        format!("{}h", hours)
    } else {
        let minutes = duration.num_minutes();
        if minutes >= 1 {
            format!("{}m", minutes)
        } else {
            "now".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_pr() -> PullRequest {
        PullRequest {
            title: "Fix login bug".to_string(),
            number: 123,
            author: "octocat".to_string(),
            repo: "owner/repo".to_string(),
            url: "https://github.com/owner/repo/pull/123".to_string(),
            created_at: Utc::now() - Duration::hours(5),
            updated_at: Utc::now() - Duration::hours(1),
            additions: 50,
            deletions: 10,
            approvals: 1,
            draft: false,
        }
    }

    #[test]
    fn test_format_pr_list_empty() {
        let prs: Vec<PullRequest> = vec![];
        let result = format_pr_list(&prs, false);
        assert_eq!(result, "No pull requests found.");
    }

    #[test]
    fn test_format_pr_list_single() {
        let prs = vec![sample_pr()];
        let result = format_pr_list(&prs, false);
        assert!(result.contains("Fix login bug"));
        assert!(result.contains("owner/repo"));
        assert!(result.contains("octocat"));
        assert!(result.contains("https://github.com/owner/repo/pull/123"));
    }

    #[test]
    fn test_format_pr_detail() {
        let pr = sample_pr();
        let result = format_pr_detail(&pr, false);
        assert!(result.contains("Fix login bug"));
        assert!(result.contains("Repo: owner/repo"));
        assert!(result.contains("Author: octocat"));
        assert!(result.contains("Size: +50/10 (60 lines)"));
        assert!(result.contains("Approvals: 1"));
    }

    #[test]
    fn test_format_age_hours() {
        let duration = Duration::hours(3);
        assert_eq!(format_age(duration), "3h");
    }

    #[test]
    fn test_format_age_days() {
        let duration = Duration::days(2);
        assert_eq!(format_age(duration), "2d");
    }

    #[test]
    fn test_format_age_weeks() {
        let duration = Duration::weeks(2);
        assert_eq!(format_age(duration), "2w");
    }

    #[test]
    fn test_format_age_minutes() {
        let duration = Duration::minutes(30);
        assert_eq!(format_age(duration), "30m");
    }

    #[test]
    fn test_format_age_now() {
        let duration = Duration::seconds(30);
        assert_eq!(format_age(duration), "now");
    }
}
