use anyhow::{Context, Result};
use octocrab::Octocrab;
use super::cache::{CacheConfig, DiskCache, get_cache_path};

/// Create an authenticated GitHub client using a personal access token
pub fn create_client(token: &str, cache_config: &CacheConfig) -> Result<Octocrab> {
    let mut builder = Octocrab::builder()
        .personal_token(token.to_string());

    if cache_config.enabled {
        let cache_path = get_cache_path();
        let disk_cache = DiskCache::new(cache_path);
        builder = builder.cache(disk_cache);
    }

    builder.build().context("Failed to create GitHub client")
}
