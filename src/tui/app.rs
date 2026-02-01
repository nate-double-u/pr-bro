use crate::config::Config;
use crate::github::types::PullRequest;
use crate::scoring::ScoreResult;
use crate::snooze::SnoozeState;
use chrono::{DateTime, Utc};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

const MAX_UNDO: usize = 50;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Active,
    Snoozed,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    SnoozeInput,
    Help,
}

#[derive(Debug, Clone)]
pub enum UndoAction {
    Snoozed {
        url: String,
        title: String,
    },
    Unsnoozed {
        url: String,
        title: String,
        until: Option<DateTime<Utc>>,
    },
}

pub struct App {
    pub active_prs: Vec<(PullRequest, ScoreResult)>,
    pub snoozed_prs: Vec<(PullRequest, ScoreResult)>,
    pub table_state: ratatui::widgets::TableState,
    pub current_view: View,
    pub snooze_state: SnoozeState,
    pub snooze_path: PathBuf,
    pub input_mode: InputMode,
    pub snooze_input: String,
    pub flash_message: Option<(String, Instant)>,
    pub undo_stack: VecDeque<UndoAction>,
    pub last_refresh: Instant,
    pub needs_refresh: bool,
    pub should_quit: bool,
    pub config: Config,
    pub verbose: bool,
}

impl App {
    pub fn new(
        active_prs: Vec<(PullRequest, ScoreResult)>,
        snoozed_prs: Vec<(PullRequest, ScoreResult)>,
        snooze_state: SnoozeState,
        snooze_path: PathBuf,
        config: Config,
        verbose: bool,
    ) -> Self {
        let mut table_state = ratatui::widgets::TableState::default();
        if !active_prs.is_empty() {
            table_state.select(Some(0));
        }

        Self {
            active_prs,
            snoozed_prs,
            table_state,
            current_view: View::Active,
            snooze_state,
            snooze_path,
            input_mode: InputMode::Normal,
            snooze_input: String::new(),
            flash_message: None,
            undo_stack: VecDeque::new(),
            last_refresh: Instant::now(),
            needs_refresh: false,
            should_quit: false,
            config,
            verbose,
        }
    }

    pub fn current_prs(&self) -> &[(PullRequest, ScoreResult)] {
        match self.current_view {
            View::Active => &self.active_prs,
            View::Snoozed => &self.snoozed_prs,
        }
    }

    pub fn next_row(&mut self) {
        let prs = self.current_prs();
        if prs.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= prs.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn previous_row(&mut self) {
        let prs = self.current_prs();
        if prs.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    prs.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn selected_pr(&self) -> Option<&PullRequest> {
        let prs = self.current_prs();
        self.table_state.selected().and_then(|i| prs.get(i).map(|(pr, _)| pr))
    }

    pub fn push_undo(&mut self, action: UndoAction) {
        self.undo_stack.push_front(action);
        if self.undo_stack.len() > MAX_UNDO {
            self.undo_stack.pop_back();
        }
    }

    pub fn update_flash(&mut self) {
        if let Some((_, timestamp)) = self.flash_message {
            if timestamp.elapsed().as_secs() >= 3 {
                self.flash_message = None;
            }
        }
    }

    pub fn show_flash(&mut self, msg: String) {
        self.flash_message = Some((msg, Instant::now()));
    }

    pub fn auto_refresh_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.config.auto_refresh_interval)
    }
}
