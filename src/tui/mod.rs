pub mod app;
pub use app::App;

use crate::scoring::ScoringConfig;

/// Stub -- replaced by real implementation in Plan 05-02.
pub async fn run_tui(
    _app: App,
    _client: octocrab::Octocrab,
    _scoring_config: ScoringConfig,
) -> anyhow::Result<()> {
    Ok(())
}
