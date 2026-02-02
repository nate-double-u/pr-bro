# pr-bro

GitHub PR review prioritization CLI/TUI

**Know which PR to review next.** pr-bro helps developers prioritize pull request reviews based on weighted scoring (age, approvals, size) across multiple GitHub queries. Features an interactive TUI with keyboard navigation, smart caching, and flexible per-query configuration.

## Features

- Weighted scoring based on age, approvals, PR size, labels, and review history
- Interactive TUI with keyboard navigation and real-time updates
- Multiple query support with per-query scoring overrides
- Snooze PRs (timed or indefinite) to hide them temporarily
- Score breakdown detail view (press `d` to see how a PR's score was calculated)
- ETag-based HTTP caching for rate limit conservation
- Parallel API fetching for faster startup
- Secure keyring credential storage (macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Config validation with clear error messages (catches typos and overlapping size ranges)

## Installation

### From Source

Requires Rust toolchain (1.70+) and system keyring support.

```bash
cargo install --path .
```

### Prerequisites

- **Rust**: Install from [rustup.rs](https://rustup.rs)
- **System Keyring**: macOS Keychain (built-in), Windows Credential Manager (built-in), or Linux Secret Service (GNOME Keyring, KWallet)

## Quick Start

On first run (without `PR_BRO_GH_TOKEN` set), pr-bro will prompt for your GitHub Personal Access Token and store it securely in your system keyring.

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

### Full Configuration Example

```yaml
# Auto-refresh interval in seconds (default: 300 = 5 minutes)
auto_refresh_interval: 300

# Global scoring configuration (applies to all queries unless overridden)
scoring:
  base_score: 100
  age: "+1 per 1h"       # Adds 1 point per hour of age
  approvals: "+10 per 1"  # Adds 10 points per approval
  size:
    exclude: ["*.lock", "package-lock.json"]
    buckets:
      - range: "<100"
        effect: "x5"      # Small PRs get 5x multiplier
      - range: "100-500"
        effect: "x1"      # Medium PRs: no change
      - range: ">500"
        effect: "x0.5"    # Large PRs get 0.5x penalty
  labels:
    - name: "urgent"
      effect: "+10"
    - name: "wip"
      effect: "x0.5"
  previously_reviewed: "x0.5"

# Queries to execute (at least one required)
queries:
  - name: my-reviews
    query: "is:pr review-requested:@me"

  - name: team-prs
    query: "is:pr org:myorg"
    scoring:               # Per-query scoring (merges with global)
      base_score: 50
      age: "+2 per 1h"
      approvals: "+5 per 1"
```

### Scoring Factors

Each scoring factor is optional and can use addition (`+N`) or multiplication (`xN`) effects.

#### Age

Format: `"+N per DURATION"` or `"xN per DURATION"`

Duration uses humantime format: `1h`, `30m`, `1d`, `1w`

Examples:
- `"+1 per 1h"` — adds 1 point per hour of age
- `"x1.1 per 1d"` — multiplies score by 1.1 per day of age

#### Approvals

Format: `"+N per 1"`, `"xN per 1"`, `"+N"`, or `"xN"`

This is NOT bucket-based — the effect applies per approval count.

Examples:
- `"+10 per 1"` — adds 10 points per approval
- `"x2 per 1"` — doubles score per approval
- `"+50"` — adds 50 points if any approvals exist

#### Size

Bucket-based configuration with optional file exclusions.

**Range formats:**
- `"<N"` — less than N lines
- `"<=N"` — less than or equal to N lines
- `">N"` — greater than N lines
- `">=N"` — greater than or equal to N lines
- `"N-M"` — inclusive range from N to M lines

**Effect formats:**
- `"+N"` — add N points
- `"xN"` — multiply score by N

**Important:** Size bucket ranges must NOT overlap. The validator will reject configurations with overlapping ranges at startup.

Example:

```yaml
size:
  exclude:
    - "*.lock"
    - "package-lock.json"
    - "yarn.lock"
  buckets:
    - range: "<100"
      effect: "x5"      # Small PRs: 5x multiplier
    - range: "100-500"
      effect: "x1"      # Medium PRs: no change
    - range: ">500"
      effect: "x0.5"    # Large PRs: 0.5x penalty
```

**Exclude pattern behavior:**
- Patterns match against the **filename only** (basename), not the full file path. For example, `*.lock` will match `Cargo.lock` and `subdir/package-lock.json`.
- When exclude patterns are configured, pr-bro fetches per-file diff data from the GitHub API to determine which files to exclude. This adds 1-2 API calls per PR (paginated at 100 files per page).
- If the per-file data fetch fails (e.g., rate limit), pr-bro falls back to the aggregate size from the PR summary (no exclusions applied).
- Without exclude patterns, no extra API calls are made.
- Invalid glob patterns are caught at startup during config validation.

#### Labels

Optional. Applies score effects based on GitHub labels on the PR. Multiple matching labels compound their effects sequentially (not first-match). Label matching is **case-insensitive**.

```yaml
labels:
  - name: "urgent"
    effect: "+10"     # Add 10 points for urgent PRs
  - name: "wip"
    effect: "x0.5"    # Halve score for work-in-progress
  - name: "critical"
    effect: "x2"      # Double score for critical PRs
```

A PR with both "urgent" and "critical" labels gets both effects: score + 10, then x2.

Each matching label appears as a separate entry in the score breakdown detail view (press `d`).

#### Previously Reviewed

Optional. Applies a score effect when the authenticated user (the user whose token is configured) has previously submitted a review on the PR. This includes all review states: approved, changes requested, commented, or dismissed.

```yaml
previously_reviewed: "x0.5"   # De-prioritize already-reviewed PRs
```

Or to boost PRs you've already engaged with:

```yaml
previously_reviewed: "+20"    # Boost PRs you've reviewed before
```

Detection uses the GitHub reviews API data that is already fetched for approval counting — no extra API calls. The authenticated username is fetched once at startup.

### Effect Syntax Summary

| Syntax | Meaning |
|--------|---------|
| `+N` | Add N points |
| `xN` | Multiply score by N |
| `+N per DURATION` | Add N points per time unit (age only) |
| `xN per DURATION` | Multiply by N per time unit (age only) |
| `+N per M` | Add N points per M units (approvals only) |
| `xN per M` | Multiply by N per M units (approvals only) |

Labels and previously_reviewed use flat effects (`+N` or `xN`), not per-unit effects.

### Per-Query Scoring

Queries can override individual fields of the global scoring configuration. When a PR appears in multiple queries, the **first query's scoring is used** (first-match-wins). Per-query scoring merges with global scoring at the **leaf level** — only the exact sub-fields you specify in a query override the global values; everything else is inherited. This means setting `scoring.size.exclude` in a query does **not** replace the entire `size` block; global `size.buckets` are preserved (and vice versa).

Example:

```yaml
scoring:
  base_score: 100
  age: "+1 per 1h"
  approvals: "+10 per 1"
  size:
    buckets:
      - range: "<100"
        effect: "x5"
      - range: "100-500"
        effect: "x1"
      - range: ">500"
        effect: "x0.5"
  labels:
    - name: "urgent"
      effect: "+20"
    - name: "wip"
      effect: "x0.5"

queries:
  - name: urgent
    query: "is:pr label:urgent"
    scoring:
      age: "+10 per 1h"       # Override: urgent PRs age faster
      size:
        exclude: ["*.lock"]   # Add exclude — inherits global buckets
      labels:
        - name: "urgent"
          effect: "+50"       # Override: stronger urgent boost for this query
      # base_score, approvals — inherited from global
      # size.buckets — inherited from global (not overridden)
      # label "wip" — inherited from global (not mentioned here)

  - name: other
    query: "is:pr org:myorg"
    # No scoring block — uses global scoring entirely
```

In this example, the "urgent" query:
- **Overrides** `age` to `"+10 per 1h"` (urgent PRs age faster).
- **Adds** `size.exclude` with `["*.lock"]`. Because merging is leaf-level, global `size.buckets` are inherited — setting `size.exclude` does NOT replace the entire `size` block.
- **Overrides** the "urgent" label effect from `"+20"` to `"+50"`. Labels merge by name (case-insensitive): the query's "urgent" label wins over the global one. The global "wip" label is preserved because the query does not mention it.
- **Inherits** `base_score`, `approvals`, and `previously_reviewed` from the global config (not specified in the query, so global values apply).

#### YAML Merge Keys

YAML merge keys (`<<:`) are supported by the YAML parser for reducing duplication within your config file. This is a YAML feature processed when reading the file, independent of the runtime merge that combines global and per-query scoring. Note that because pr-bro validates config structure strictly (`deny_unknown_fields`), YAML anchors must be placed inside fields that expect the anchored structure, not at the top level. For advanced YAML anchor/merge-key usage, refer to the [YAML specification](https://yaml.org/type/merge.html).

### Config Validation

pr-bro validates your configuration at startup with clear error messages:

- **Unknown YAML keys** are rejected (catches typos like `approvalls` instead of `approvals`)
- **Overlapping size bucket ranges** are rejected (prevents ambiguous scoring)
- **Invalid effect syntax** is caught with helpful messages
- **Empty label names** are rejected
- **Invalid glob patterns** in `size.exclude` are caught (e.g., unclosed character classes like `[invalid`)
- **Invalid label effects** and **invalid previously_reviewed effects** are caught at startup

Validation errors will show exactly what's wrong and where, so you can fix configuration issues quickly.

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

### Snooze Duration Input

When prompted for snooze duration:
- Enter duration like `2h`, `3d`, `1w` and press `Enter`
- Leave empty and press `Enter` for indefinite snooze
- Press `Esc` to cancel

### Score Breakdown

Press `d` to see how the selected PR's score was calculated. Shows:
- Base score
- Age contribution (if configured)
- Approvals contribution (if configured)
- Size contribution (if configured)
- Label contributions (if configured and labels match)
- Previously reviewed contribution (if configured and you've reviewed the PR)

Press `Esc` or `d` again to dismiss.

## Credentials

### GitHub Personal Access Token

pr-bro stores your GitHub token securely in your system keyring:
- **macOS**: Keychain
- **Windows**: Credential Manager
- **Linux**: Secret Service (GNOME Keyring, KWallet)

### Environment Variable

For CI pipelines, scripts, or environments without a system keyring, set the `PR_BRO_GH_TOKEN` environment variable:

```bash
export PR_BRO_GH_TOKEN="ghp_your_token_here"
pr-bro list
```

Behavior:
- When `PR_BRO_GH_TOKEN` is set and non-empty, the token is used directly with **no keyring access and no interactive prompt**
- Empty or whitespace-only values are treated as unset (falls through to keyring)
- If the token is invalid (401), pr-bro will prompt for a new token and store it in the keyring (fix the env var externally for future runs)
- Verbose mode (`-v`) reports whether the token came from the env var or system keyring

### First Run

On first run (without `PR_BRO_GH_TOKEN` set), pr-bro will prompt for your GitHub Personal Access Token and store it securely in your system keyring. Create one at:
https://github.com/settings/tokens

**Required scopes:**
- `repo` (for private repositories)
- `public_repo` (for public repositories only)

### Re-authenticating

If your token becomes invalid (401 error), pr-bro will automatically prompt you for a new token. In TUI mode, the terminal will be temporarily restored for token input, then resume the TUI.

### Resetting Token

To manually reset your stored token, delete the keyring entry:
- **macOS**: Open Keychain Access, search for "pr-bro", delete entry
- **Windows**: Open Credential Manager, search for "pr-bro", remove entry
- **Linux**: Use your keyring manager to remove the "pr-bro" entry

Then run pr-bro again to be prompted for a new token.

## Caching

pr-bro uses ETag-based HTTP caching to reduce GitHub API rate limit consumption.

### Cache Location

Platform-specific cache directory:
- **macOS**: `~/Library/Caches/pr-bro`
- **Linux**: `~/.cache/pr-bro`
- **Windows**: `%LOCALAPPDATA%\pr-bro\cache`

### Cache Behavior

- **In-memory cache**: Fast access to recently fetched data
- **Disk cache**: Persistent storage using ETags for validation
- **Manual refresh** (`r` key in TUI): Bypasses in-memory cache for fresh data
- **Auto-refresh**: Uses cache (only fetches if data changed on server)

### Cache Management

```bash
# Disable caching for one run
pr-bro --no-cache

# Clear all cached responses
pr-bro --clear-cache
```

Clearing cache removes all stored API responses but preserves configuration and snooze state.

## License

MIT (placeholder - add LICENSE file for actual license)
