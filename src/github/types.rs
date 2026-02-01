use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct PullRequest {
    pub title: String,
    pub number: u64,
    pub author: String,
    pub repo: String,           // "owner/repo" format
    pub url: String,            // HTML URL for browser
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub additions: u64,         // Lines added
    pub deletions: u64,         // Lines deleted
    pub approvals: u32,         // Approval count (will need separate API call)
    pub draft: bool,
}

impl PullRequest {
    /// Calculate PR age from creation time
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }

    /// Calculate total size (additions + deletions)
    pub fn size(&self) -> u64 {
        self.additions + self.deletions
    }

    /// Return a short reference in the format "owner/repo#123"
    pub fn short_ref(&self) -> String {
        format!("{}#{}", self.repo, self.number)
    }
}
