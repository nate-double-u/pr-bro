use clap::{Parser, Subcommand};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Instant;

// Exit codes per CONTEXT.md
const EXIT_SUCCESS: i32 = 0;
const EXIT_AUTH: i32 = 1;
const EXIT_NETWORK: i32 = 2;
#[allow(dead_code)]
const EXIT_RATE_LIMIT: i32 = 3;
const EXIT_CONFIG: i32 = 4;

#[derive(Subcommand, Debug)]
enum Commands {
    /// List PRs sorted by priority (default if no subcommand)
    List {
        /// Show snoozed PRs instead of active PRs
        #[arg(long)]
        show_snoozed: bool,
    },
    /// Open a PR in browser by its index number
    Open {
        /// Index number of the PR to open (1-based, as shown in list)
        index: usize,
    },
    /// Snooze a PR by its index number
    Snooze {
        /// Index number of the PR to snooze (1-based, as shown in list)
        index: usize,
        /// Duration to snooze (e.g., "2h", "3d", "1w"). Omit for indefinite.
        #[arg(long, value_name = "DURATION")]
        r#for: Option<String>,
    },
    /// Unsnooze a PR by its index in the snoozed list
    Unsnooze {
        /// Index number of the snoozed PR to unsnooze (1-based, as shown in --show-snoozed list)
        index: usize,
    },
}

#[derive(Parser, Debug)]
#[command(name = "pr-bro")]
#[command(about = "GitHub PR review prioritization CLI", long_about = None)]
#[command(version)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Path to config file (defaults to ~/.config/pr-bro/config.yaml)
    #[arg(short, long, global = true)]
    config: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[tokio::main]
