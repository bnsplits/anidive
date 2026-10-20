use ratatui::{Frame, layout::Rect, widgets::ListState};
use throbber_widgets_tui::ThrobberState;
use tui_input::Input;

use crate::pages::{InputMode, nyaa::NyaaResult};

pub struct DownloadsStepState {
    pub throbber_state: ThrobberState,
    pub result_list_state: ListState,
    pub result_list_items: Vec<NyaaResult>,
    pub last_search_error_message: String,
    pub is_searching: bool,
    pub input: Input,
}

impl DownloadsStepState {
    /// View
    pub fn view(&mut self, input_mode: InputMode, area: Rect, frame: &mut Frame) {}
}
