pub mod downloaders;
pub mod downloads_step;
pub mod extractors;
pub mod search_step;
pub mod select_episodes_step;
pub mod select_lang_step;
pub mod select_season_step;

use std::{fmt::Display, path::PathBuf, sync::mpsc};

use crossterm::event::{Event, KeyEvent};
use ratatui::{Frame, layout::Rect, widgets::ListState};
use throbber_widgets_tui::ThrobberState;
use tui_input::Input;

use crate::{
    ThreadEvent,
    pages::{
        InputMode, PageState,
        animesama::{
            downloads_step::DownloadsStepState, search_step::SearchStepState,
            select_episodes_step::SelectEpisodesStepState, select_lang_step::SelectLangStepState,
            select_season_step::SelectSeasonStepState,
        },
    },
};

type AnimeResult = (String, String, String); // title, description, link
type SeasonResult = (String, String); // name, url
type LangResult = (String, String); // language, url

#[derive(Clone, Copy)]
enum EpisodeProvider {
    Ansembed,
    Uqload,
    Sendvid,
    Embed4me,
    Voe,
    Unknown,
}

impl Display for EpisodeProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            EpisodeProvider::Ansembed => "Ansembed",
            EpisodeProvider::Uqload => "Uqload",
            EpisodeProvider::Sendvid => "Sendvid",
            EpisodeProvider::Embed4me => "Embed4me",
            EpisodeProvider::Voe => "Voe",
            _ => "Unknown",
        };
        write!(f, "{name}")
    }
}

impl EpisodeProvider {
    fn from_url(url: &str) -> Self {
        const HOSTS: &[(&str, EpisodeProvider)] = &[
            ("ansembed.net", EpisodeProvider::Ansembed),
            ("uqload.", EpisodeProvider::Uqload),
            ("sendvid.com", EpisodeProvider::Sendvid),
            ("embed4me.com", EpisodeProvider::Embed4me),
            ("teresapoliticallearn.com", EpisodeProvider::Voe),
        ];

        HOSTS
            .iter()
            .find(|(host, _)| url.contains(host))
            .map_or(EpisodeProvider::Unknown, |&(_, p)| Clone::clone(&p))
    }
}

#[derive(Clone)]
pub struct Episode {
    name: String,
    url: String,
    provider: EpisodeProvider,
    status: DownloadStatus,
}

#[derive(Clone)]
pub enum DownloadStatus {
    Pending,
    Running {
        percent: f32,
        speed: String,
        path: PathBuf,
        eta: String,
    },
    Done,
    Failed,
}

#[derive(Clone)]
pub struct EpisodePlayer {
    name: String,
    episodes: Vec<Episode>,
}

pub enum AnimeSamaPageStep {
    Search(SearchStepState),
    SelectSeason(SelectSeasonStepState),
    SelectLang(SelectLangStepState),
    SelectEpisodes(SelectEpisodesStepState),
    Downloads(DownloadsStepState),
}

pub enum AnimeSamaEvent {
    Search {
        poll: bool,
        search_result: Option<Vec<AnimeResult>>,
        search_error_message: String,
    },

    Seasons {
        poll: bool,
        fetch_seasons_result: Option<Vec<SeasonResult>>,
        fetch_seasons_error_message: String,
    },

    Langs {
        poll: bool,
        fetch_langs_result: Option<Vec<LangResult>>,
        fetch_langs_error_message: String,
    },

    Episodes {
        poll: bool,
        fetch_episodes_result: Option<Vec<EpisodePlayer>>,
        fetch_episodes_error_message: String,
    },

    Downloads {
        current_episode: Option<Episode>,
    },
}

pub struct AnimeSamaPage {
    pub state: PageState,
    tx: mpsc::Sender<ThreadEvent>,
    current_step: AnimeSamaPageStep,
}

impl AnimeSamaPage {
    // New
    pub fn new(tx: mpsc::Sender<ThreadEvent>) -> Self {
        Self {
            tx,
            current_step: AnimeSamaPageStep::Search(SearchStepState {
                input: Input::default(),
                throbber_state: ThrobberState::default(),
                result_list_state: ListState::default().with_selected(Some(0)),
                result_list_items: Vec::new(),
                last_search_error_message: String::new(),
                is_searching: false,
            }),
            state: PageState {
                title: " AnimeSama ".into(),
                full_path: vec!["Search".into()],
                input_mode: InputMode::Normal,
                helps: vec![
                    ("[j/k]".into(), "move".into()),
                    ("[r]".into(), "refresh".into()),
                    ("[/]".into(), "insert mode".into()),
                    ("[escape]".into(), "normal mode".into()),
                    ("[enter]".into(), "confirm".into()),
                ],
            },
        }
    }

    /// Handle views
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        let input_mode = self.state.input_mode;
        match &mut self.current_step {
            AnimeSamaPageStep::Search(s) => s.view(input_mode, area, frame),
            AnimeSamaPageStep::SelectSeason(s) => s.view(area, frame),
            AnimeSamaPageStep::SelectLang(s) => s.view(area, frame),
            AnimeSamaPageStep::SelectEpisodes(s) => s.view(input_mode, area, frame),
            AnimeSamaPageStep::Downloads(s) => s.view(area, frame),
        }
    }

    /// Handle crossterm raw events
    pub fn handle_crossterm_raw_event(&mut self, event: &Event) {
        let page_state = &mut self.state;
        let tx = self.tx.clone();
        let next_step = match &mut self.current_step {
            AnimeSamaPageStep::Search(s) => s.handle_crossterm_raw_event(page_state, tx, event),
            AnimeSamaPageStep::SelectEpisodes(s) => {
                s.handle_crossterm_raw_event(page_state, tx, event)
            }
            _ => None,
        };

        if let Some(next_step) = next_step {
            self.current_step = next_step;
        }
    }

    /// Handle crossterm keys
    pub fn handle_crossterm_key(&mut self, key_event: KeyEvent) {
        let page_state = &mut self.state;
        let tx = self.tx.clone();
        let next_step = match &mut self.current_step {
            AnimeSamaPageStep::Search(s) => s.handle_crossterm_key(page_state, tx, key_event),
            AnimeSamaPageStep::SelectSeason(s) => s.handle_crossterm_key(page_state, tx, key_event),
            AnimeSamaPageStep::SelectLang(s) => s.handle_crossterm_key(page_state, tx, key_event),
            AnimeSamaPageStep::SelectEpisodes(s) => {
                s.handle_crossterm_key(page_state, tx, key_event)
            }
            AnimeSamaPageStep::Downloads(s) => s.handle_crossterm_key(key_event),
        };

        if let Some(next_step) = next_step {
            self.current_step = next_step;
        }
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &AnimeSamaEvent) {
        match &mut self.current_step {
            AnimeSamaPageStep::Search(s) => s.handle_thread_event(event),
            AnimeSamaPageStep::SelectSeason(s) => s.handle_thread_event(event),
            AnimeSamaPageStep::SelectLang(s) => s.handle_thread_event(event),
            AnimeSamaPageStep::SelectEpisodes(s) => s.handle_thread_event(event),
            AnimeSamaPageStep::Downloads(s) => s.handle_thread_event(event),
        };
    }
}
