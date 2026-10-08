pub mod animesama;
pub mod home;

use crossterm::event::{Event, KeyEvent};
use ratatui::{Frame, layout::Rect};
use std::time::Duration;

use crate::pages::{animesama::AnimeSamaPage, home::HomePage};

pub struct PageState {
    pub title: String,
    pub full_path: Vec<String>,
    pub helps: Vec<(String, String)>,
    pub input_mode: InputMode,
    pub can_change_input_mode: bool,
    pub want_poll: bool,
    pub poll_timeout: Option<Duration>,
}

pub enum InputMode {
    Normal,
    Insert,
}

#[derive(Clone, Copy)]
pub enum AnimePageNames {
    AnimeSama,
}

pub enum AnimePagess {
    HomePage(HomePage),
    AnimeSama(AnimeSamaPage),
}

impl AnimePagess {
    pub fn state(&self) -> &PageState {
        match self {
            AnimePagess::HomePage(p) => &p.state,
            AnimePagess::AnimeSama(p) => &p.state,
        }
    }

    pub fn state_mut(&mut self) -> &mut PageState {
        match self {
            AnimePagess::HomePage(p) => &mut p.state,
            AnimePagess::AnimeSama(p) => &mut p.state,
        }
    }

    pub fn render(&mut self, area: Rect, frame: &mut Frame) {
        match self {
            AnimePagess::HomePage(p) => p.view(area, frame),
            AnimePagess::AnimeSama(p) => p.view(area, frame),
        }
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) {
        match self {
            AnimePagess::HomePage(p) => p.handle_key(key_event),
            AnimePagess::AnimeSama(p) => p.handle_key(key_event),
        }
    }

    pub fn handle_raw_event(&mut self, event: &Event) {
        match self {
            AnimePagess::HomePage(_) => {}
            AnimePagess::AnimeSama(p) => p.handle_raw_event(event),
        }
    }

    pub fn handle_tick(&mut self) {
        match self {
            AnimePagess::HomePage(_) => {}
            AnimePagess::AnimeSama(p) => p.handle_tick(),
        }
    }
}
