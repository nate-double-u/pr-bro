use ratatui::prelude::*;
use ratatui::widgets::{Block, Cell, Clear, Paragraph, Row, Table, Tabs};
use crate::tui::app::{App, InputMode, View};
use crate::tui::theme;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Handle very small terminal sizes gracefully
    if area.height < 6 || area.width < 30 {
        let msg = Paragraph::new("Terminal too small")
            .alignment(Alignment::Center);
        frame.render_widget(msg, area);
        return;
    }

    // Layout: Title(1) + Tabs(1) + Table(fill) + Status(1)
    let chunks = Layout::vertical([
        Constraint::Length(1),  // Title bar
        Constraint::Length(1),  // Tab bar
        Constraint::Fill(1),    // PR table
        Constraint::Length(1),  // Status bar
    ])
    .split(area);

    render_title(frame, chunks[0], app);
    render_tabs(frame, chunks[1], app);
    render_table(frame, chunks[2], app);
    render_status_bar(frame, chunks[3], app);

    // Render overlays based on input mode
    match app.input_mode {
        InputMode::SnoozeInput => render_snooze_popup(frame, app),
        InputMode::Help => render_help_popup(frame),
        InputMode::Normal => {}
    }

    // Render loading overlay if loading (appears on top of everything)
    if app.is_loading {
        render_loading_overlay(frame, app);
    }
}

fn render_title(frame: &mut Frame, area: Rect, app: &App) {
    // Build title with rate limit on the right
    let mut spans = vec![Span::styled("PR Bro", Style::default().fg(theme::TITLE_COLOR).bold())];

    // Add rate limit info on the right if available
    if let Some(remaining) = app.rate_limit_remaining {
        let rate_limit_text = format!("API: {} remaining", remaining);
        let left_len = "PR Bro".len();
        let right_len = rate_limit_text.len();
        let padding_len = (area.width as usize).saturating_sub(left_len + right_len);

        // Add padding and rate limit text
        spans.push(Span::raw(" ".repeat(padding_len)));
        spans.push(Span::styled(rate_limit_text, Style::default().fg(theme::MUTED)));
    }

    let title = Line::from(spans);
    frame.render_widget(Paragraph::new(title), area);
}

fn render_tabs(frame: &mut Frame, area: Rect, app: &App) {
    let titles = vec!["Active", "Snoozed"];
    let selected = match app.current_view {
        View::Active => 0,
        View::Snoozed => 1,
    };

    let tabs = Tabs::new(titles)
        .select(selected)
        .style(Style::default().fg(theme::MUTED))
        .highlight_style(Style::default().fg(theme::TITLE_COLOR).bold().reversed())
        .divider(" | ");

    frame.render_widget(tabs, area);
}

