//! Centralized theme module for TUI color constants and styles

use ratatui::prelude::*;

// Score-based colors (based on relative score percentage 0-100%)
pub const SCORE_HIGH: Color = Color::Green;    // >= 70% of max
pub const SCORE_MID: Color = Color::Yellow;    // >= 40% of max
pub const SCORE_LOW: Color = Color::Red;       // < 40% of max

/// Returns the appropriate color for a score based on its percentage of max score
pub fn score_color(score: f64, max_score: f64) -> Color {
    let percentage = if max_score > 0.0 {
        (score / max_score) * 100.0
    } else {
        0.0
    };

    if percentage >= 70.0 {
        SCORE_HIGH
    } else if percentage >= 40.0 {
        SCORE_MID
    } else {
        SCORE_LOW
    }
}

// Score bar colors (same thresholds as score text)
pub const BAR_FILLED_HIGH: Color = Color::Green;
pub const BAR_FILLED_MID: Color = Color::Yellow;
pub const BAR_FILLED_LOW: Color = Color::Red;
pub const BAR_EMPTY: Color = Color::DarkGray;

// Table colors
pub const ROW_ALT_BG: Color = Color::Indexed(235);  // 256-color dark gray for alternating rows
pub const INDEX_COLOR: Color = Color::DarkGray;     // Index column color

// Styles
pub const TITLE_STYLE: Style = Style::new().bold();
pub const HEADER_STYLE: Style = Style::new().bold();
pub const TAB_ACTIVE: Style = Style::new().reversed();
pub const ROW_SELECTED: Style = Style::new().reversed();

// General colors
pub const MUTED: Color = Color::DarkGray;  // For secondary text
