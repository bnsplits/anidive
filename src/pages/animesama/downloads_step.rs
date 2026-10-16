use std::sync::mpsc;

use ratatui::{Frame, layout::Rect};

use crate::{
    ThreadEvent,
    pages::animesama::{AnimeSamaEvent, Episode},
};

pub struct DownloadsStepState {
    pub selected_episodes: Vec<Episode>,
}

impl DownloadsStepState {
    /// View
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {}

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &AnimeSamaEvent) {}

    /// Start downloads
    pub fn start_downlaods(&mut self, tx: mpsc::Sender<ThreadEvent>) {}
}
