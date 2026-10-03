use ratatui::{Frame, layout::Rect};

use crate::providers::{
    animesama::AnimeSamaProvider, home::HomeProvider, state::AnimeProviderState,
};

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

    pub fn state_mut(&mut self) -> &mut AnimeProviderState {
        match self {
            AnimeProviders::HomePage(p) => &mut p.state,
            AnimeProviders::AnimeSama(p) => &mut p.state,
        }
    }

    pub fn render(&self, area: Rect, frame: &mut Frame) {
        match self {
            AnimeProviders::HomePage(p) => p.view(area, frame),
            AnimeProviders::AnimeSama(p) => p.view(area, frame),
        }
    }
}