fn render_table(frame: &mut Frame, area: Rect, app: &mut App) {
    let prs = app.current_prs();

    if prs.is_empty() {
        let empty_msg = Paragraph::new("No PRs to review")
            .alignment(Alignment::Center)
            .block(Block::default());
        frame.render_widget(empty_msg, area);
        return;
    }

    // Calculate max score for bar scaling
    let max_score = prs.iter()
        .map(|(_, result)| result.score)
        .fold(0.0_f64, f64::max);

    // Build rows
    let rows: Vec<Row> = prs
        .iter()
        .enumerate()
        .map(|(idx, (pr, score_result))| {
            let index = format!("{}.", idx + 1);
            let score_str = format_score(score_result.score, score_result.incomplete);
            let bar_line = score_bar(score_result.score, max_score, 8);

            // Build score cell with colored text and bar
            let score_color = theme::score_color(score_result.score, max_score);
            let mut score_spans = vec![
                Span::styled(format!("{:>5} ", score_str), Style::default().fg(score_color))
            ];
            score_spans.extend(bar_line.spans);
            let score_line = Line::from(score_spans);

            // Truncate title to fit available width
            let title = truncate_title(&pr.title, 60);

            // Alternating row background (odd rows get subtle background)
            let row_style = if idx % 2 == 1 {
                Style::default().bg(theme::ROW_ALT_BG)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(index).style(Style::default().fg(theme::INDEX_COLOR)),
                Cell::from(score_line),
                Cell::from(title),
                Cell::from(pr.short_ref()),
            ])
            .style(row_style)
        })
        .collect();

    // Column widths
    let widths = [
        Constraint::Length(4),   // Index: "99."
        Constraint::Length(16),  // Score + bar: "12.3k ████░░░░"
        Constraint::Fill(1),     // Title
        Constraint::Length(30),  // PR: "owner/repo#123"
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["#", "Score", "Title", "PR"])
                .style(theme::HEADER_STYLE)
                .bottom_margin(1),
        )
        .row_highlight_style(theme::ROW_SELECTED);

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let text = if let Some((ref msg, _)) = app.flash_message {
        // Show flash message with color based on message type
        let msg_color = if msg.starts_with("Failed") || msg.starts_with("Error") || msg.contains("cancelled") {
            theme::FLASH_ERROR
        } else if msg.starts_with("Snoozed:") || msg.starts_with("Unsnoozed:") ||
                  msg.starts_with("Refreshed") || msg.starts_with("Opened:") {
            theme::FLASH_SUCCESS
        } else {
            Color::White  // Default for unknown message types
        };
        Line::from(Span::styled(msg.clone(), Style::default().fg(msg_color)))
    } else {
        // Show normal status
        let prs = app.current_prs();
        let count = format!("{} PRs", prs.len());

        let view_mode = match app.current_view {
            View::Active => "Active",
            View::Snoozed => "Snoozed",
        };

        let elapsed = app.last_refresh.elapsed();
        let refresh_time = if elapsed.as_secs() < 60 {
            format!("refreshed {}s ago", elapsed.as_secs())
        } else {
            format!("refreshed {}m ago", elapsed.as_secs() / 60)
        };

        // Build hints with colored shortcut keys
        let mut hint_spans = Vec::new();
        let hints = match app.current_view {
            View::Active => vec![
                ("j", "/", "k", ":nav "),
                ("Enter", "", "", ":open "),
                ("s", "", "", ":snooze "),
                ("r", "", "", ":refresh "),
                ("Tab", "", "", ":snoozed "),
                ("?", "", "", ":help "),
                ("q", "", "", ":quit"),
            ],
            View::Snoozed => vec![
                ("j", "/", "k", ":nav "),
                ("Enter", "", "", ":open "),
                ("u", "", "", ":unsnooze "),
                ("r", "", "", ":refresh "),
                ("Tab", "", "", ":active "),
                ("?", "", "", ":help "),
                ("q", "", "", ":quit"),
            ],
        };

        for (i, (key1, sep, key2, label)) in hints.iter().enumerate() {
            if i > 0 {
                hint_spans.push(Span::raw(" "));
            }
            hint_spans.push(Span::styled(*key1, Style::default().fg(theme::STATUS_KEY_COLOR)));
            if !sep.is_empty() {
                hint_spans.push(Span::raw(*sep));
                hint_spans.push(Span::styled(*key2, Style::default().fg(theme::STATUS_KEY_COLOR)));
            }
            hint_spans.push(Span::raw(*label));
        }

        let mut spans = vec![
            Span::styled(count, Style::default().fg(theme::MUTED)),
            Span::raw(" "),
            Span::styled(view_mode, Style::default().fg(theme::MUTED)),
            Span::raw(" "),
            Span::styled(refresh_time, Style::default().fg(theme::MUTED)),
            Span::raw("  "),
        ];
        spans.extend(hint_spans);
        Line::from(spans)
    };

    frame.render_widget(
        Paragraph::new(text).style(Style::default().bg(theme::STATUS_BAR_BG)),
        area
    );
}

fn format_score(score: f64, incomplete: bool) -> String {
    let formatted = if score >= 1_000_000.0 {
        format!("{:.1}M", score / 1_000_000.0)
    } else if score >= 1_000.0 {
        format!("{:.1}k", score / 1_000.0)
    } else {
        format!("{:.0}", score)
    };

    // Trim trailing .0
    let trimmed = formatted
        .replace(".0M", "M")
        .replace(".0k", "k");

    if incomplete {
        format!("{}*", trimmed)
    } else {
        trimmed
    }
}

fn score_bar(score: f64, max_score: f64, width: usize) -> Line<'static> {
    let ratio = if max_score > 0.0 {
        (score / max_score).min(1.0)
    } else {
        0.0
    };
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);

    // Get color based on score
    let bar_color = theme::score_color(score, max_score);

    let mut spans = Vec::new();
    if filled > 0 {
        spans.push(Span::styled("█".repeat(filled), Style::default().fg(bar_color)));
    }
    if empty > 0 {
        spans.push(Span::styled("░".repeat(empty), Style::default().fg(theme::BAR_EMPTY)));
    }

    Line::from(spans)
}

