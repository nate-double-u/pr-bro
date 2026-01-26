use anyhow::{Context, Result};
use octocrab::Octocrab;

/// Create an authenticated GitHub client using a personal access token
pub fn create_client(token: &str) -> Result<Octocrab> {
    Octocrab::builder()
        .personal_token(token.to_string())
        .build()
        .context("Failed to create GitHub client")
}
