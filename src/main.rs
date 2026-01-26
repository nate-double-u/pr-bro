use clap::Parser;
use std::path::PathBuf;
use std::time::Instant;

// Exit codes per CONTEXT.md
const EXIT_SUCCESS: i32 = 0;
const EXIT_AUTH: i32 = 1;
const EXIT_NETWORK: i32 = 2;
#[allow(dead_code)]
const EXIT_RATE_LIMIT: i32 = 3;
const EXIT_CONFIG: i32 = 4;

#[derive(Parser, Debug)]
#[command(name = "pr-bro")]
#[command(about = "GitHub PR review prioritization CLI", long_about = None)]
#[command(version)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Path to config file (defaults to ~/.config/pr-bro/config.yaml)
    #[arg(short, long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let start_time = Instant::now();

    // Load config
    let config_path = cli.config.map(PathBuf::from);
    let config = match pr_bro::config::load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}", e);
            std::process::exit(EXIT_CONFIG);
        }
    };

    if cli.verbose {
        eprintln!("Loaded {} queries from config", config.queries.len());
        for (i, query) in config.queries.iter().enumerate() {
            eprintln!(
                "  Query {}: {} ({})",
                i + 1,
                query.name.as_deref().unwrap_or("(unnamed)"),
                query.query
            );
        }
    }

    // Check if any queries are configured
    if config.queries.is_empty() {
        eprintln!("No queries configured in config file.");
        eprintln!("Add queries to ~/.config/pr-bro/config.yaml:");
        eprintln!("  queries:");
        eprintln!("    - name: my-reviews");
        eprintln!("      query: \"is:pr review-requested:@me\"");
        std::process::exit(EXIT_CONFIG);
    }

    // Setup credentials (prompts for token on first run)
    let token = match pr_bro::credentials::setup_token_if_missing().await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Credential error: {}", e);
            std::process::exit(EXIT_AUTH);
        }
    };

    if cli.verbose {
        eprintln!("Token retrieved from keyring");
    }

    // Create GitHub client
    let client = match pr_bro::github::create_client(&token) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create GitHub client: {}", e);
            std::process::exit(EXIT_NETWORK);
        }
    };

    // Search PRs for each query
    let mut all_prs = Vec::new();
    let mut any_succeeded = false;

    for query_config in &config.queries {
        if cli.verbose {
            let query_start = Instant::now();
            eprintln!("Searching: {}", query_config.query);

            match pr_bro::github::search_and_enrich_prs(&client, &query_config.query).await {
                Ok(prs) => {
                    eprintln!(
                        "  Found {} PRs in {:?}",
                        prs.len(),
                        query_start.elapsed()
                    );
                    all_prs.extend(prs);
                    any_succeeded = true;
                }
                Err(e) => {
                    eprintln!("  Query failed: {}", e);
                    // Continue with other queries (partial failure per CONTEXT.md)
                }
            }
        } else {
            match pr_bro::github::search_and_enrich_prs(&client, &query_config.query).await {
                Ok(prs) => {
                    all_prs.extend(prs);
                    any_succeeded = true;
                }
                Err(e) => {
                    eprintln!("Query failed: {} - {}", query_config.query, e);
                    // Continue with other queries
                }
            }
        }
    }

    // If all queries failed, exit with network error
    if !any_succeeded && !config.queries.is_empty() {
        eprintln!("All queries failed. Check your network connection and GitHub token.");
        std::process::exit(EXIT_NETWORK);
    }

    // Output results
    let use_colors = pr_bro::output::should_use_colors();

    if cli.verbose && !all_prs.is_empty() {
        // Verbose mode: detailed output
        for pr in &all_prs {
            println!("{}", pr_bro::output::format_pr_detail(pr, use_colors));
            println!();
        }
    } else {
        // Normal mode: one line per PR
        let output = pr_bro::output::format_pr_list(&all_prs, use_colors);
        println!("{}", output);
    }

    if cli.verbose {
        eprintln!();
        eprintln!("Total: {} PRs in {:?}", all_prs.len(), start_time.elapsed());
    }

    std::process::exit(EXIT_SUCCESS);
}