fn truncate_title(title: &str, max_width: usize) -> String {
    let chars: Vec<char> = title.chars().collect();
    if chars.len() <= max_width {
        title.to_string()
    } else if max_width > 3 {
        format!("{}...", chars[..max_width - 3].iter().collect::<String>())
    } else {
        chars[..max_width].iter().collect()
    }
}

/// Render the snooze duration input popup
fn render_snooze_popup(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect_fixed(40, 5, frame.area());

    // Clear the background
    frame.render_widget(Clear, popup_area);

    // Render the popup border
    let block = Block::bordered().title("Snooze Duration");
    frame.render_widget(block.clone(), popup_area);

    // Get inner area (inside the border)
    let inner = block.inner(popup_area);

    // Split inner area for input and help text
    let chunks = Layout::vertical([
        Constraint::Length(1),  // Input line
        Constraint::Length(1),  // Help text
    ])
    .split(inner);

    // Render input with cursor
    let input_text = format!("{}|", app.snooze_input);
    let input = Paragraph::new(input_text);
    frame.render_widget(input, chunks[0]);

    // Render help text
    let help = Paragraph::new("Enter: confirm | Esc: cancel | empty = indefinite")
        .style(Style::default().fg(theme::MUTED));
    frame.render_widget(help, chunks[1]);
}

/// Create a centered rectangle with fixed width and height
fn centered_rect_fixed(width: u16, height: u16, area: Rect) -> Rect {
    // Clamp dimensions to area bounds
    let width = width.min(area.width);
    let height = height.min(area.height);

    // Calculate centered position
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;

    Rect {
        x,
        y,
        width,
        height,
    }
}

/// Render the help overlay popup
fn render_help_popup(frame: &mut Frame) {
    let popup_area = centered_rect_fixed(50, 16, frame.area());

    // Clear the background
    frame.render_widget(Clear, popup_area);

    // Render the popup border
    let block = Block::bordered().title(" Keyboard Shortcuts ");
    frame.render_widget(block.clone(), popup_area);

    // Get inner area (inside the border)
    let inner = block.inner(popup_area);

    // Build help text with two-column layout
    let help_lines = vec![
        Line::from(vec![
            Span::styled("j / Down      ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Move down"),
        ]),
        Line::from(vec![
            Span::styled("k / Up        ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Move up"),
        ]),
        Line::from(vec![
            Span::styled("Enter / o     ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Open PR in browser"),
        ]),
        Line::from(vec![
            Span::styled("s             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Snooze PR"),
        ]),
        Line::from(vec![
            Span::styled("u             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Unsnooze PR"),
        ]),
        Line::from(vec![
            Span::styled("z             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Undo last action"),
        ]),
        Line::from(vec![
            Span::styled("Tab           ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Toggle Active/Snoozed"),
        ]),
        Line::from(vec![
            Span::styled("r             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Refresh PRs (bypasses cache)"),
        ]),
        Line::from(vec![
            Span::styled("?             ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Show/hide this help"),
        ]),
        Line::from(vec![
            Span::styled("q / Ctrl-c    ", Style::default().fg(Color::Cyan).bold()),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(
            Span::styled("Press any key to close", Style::default().fg(theme::MUTED))
        ),
    ];

    let help_text = Paragraph::new(help_lines);
    frame.render_widget(help_text, inner);
}

/// Render the loading spinner overlay
fn render_loading_overlay(frame: &mut Frame, app: &App) {
    let popup_area = centered_rect_fixed(30, 3, frame.area());

    // Clear the background
    frame.render_widget(Clear, popup_area);

    // Render the popup border
    let block = Block::bordered();
    frame.render_widget(block.clone(), popup_area);

    // Get inner area (inside the border)
    let inner = block.inner(popup_area);

    // Braille spinner animation
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = spinner_chars[app.spinner_frame % 10];

    // Display different text based on whether this is initial load or refresh
    let text = if app.active_prs.is_empty() && app.snoozed_prs.is_empty() {
        format!("{} Loading PRs...", spinner)
    } else {
        format!("{} Refreshing...", spinner)
    };

    let loading_text = Paragraph::new(text)
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Cyan));

    frame.render_widget(loading_text, inner);
}
