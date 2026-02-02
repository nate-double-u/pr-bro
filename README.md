# pr-bro

GitHub PR review prioritization CLI/TUI

**Know which PR to review next.** pr-bro helps developers prioritize pull request reviews based on weighted scoring across multiple GitHub queries. Features an interactive TUI with keyboard navigation, smart caching, and flexible configuration.

## Features

- Weighted scoring based on age, approvals, PR size, labels, and review history
- Interactive TUI with keyboard navigation and real-time updates
- Multiple query support with per-query scoring overrides
- Snooze PRs (timed or indefinite) to hide them temporarily
- Score breakdown detail view (press `d` to see how a PR's score was calculated)
- ETag-based HTTP caching for rate limit conservation
- Token authentication via `PR_BRO_GH_TOKEN` environment variable

## Installation

### From Source

Requires Rust toolchain (1.70+).

```bash
cargo install --path .
```

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs)

## Quick Start

Set `PR_BRO_GH_TOKEN` to your GitHub Personal Access Token, then run pr-bro:

```bash
PR_BRO_GH_TOKEN="ghp_your_token_here" pr-bro
```

### Minimal Configuration

Create `~/.config/pr-bro/config.yaml`:

```yaml
queries:
  - name: my-reviews
    query: "is:pr review-requested:@me"
```

### Running

```bash
# Launch interactive TUI (default)
pr-bro

# Plain text table output (non-interactive)
pr-bro list

# Show snoozed PRs
pr-bro list --show-snoozed
```

## Configuration

Configuration file location: `~/.config/pr-bro/config.yaml`

At minimum, define one or more queries. Scoring is optional and customizable:

```yaml
scoring:
  base_score: 100
  age: "+1 per 1h"
  size:
    buckets:
      - range: "<100"
        effect: "x5"
      - range: "100-500"
        effect: "x1"
      - range: ">500"
        effect: "x0.5"

queries:
  - name: my-reviews
    query: "is:pr review-requested:@me"
```

Queries can include their own `scoring` block to override specific fields from the global config.

For the full configuration reference including all scoring factors, per-query overrides, effect syntax, and validation details, see [Configuration Reference](docs/configuration.md).

Run `pr-bro --help` to see all available commands and flags.

In the TUI, press `?` to see all keyboard shortcuts.

## Authentication

Set the `PR_BRO_GH_TOKEN` environment variable with a GitHub Personal Access Token.

**Required scopes:**
- `repo` (for private repositories)
- `public_repo` (for public repositories only)

Create a token at: https://github.com/settings/tokens

If the environment variable is not set, pr-bro will prompt for a token interactively (valid for the current session only).

## Caching

pr-bro caches GitHub API responses using ETags to conserve rate limits. Use `--no-cache` to skip caching or `--clear-cache` to remove cached data.

For details on cache location and behavior, see [Caching](docs/caching.md).

## License

MIT (placeholder - add LICENSE file for actual license)
