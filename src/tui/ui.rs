use ratatui::prelude::*;
use ratatui::widgets::{Block, Cell, Clear, Paragraph, Row, Table, Tabs};
use crate::tui::app::{App, InputMode, View};

pub fn draw(frame: &mut Frame, app: &mut App) {
    // Layout: Title(1) + Tabs(1) + Table(fill) + Status(1)
    let chunks = Layout::vertical([
        Constraint::Length(1),  // Title bar
        Constraint::Length(1),  // Tab bar
        Constraint::Fill(1),    // PR table
        Constraint::Length(1),  // Status bar
    ])
    .split(frame.area());

    render_title(frame, chunks[0]);
    render_tabs(frame, chunks[1], app);
    render_table(frame, chunks[2], app);
    render_status_bar(frame, chunks[3], app);

    // Render overlays based on input mode
    match app.input_mode {
        InputMode::SnoozeInput => render_snooze_popup(frame, app),
        InputMode::Help => render_help_popup(frame),
        InputMode::Normal => {}
    }
}

fn render_title(frame: &mut Frame, area: Rect) {
    let title = Line::from("PR Bro").bold();
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
        .highlight_style(Style::default().reversed())
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
            let bar = score_bar(score_result.score, max_score, 8);
            let score_with_bar = format!("{:>5} {}", score_str, bar);

            // Truncate title to fit available width
            let title = truncate_title(&pr.title, 60);

            Row::new(vec![
                Cell::from(index).style(Style::default().fg(Color::DarkGray)),
                Cell::from(score_with_bar),
                Cell::from(title),
                Cell::from(pr.url.clone()),
            ])
        })
        .collect();

    // Column widths
    let widths = [
        Constraint::Length(4),   // Index: "99."
        Constraint::Length(16),  // Score + bar: "12.3k ████░░░░"
        Constraint::Fill(1),     // Title
        Constraint::Length(50),  // URL
    ];

    let table = Table::new(rows, widths)
        .header(
            Row::new(vec!["#", "Score", "Title", "URL"])
                .style(Style::new().bold())
                .bottom_margin(1),
        )
        .row_highlight_style(Style::new().reversed());

    frame.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let text = if let Some((ref msg, _)) = app.flash_message {
        // Show flash message
        Line::from(msg.clone())
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

        // Context-aware hints based on current view
        let hints = match app.current_view {
            View::Active => "j/k:nav Enter:open s:snooze Tab:snoozed ?:help q:quit",
            View::Snoozed => "j/k:nav Enter:open u:unsnooze Tab:active ?:help q:quit",
        };

        Line::from(vec![
            Span::styled(count, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(view_mode, Style::default().fg(Color::DarkGray)),
            Span::raw(" "),
            Span::styled(refresh_time, Style::default().fg(Color::DarkGray)),
            Span::raw("  "),
            Span::raw(hints),
        ])
    };

    frame.render_widget(Paragraph::new(text), area);
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

fn score_bar(score: f64, max_score: f64, width: usize) -> String {
    let ratio = if max_score > 0.0 {
        (score / max_score).min(1.0)
    } else {
        0.0
    };
    let filled = (ratio * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
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
        .style(Style::default().fg(Color::DarkGray));
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
            Span::raw("Refresh PRs"),
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
            Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))
        ),
    ];

    let help_text = Paragraph::new(help_lines);
    frame.render_widget(help_text, inner);
}
