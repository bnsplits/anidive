use std::time::Duration;

use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

use crate::providers::state::AnimeProviderState;

pub struct HomeProvider {
    pub state: AnimeProviderState,
}

impl Default for HomeProvider {
    fn default() -> Self {
        Self {
            state: AnimeProviderState {
                name: " AniDive ".into(),
                full_path: vec!["".into()],
                want_poll: false,
                poll_timeout: Duration::from_millis(1000),
                helps: vec![
                    ("[j/k]".into(), "move".into()),
                    ("[enter]".into(), "select".into()),
                ],
            },
        }
    }
}

impl HomeProvider {
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
