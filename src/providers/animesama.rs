use ratatui::{Frame, layout::Rect};

use crate::providers::state::AnimeProviderState;

pub struct AnimeSamaProvider {
    pub state: AnimeProviderState,
}

impl Default for AnimeSamaProvider {
    fn default() -> Self {
        Self {
            state: AnimeProviderState {
                name: "AnimeSama".into(),
                full_path: vec!["".into()],
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
        // TODO
    }
}
