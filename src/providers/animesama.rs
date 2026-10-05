use std::time::Duration;

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

use crate::providers::AnimeProviderState;

pub struct AnimeSamaProvider {
    pub state: AnimeProviderState,
}

impl Default for AnimeSamaProvider {
    fn default() -> Self {
        Self {
            state: AnimeProviderState {
                title: " AnimeSama ".into(),
                full_path: vec!["".into()],
                want_poll: true,
                poll_timeout: Some(Duration::from_millis(1000)),
                helps: vec![
                    ("[j/k]".into(), "move".into()),
                    ("[enter]".into(), "select".into()),
                ],
            },
        }
    }
}

impl AnimeSamaProvider {
    pub fn view(&self, area: Rect, frame: &mut Frame) {
        //TODO
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) {
        //TODO
    }

    pub fn handle_tick(&mut self) {
        //TODO
    }
}
