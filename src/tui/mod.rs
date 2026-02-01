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
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    app.should_quit = true
                }
                KeyCode::Char('j') | KeyCode::Down => app.next_row(),
                KeyCode::Char('k') | KeyCode::Up => app.previous_row(),
                // Other keys (Enter/o, s, u, Tab, r, ?, z) are stubs for now
                _ => {}
            }
        }
        // Other input modes will be handled in Plan 03
        _ => {}
    }
}
