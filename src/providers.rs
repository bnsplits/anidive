pub mod animesama;
pub mod home;

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};
use std::time::Duration;

use crate::providers::{animesama::AnimeSamaProvider, home::HomeProvider};

pub struct AnimeProviderState {
    pub title: String,
    pub full_path: Vec<String>,
    pub helps: Vec<(String, String)>,
    pub want_poll: bool,
    pub poll_timeout: Option<Duration>,
}

#[derive(Clone, Copy)]
pub enum AnimeProviderNames {
    AnimeSama,
}

pub enum AnimeProviders {
    HomePage(HomeProvider),
    AnimeSama(AnimeSamaProvider),
}

impl AnimeProviders {
    pub fn state(&self) -> &AnimeProviderState {
        match self {
            AnimeProviders::HomePage(p) => &p.state,
            AnimeProviders::AnimeSama(p) => &p.state,
        }
    }

    pub fn render(&mut self, area: Rect, frame: &mut Frame) {
        match self {
            AnimeProviders::HomePage(p) => p.view(area, frame),
            AnimeProviders::AnimeSama(p) => p.view(area, frame),
        }
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) {
        match self {
            AnimeProviders::HomePage(p) => p.handle_key(key_event),
            AnimeProviders::AnimeSama(p) => p.handle_key(key_event),
        }
    }

    pub fn handle_tick(&mut self) {
        match self {
            AnimeProviders::HomePage(_) => {}
            AnimeProviders::AnimeSama(p) => p.handle_tick(),
        }
    }
}
