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

Set your GitHub Personal Access Token as an environment variable, then run pr-bro:

```bash
export PR_BRO_GH_TOKEN="ghp_your_token_here"
pr-bro
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

## Commands

### Default (TUI)

```bash
pr-bro
```

Launches interactive TUI when running in a terminal. Shows active PRs by default.

### List

```bash
# Plain text table output
pr-bro list

# Show snoozed PRs instead of active
pr-bro list --show-snoozed
```

### Open

```bash
pr-bro open <INDEX>
```

Open a PR in your browser by its index number (1-based, as shown in list).

### Snooze

```bash
# Snooze indefinitely
pr-bro snooze <INDEX>

# Snooze for a duration
pr-bro snooze <INDEX> --for 2h
pr-bro snooze <INDEX> --for 3d
pr-bro snooze <INDEX> --for 1w
```

Duration format uses humantime: `2h` (2 hours), `3d` (3 days), `1w` (1 week), `30m` (30 minutes).

### Unsnooze

```bash
pr-bro unsnooze <INDEX>
```

Unsnooze a PR by its index in the snoozed list (use with `list --show-snoozed`).

### Global Flags

Available with all commands:

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Enable verbose logging |
| `-c, --config PATH` | Path to config file (default: `~/.config/pr-bro/config.yaml`) |
| `--non-interactive` | Force plain text output even in a terminal |
| `--format table\|tsv` | Output format when non-interactive (default: `table`) |
| `--no-cache` | Disable HTTP response caching for this run |
| `--clear-cache` | Remove cached GitHub API responses and exit |

## TUI Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `j` / `Down` | Next PR |
| `k` / `Up` | Previous PR |
| `Enter` / `o` | Open PR in browser |
| `s` | Snooze (prompts for duration) |
| `u` | Unsnooze selected PR |
| `z` | Undo last action |
| `d` | Show score breakdown detail |
| `Tab` | Switch between Active/Snoozed view |
| `r` | Manual refresh (fetches fresh data) |
| `?` | Show help overlay |
| `q` / `Ctrl+C` | Quit |

## Authentication

Set the `PR_BRO_GH_TOKEN` environment variable:

```bash
export PR_BRO_GH_TOKEN="ghp_your_token_here"
```

Add this to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) to persist it across sessions.

**Required scopes:**
- `repo` (for private repositories)
- `public_repo` (for public repositories only)

Create a token at: https://github.com/settings/tokens

If not set, pr-bro prompts interactively for the current session.

## Caching

pr-bro caches GitHub API responses using ETags to conserve rate limits. Use `--no-cache` to skip caching or `--clear-cache` to remove cached data.

For details on cache location and behavior, see [Caching](docs/caching.md).

## License

MIT (placeholder - add LICENSE file for actual license)
