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
}
