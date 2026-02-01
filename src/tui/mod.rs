pub mod app;
pub mod event;
pub mod ui;

pub use app::App;

use crate::scoring::ScoringConfig;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use event::{Event, EventHandler};

pub async fn run_tui(
    mut app: App,
    _client: octocrab::Octocrab,
    _scoring_config: ScoringConfig,
) -> anyhow::Result<()> {
    // Init terminal (sets up panic hooks automatically)
    let mut terminal = ratatui::init();

    // Create event handler
    let mut events = EventHandler::new(250); // 250ms tick

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|frame| ui::draw(frame, &mut app))?;

        // Handle events
        match events.next().await {
            Event::Key(key) => handle_key_event(&mut app, key),
            Event::Tick => app.update_flash(),
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    ratatui::restore();
    Ok(())
}

fn handle_key_event(app: &mut App, key: KeyEvent) {
    match app.input_mode {
        app::InputMode::Normal => {
            match key.code {
                // Quit
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.should_quit = true
                }

                // Navigation
                KeyCode::Char('j') | KeyCode::Down => app.next_row(),
                KeyCode::Char('k') | KeyCode::Up => app.previous_row(),

                // Open PR in browser
                KeyCode::Enter | KeyCode::Char('o') => {
                    if let Some(pr) = app.selected_pr() {
                        let title = pr.title.clone();
                        if let Err(e) = app.open_selected() {
                            app.show_flash(format!("Failed to open browser: {}", e));
                        } else {
                            app.show_flash(format!("Opened: {}", title));
                        }
                    }
                }

                // Snooze
                KeyCode::Char('s') => app.start_snooze_input(),

                // Unsnooze
                KeyCode::Char('u') => app.unsnooze_selected(),

                // Undo
                KeyCode::Char('z') => app.undo_last(),

                // Stubs for future plans
                KeyCode::Tab => {} // View switching (Plan 04)
                KeyCode::Char('r') => {} // Refresh (Plan 04)
                KeyCode::Char('?') => {} // Help (Plan 04)

                _ => {}
            }
        }
        app::InputMode::SnoozeInput => {
            match key.code {
                // Confirm snooze
                KeyCode::Enter => app.confirm_snooze_input(),

                // Cancel snooze
                KeyCode::Esc => app.cancel_snooze_input(),

                // Backspace
                KeyCode::Backspace => {
                    app.snooze_input.pop();
                }

                // Character input (alphanumeric + space)
                KeyCode::Char(c) if c.is_alphanumeric() || c == ' ' => {
                    app.snooze_input.push(c);
                }

                // Ignore all other keys (don't propagate to Normal mode)
                _ => {}
            }
        }
        app::InputMode::Help => {
            // Any key exits help (will be implemented in Plan 04)
            match key.code {
                KeyCode::Esc | KeyCode::Char('?') => {
                    app.input_mode = app::InputMode::Normal;
                }
                _ => {
                    app.input_mode = app::InputMode::Normal;
                }
            }
        }
    }
}
