pub mod downloads_step;
pub mod search_step;

use std::sync::mpsc;

use crossterm::event::{Event, KeyEvent};
use ratatui::{Frame, layout::Rect, widgets::ListState};
use throbber_widgets_tui::ThrobberState;
use tui_input::Input;

use crate::{
    ThreadEvent,
    pages::{
        InputMode, PageState,
        nyaa::{downloads_step::DownloadsStepState, search_step::SearchStepState},
    },
};

type NyaaResult = (String, String, String, String, String, String, String); // name, magnet_link, size, date, seeders, leechers, downloads
type NyaaResultPagination = (u32, bool, String); // number, is_active, full_href

#[derive(Clone)]
pub enum NyaaDownloadStatus {
    FetchingMetadata,
    Running {
        percent: f32,
        speed: String,
        eta: String,
    },
    Done,
    Failed,
}

pub enum NyaaPageStep {
    Search(SearchStepState),
    Downloads(DownloadsStepState),
}

pub enum NyaaEvent {
    Search {
        poll: bool,
        search_result: Option<Vec<NyaaResult>>,
        pagination: Option<Vec<NyaaResultPagination>>,
        search_error_message: String,
    },

    Downloads {
        status: NyaaDownloadStatus,
    },
}

pub struct NyaaPage {
    pub state: PageState,
    tx: mpsc::Sender<ThreadEvent>,
    current_step: NyaaPageStep,
}

impl NyaaPage {
    // New
    pub fn new(tx: mpsc::Sender<ThreadEvent>) -> Self {
        Self {
            tx,
            current_step: NyaaPageStep::Search(SearchStepState {
                throbber_state: ThrobberState::default(),
                result_list_state: ListState::default().with_selected(Some(0)),
                result_list_items: Vec::new(),
                pagination: Vec::new(),
                selected_page_index: 0,
                last_search_error_message: String::new(),
                is_searching: false,
                input: Input::default(),
            }),
            state: PageState {
                title: " Nyaa.si ".into(),
                full_path: vec!["Search".into()],
                input_mode: InputMode::Normal,
                helps: vec![
                    ("[j/k]".into(), "move".into()),
                    ("[h/l]".into(), "page".into()),
                    ("[1-9]".into(), "jump page".into()),
                    ("[r]".into(), "refresh".into()),
                    ("[/]".into(), "insert mode".into()),
                    ("[escape]".into(), "normal mode".into()),
                    ("[enter]".into(), "download".into()),
                ],
            },
        }
    }

    /// Handle views
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        let input_mode = self.state.input_mode;
        match &mut self.current_step {
            NyaaPageStep::Search(s) => s.view(input_mode, area, frame),
            NyaaPageStep::Downloads(s) => s.view(input_mode, area, frame),
        }
    }

    /// Handle crossterm raw events
    pub fn handle_crossterm_raw_event(&mut self, event: &Event) {
        let page_state = &mut self.state;
        let tx = self.tx.clone();
        let next_step = match &mut self.current_step {
            NyaaPageStep::Search(s) => s.handle_crossterm_raw_event(page_state, tx, event),
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
            NyaaPageStep::Search(s) => s.handle_crossterm_key(page_state, tx, key_event),
            NyaaPageStep::Downloads(s) => s.handle_crossterm_key(key_event),
        };

        if let Some(next_step) = next_step {
            self.current_step = next_step;
        }
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &NyaaEvent) {
        match &mut self.current_step {
            NyaaPageStep::Search(s) => s.handle_thread_event(event),
            NyaaPageStep::Downloads(s) => s.handle_thread_event(event),
        }
    }
}
