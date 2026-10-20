pub mod downloads_step;
pub mod search_step;

use std::sync::mpsc;

use crossterm::event::{Event, KeyEvent};
use ratatui::{
    Frame,
    layout::Rect,
    text::Line,
    widgets::{Paragraph, Widget},
};

use crate::{
    ThreadEvent,
    pages::{
        InputMode, PageState,
        nyaa::{downloads_step::DownloadsStepState, search_step::SearchStepState},
    },
};

pub enum NyaaPageStep {
    Search(SearchStepState),
    Downloads(DownloadsStepState),
}

#[derive(Default)]
pub struct NyaaEvent {}

pub struct NyaaResult {}

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
                throbber_state: todo!(),
                result_list_state: todo!(),
                result_list_items: todo!(),
                last_search_error_message: todo!(),
                is_searching: todo!(),
                input: todo!(),
            }),
            state: PageState {
                title: " Nyaa.si ".into(),
                full_path: vec!["Search".into()],
                input_mode: InputMode::Normal,
                helps: vec![],
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
    pub fn handle_crossterm_raw_event(&mut self, event: &Event) {}

    /// Handle crossterm keys
    pub fn handle_crossterm_key(&mut self, key_event: KeyEvent) {}

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &NyaaEvent) {}
}
