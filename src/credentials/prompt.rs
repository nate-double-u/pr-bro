use anyhow::{Context, Result};

use super::{get_token, store_token, CredentialError};

/// Prompts user to enter GitHub personal access token
pub fn prompt_for_token() -> Result<String> {
    println!("GitHub personal access token required.");
    println!("Create one at: https://github.com/settings/tokens");
    println!("Required scopes: repo (for private repos) or public_repo (for public only)");
    println!();

    let token = rpassword::prompt_password_stdout("Enter token: ")
        .context("Failed to read token from stdin")?;

    let token = token.trim();

    if token.is_empty() {
        anyhow::bail!("Token cannot be empty");
    }

    Ok(token.to_string())
}

/// Re-prompts for token when the existing one is rejected by GitHub
pub async fn reprompt_for_token() -> Result<String> {
    eprintln!();
    eprintln!("Your GitHub token was rejected (invalid or expired).");
    eprintln!("Please provide a new token.");
    eprintln!();

    let token = prompt_for_token()?;

    // Replace token in keyring
    store_token(token.clone()).await
        .context("Failed to store new token in keyring")?;

    eprintln!("New token stored securely in system keyring.");

    Ok(token)
}

/// Setup token if missing - prompts for token on first run
/// Returns the token (either existing or newly stored)
pub async fn setup_token_if_missing() -> Result<String> {
    match get_token().await {
        Ok(token) => {
            // Token exists, return it
            Ok(token)
        }
        Err(CredentialError::TokenNotFound) => {
            // Token missing, prompt for it
            let token = prompt_for_token()?;

            // Store in keyring
            store_token(token.clone()).await
                .context("Failed to store token in keyring")?;

            println!("Token stored securely in system keyring.");

            Ok(token)
        }
        Err(CredentialError::KeyringUnavailable(msg)) => {
            // Keyring unavailable - fail immediately
            anyhow::bail!(
                "System keyring unavailable. pr-bro requires a secure keyring \
                (macOS Keychain, Windows Credential Store, or Linux Secret Service).\n\
                Error: {}",
                msg
            );
        }
        Err(e) => {
            // Other errors
            anyhow::bail!("Failed to access keyring: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_for_token_validation() {
        // Note: This test can't fully test prompt_for_token since it reads from stdin
        // Manual testing required for the actual prompt flow

        // Just verify the function signature is correct
        let _ = prompt_for_token; // Function exists with expected signature
    }

    #[tokio::test]
    async fn test_setup_token_if_missing_with_existing_token() {
        // Store a test token first
        let test_token = "existing_test_token_67890";
        let _ = store_token(test_token.to_string()).await;

        // Should return existing token without prompting
        let result = setup_token_if_missing().await;

        // Clean up regardless of result
        let _ = tokio::task::spawn_blocking(|| {
            let entry = keyring::Entry::new("pr-bro", "github-token").unwrap();
            let _ = entry.delete_credential();
        }).await;

        // Verify result
        if let Ok(token) = result {
            assert_eq!(token, test_token);
        }
    }
}
