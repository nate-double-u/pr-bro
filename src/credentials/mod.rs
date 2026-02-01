pub mod prompt;

use keyring::Entry;
use std::fmt;

const SERVICE_NAME: &str = "pr-bro";
const TOKEN_KEY: &str = "github-token";

/// Environment variable name for providing a GitHub token without keyring
pub const ENV_TOKEN_VAR: &str = "PR_BRO_GH_TOKEN";

// Re-export prompt functions for convenience
pub use prompt::{prompt_for_token, reprompt_for_token, setup_token_if_missing};

/// Check for a GitHub token in the PR_BRO_GH_TOKEN environment variable.
/// Returns Some(token) if the env var is set and non-empty, None otherwise.
pub fn get_token_from_env() -> Option<String> {
    match std::env::var(ENV_TOKEN_VAR) {
        Ok(val) => {
            let trimmed = val.trim().to_string();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed)
            }
        }
        Err(_) => None,
    }
}

#[derive(Debug)]
pub enum CredentialError {
    KeyringUnavailable(String),
    TokenNotFound,
    StoreFailed(String),
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CredentialError::KeyringUnavailable(msg) => write!(f, "Keyring unavailable: {}", msg),
            CredentialError::TokenNotFound => write!(f, "Token not found in keyring"),
            CredentialError::StoreFailed(msg) => write!(f, "Failed to store token: {}", msg),
        }
    }
}

impl std::error::Error for CredentialError {}

/// Synchronous version of get_token - retrieves token from system keyring
fn get_token_sync() -> Result<String, CredentialError> {
    let entry = Entry::new(SERVICE_NAME, TOKEN_KEY)
        .map_err(|e| CredentialError::KeyringUnavailable(format!("{}", e)))?;

    entry.get_password()
        .map_err(|e| {
            match e {
                keyring::Error::NoEntry => CredentialError::TokenNotFound,
                _ => CredentialError::KeyringUnavailable(format!("{}", e)),
            }
        })
}

/// Synchronous version of store_token - stores token in system keyring
fn store_token_sync(token: &str) -> Result<(), CredentialError> {
    let entry = Entry::new(SERVICE_NAME, TOKEN_KEY)
        .map_err(|e| CredentialError::KeyringUnavailable(format!("{}", e)))?;

    entry.set_password(token)
        .map_err(|e| CredentialError::StoreFailed(format!("{}", e)))?;

    Ok(())
}

/// Async wrapper for get_token_sync - retrieves token from system keyring
/// Uses spawn_blocking to prevent blocking the async runtime
pub async fn get_token() -> Result<String, CredentialError> {
    tokio::task::spawn_blocking(|| get_token_sync())
        .await
        .map_err(|e| CredentialError::KeyringUnavailable(format!("Task join error: {}", e)))?
}

/// Async wrapper for store_token_sync - stores token in system keyring
/// Uses spawn_blocking to prevent blocking the async runtime
pub async fn store_token(token: String) -> Result<(), CredentialError> {
    tokio::task::spawn_blocking(move || store_token_sync(&token))
        .await
        .map_err(|e| CredentialError::KeyringUnavailable(format!("Task join error: {}", e)))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_keyring_roundtrip() {
        let test_token = "test_token_12345";

        // Clean up any existing token first
        let _ = tokio::task::spawn_blocking(|| {
            let entry = Entry::new(SERVICE_NAME, TOKEN_KEY).unwrap();
            let _ = entry.delete_credential();
        }).await;

        // Try to store and retrieve
        let store_result = store_token(test_token.to_string()).await;
        assert!(store_result.is_ok(), "Failed to store token: {:?}", store_result);

        let retrieved = get_token().await;
        assert!(retrieved.is_ok(), "Failed to retrieve token: {:?}", retrieved);
        assert_eq!(retrieved.unwrap(), test_token);

        // Clean up
        let _ = tokio::task::spawn_blocking(|| {
            let entry = Entry::new(SERVICE_NAME, TOKEN_KEY).unwrap();
            let _ = entry.delete_credential();
        }).await;
    }
}
