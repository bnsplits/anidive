use std::time::Duration;

use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, Paragraph},
};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::providers::{
    AnimeProviderState,
    InputMode::{self, Normal},
};

pub struct AnimeSamaProvider {
    pub state: AnimeProviderState,
    pub input: Input,
}

impl Default for AnimeSamaProvider {
    fn default() -> Self {
        Self {
            input: Input::default(),
            state: AnimeProviderState {
                title: " AnimeSama ".into(),
                full_path: vec!["".into()],
                can_change_input_mode: true,
                input_mode: InputMode::Normal,
                want_poll: false,
                poll_timeout: Some(Duration::from_millis(1000)),
                helps: vec![
                    ("[j/k]".into(), "move".into()),
                    ("[i]".into(), "insert mode".into()),
                    ("[escape]".into(), "normal mode".into()),
                    ("[enter]".into(), "confirm".into()),
                ],
            },
        }
    }
}

impl AnimeSamaProvider {
    pub fn view(&self, area: Rect, frame: &mut Frame) {
        let [search_bar_layout, _results_layout] =
            Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]).areas(area);
        let search_bar_layout = search_bar_layout
            .centered_horizontally(Constraint::Percentage(50))
            .centered_vertically(Constraint::Length(3));

        let width = search_bar_layout.width.max(2) - 3;
        let scroll = self.input.visual_scroll(width as usize);
        let style = match self.state.input_mode {
            InputMode::Normal => Style::default().dark_gray().dim(),
            InputMode::Insert => Style::default(),
        };
        let search_field_text = if self.input.value().is_empty()
            && let InputMode::Normal = self.state.input_mode
        {
            "Search anything..."
        } else {
            self.input.value()
        };
        let search_field = Paragraph::new(search_field_text)
            .block(Block::bordered())
            .style(style)
            .scroll((0, scroll as u16));

        frame.render_widget(search_field, search_bar_layout);
        if let InputMode::Insert = self.state.input_mode {
            frame.set_cursor_position((
                search_bar_layout.x + 1 + (self.input.visual_cursor().max(scroll) - scroll) as u16,
                search_bar_layout.y + 1,
            ));
        }
    }

    pub fn handle_raw_event(&mut self, event: &Event) {
        self.input.handle_event(event);
        if let Event::Key(k) = event
            && let KeyCode::Enter = k.code
        {
            self.state.input_mode = Normal;
        }
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) {
        //TODO
    }

    pub fn handle_tick(&mut self) {
        //TODO
    }
}
