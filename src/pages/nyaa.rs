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
    pages::{InputMode, PageState},
};

#[derive(Default)]
pub struct NyaaEvent {}

pub struct NyaaPage {
    pub state: PageState,
    tx: mpsc::Sender<ThreadEvent>,
}

impl NyaaPage {
    // New
    pub fn new(tx: mpsc::Sender<ThreadEvent>) -> Self {
        Self {
            tx,
            state: PageState {
                title: " Nyaa.si ".into(),
                full_path: vec!["Search".into()],
                input_mode: Some(InputMode::Normal),
                helps: vec![],
            },
        }
    }

    /// Handle views
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        Paragraph::new(Line::from("Page in construction ...").centered())
            .render(area, frame.buffer_mut());
    }

    /// Handle crossterm raw events
    pub fn handle_crossterm_raw_event(&mut self, event: &Event) {}

    /// Handle crossterm keys
    pub fn handle_crossterm_key(&mut self, key_event: KeyEvent) {}

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &NyaaEvent) {}
}
