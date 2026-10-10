pub mod animesama;
pub mod home;

use crossterm::event::{Event, KeyEvent};
use ratatui::{Frame, layout::Rect};

use crate::{
    ThreadEvent::{self, AnimeSamaEvent, CrosstermEvent},
    pages::{animesama::AnimeSamaPage, home::HomePage},
};

pub struct PageState {
    pub title: String,
    pub full_path: Vec<String>,
    pub helps: Vec<(String, String)>,
    pub input_mode: Option<InputMode>,
}

#[derive(Clone, Copy)]
pub enum InputMode {
    Normal,
    Insert,
}

#[derive(Clone, Copy)]
pub enum AnimePageNames {
    AnimeSama,
}

pub enum AnimePages {
    HomePage(HomePage),
    AnimeSama(AnimeSamaPage),
}

impl AnimePages {
    pub fn state(&self) -> &PageState {
        match self {
            AnimePages::HomePage(p) => &p.state,
            AnimePages::AnimeSama(p) => &p.state,
        }
    }

    pub fn state_mut(&mut self) -> &mut PageState {
        match self {
            AnimePages::HomePage(p) => &mut p.state,
            AnimePages::AnimeSama(p) => &mut p.state,
        }
    }

    pub fn render(&mut self, area: Rect, frame: &mut Frame) {
        match self {
            AnimePages::HomePage(p) => p.view(area, frame),
            AnimePages::AnimeSama(p) => p.view(area, frame),
        }
    }

    pub fn handle_crossterm_key(&mut self, key_event: KeyEvent) {
        match self {
            AnimePages::HomePage(p) => p.handle_crossterm_key(key_event),
            AnimePages::AnimeSama(p) => p.handle_crossterm_key(key_event),
        }
    }

    pub fn handle_crossterm_raw_event(&mut self, event: &Event) {
        match self {
            AnimePages::HomePage(_) => {}
            AnimePages::AnimeSama(p) => p.handle_crossterm_raw_event(event),
        }
    }

    pub fn handle_thread_event(&mut self, event: &ThreadEvent) {
        match event {
            AnimeSamaEvent(e) => {
                if let AnimePages::AnimeSama(p) = self {
                    p.handle_thread_event(e);
                }
            }
            CrosstermEvent(e) => {
                print!("{:?}", e)
            }
        }
    }
}
