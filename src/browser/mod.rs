use anyhow::{Context, Result};

/// Open a URL in the user's default browser
///
/// # Arguments
/// * `url` - The URL to open (e.g., GitHub PR URL)
///
/// # Errors
/// Returns error if browser cannot be opened (e.g., no browser available)
pub fn open_url(url: &str) -> Result<()> {
    webbrowser::open(url)
        .with_context(|| format!("Failed to open browser for URL: {}", url))?;
    Ok(())
}