async fn main() {
    // Install rustls crypto provider (required for rustls 0.23+)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Commands::List { show_snoozed: false });
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

    // Validate scoring config at startup
    let effective_scoring = config.scoring.clone().unwrap_or_default();
    if let Err(errors) = pr_bro::scoring::validate_scoring(&effective_scoring) {
        eprintln!("Scoring config errors:");
        for error in errors {
            eprintln!("  - {}", error);
        }
        std::process::exit(EXIT_CONFIG);
    }

    // Load snooze state (before credential setup - no network required)
    let snooze_path = pr_bro::snooze::get_snooze_path();
    let mut snooze_state = match pr_bro::snooze::load_snooze_state(&snooze_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Warning: Could not load snooze state: {}", e);
            pr_bro::snooze::SnoozeState::new()
        }
    };
    // Clean expired snoozes on load
    snooze_state.clean_expired();

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

    // Deduplicate PRs by URL (same PR may appear in multiple queries)
    let mut seen_urls = HashSet::new();
    let unique_prs: Vec<_> = all_prs
        .into_iter()
        .filter(|pr| seen_urls.insert(pr.url.clone()))
        .collect();

    if cli.verbose {
        let deduped_count = unique_prs.len();
        eprintln!("After deduplication: {} unique PRs", deduped_count);
    }

    // Apply snooze filtering based on command type
    let use_snoozed_view = matches!(&command, Commands::List { show_snoozed: true } | Commands::Unsnooze { .. });
    let filtered_prs = if use_snoozed_view {
        pr_bro::snooze::filter_snoozed_prs(unique_prs, &snooze_state)
    } else {
        pr_bro::snooze::filter_active_prs(unique_prs, &snooze_state)
    };

    if cli.verbose {
        let filter_type = if use_snoozed_view { "snoozed" } else { "active" };
        eprintln!("After {} filter: {} PRs", filter_type, filtered_prs.len());
    }

    // Calculate scores for all PRs
    let mut scored_prs: Vec<_> = filtered_prs
        .into_iter()
        .map(|pr| {
            let result = pr_bro::scoring::calculate_score(&pr, &effective_scoring);
            (pr, result)
        })
        .collect();

    // Sort by score descending, then by age ascending (older first for ties)
    scored_prs.sort_by(|a, b| {
        // Primary: score descending
        let score_cmp = b.1.score.partial_cmp(&a.1.score).unwrap_or(std::cmp::Ordering::Equal);
        if score_cmp != std::cmp::Ordering::Equal {
            return score_cmp;
        }
        // Tie-breaker: age ascending (older first = smaller created_at)
        a.0.created_at.cmp(&b.0.created_at)
    });

    // Route based on subcommand
    match command {
        Commands::List { show_snoozed: _ } => {
            // Build ScoredPr references for formatter
            let scored_refs: Vec<pr_bro::output::ScoredPr> = scored_prs
                .iter()
                .map(|(pr, result)| pr_bro::output::ScoredPr {
                    pr,
                    score: result.score,
                    incomplete: result.incomplete,
                })
                .collect();

            // Output results
            let use_colors = pr_bro::output::should_use_colors();

            if cli.verbose && !scored_refs.is_empty() {
                // Verbose mode: detailed output with scores
                for scored in &scored_refs {
                    println!(
                        "{}",
                        pr_bro::output::format_pr_detail(scored.pr, use_colors)
                    );
                    println!(
                        "  Score: {}",
                        pr_bro::output::format_score(scored.score, scored.incomplete)
                    );
                    println!();
                }
            } else {
                // Normal mode: scored table
                let output = pr_bro::output::format_scored_table(&scored_refs, use_colors);
                println!("{}", output);
            }

            if cli.verbose {
                eprintln!();
                eprintln!("Total: {} PRs in {:?}", scored_prs.len(), start_time.elapsed());
            }
        }
        Commands::Open { index } => {
            // Handle empty result case
            if scored_prs.is_empty() {
                eprintln!("No pull requests found. Nothing to open.");
                std::process::exit(EXIT_SUCCESS);
            }

            // Validate index bounds (1-based)
            if index < 1 || index > scored_prs.len() {
                eprintln!(
                    "Invalid index {}. Must be between 1 and {}.",
                    index,
                    scored_prs.len()
                );
                std::process::exit(EXIT_CONFIG);
            }

            // Get PR at index (convert to 0-based)
            let (pr, _result) = &scored_prs[index - 1];

            // Open in browser
            if let Err(e) = pr_bro::browser::open_url(&pr.url) {
                eprintln!("Failed to open browser: {}", e);
                std::process::exit(EXIT_NETWORK);
            }

            println!("Opening PR #{} in browser: {}", pr.number, pr.url);
        }
        Commands::Snooze { index, r#for: duration } => {
            if scored_prs.is_empty() {
                eprintln!("No pull requests found. Nothing to snooze.");
                std::process::exit(EXIT_SUCCESS);
            }
            if index < 1 || index > scored_prs.len() {
                eprintln!("Invalid index {}. Must be between 1 and {}.", index, scored_prs.len());
                std::process::exit(EXIT_CONFIG);
            }

            let (pr, _) = &scored_prs[index - 1];
            let snooze_until = if let Some(dur_str) = duration {
                let std_duration = humantime::parse_duration(&dur_str)
                    .unwrap_or_else(|_| {
                        eprintln!("Invalid duration '{}'. Use formats like: 2h, 3d, 1w", dur_str);
                        std::process::exit(EXIT_CONFIG);
                    });
                let chrono_duration = chrono::Duration::from_std(std_duration)
                    .unwrap_or_else(|_| {
                        eprintln!("Duration '{}' is too large.", dur_str);
                        std::process::exit(EXIT_CONFIG);
                    });
                Some(chrono::Utc::now() + chrono_duration)
            } else {
                None
            };

            snooze_state.snooze(pr.url.clone(), snooze_until);
            if let Err(e) = pr_bro::snooze::save_snooze_state(&snooze_path, &snooze_state) {
                eprintln!("Failed to save snooze state: {}", e);
                std::process::exit(EXIT_CONFIG);
            }

            let duration_msg = match snooze_until {
                Some(until) => format!(" until {}", until.format("%Y-%m-%d %H:%M UTC")),
                None => " indefinitely".to_string(),
            };
            println!("Snoozed PR #{}{}: {}", pr.number, duration_msg, pr.title);
        }
        Commands::Unsnooze { index } => {
            if scored_prs.is_empty() {
                eprintln!("No snoozed pull requests found. Nothing to unsnooze.");
                std::process::exit(EXIT_SUCCESS);
            }
            if index < 1 || index > scored_prs.len() {
                eprintln!("Invalid index {}. Must be between 1 and {}.", index, scored_prs.len());
                std::process::exit(EXIT_CONFIG);
            }

            let (pr, _) = &scored_prs[index - 1];
            let removed = snooze_state.unsnooze(&pr.url);
            if removed {
                if let Err(e) = pr_bro::snooze::save_snooze_state(&snooze_path, &snooze_state) {
                    eprintln!("Failed to save snooze state: {}", e);
                    std::process::exit(EXIT_CONFIG);
                }
                println!("Unsnoozed PR #{}: {}", pr.number, pr.title);
            } else {
                eprintln!("PR #{} was not snoozed.", pr.number);
            }
        }
    }

    std::process::exit(EXIT_SUCCESS);
}
