use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "pr-bro")]
#[command(about = "GitHub PR review prioritization CLI", long_about = None)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Path to config file (defaults to ~/.config/pr-bro/config.yaml)
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    println!("pr-bro starting...");

    // Load config to verify it works
    let config_path = cli.config.map(PathBuf::from);
    let config = pr_bro::config::load_config(config_path)?;

    if cli.verbose {
        println!("Loaded {} queries from config", config.queries.len());
        for (i, query) in config.queries.iter().enumerate() {
            println!("  Query {}: {}", i + 1, query.name.as_deref().unwrap_or("(unnamed)"));
        }
    }

    Ok(())
}
