use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Paragraph},
};
use throbber_widgets_tui::{Throbber, ThrobberState};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::pages::{
    InputMode::{self, Normal},
    PageState,
};

enum AnimePageStep {
    Search,
    SelectSeason,
    SelectLang,
    SelectEpisodes,
    Downloads,
}

pub struct AnimeSamaPage {
    pub state: PageState,
    pub input: Input,
    current_step: AnimePageStep,
    pub throbber_state: ThrobberState,
    pub is_searching: bool,
}

impl Default for AnimeSamaPage {
    fn default() -> Self {
        Self {
            input: Input::default(),
            throbber_state: ThrobberState::default(),
            is_searching: false,
            current_step: AnimePageStep::Search,
            state: PageState {
                title: " AnimeSama ".into(),
                full_path: vec!["".into()],
                can_change_input_mode: true,
                input_mode: InputMode::Normal,
                helps: vec![
                    ("[j/k]".into(), "move".into()),
                    ("[/]".into(), "insert mode".into()),
                    ("[escape]".into(), "normal mode".into()),
                    ("[enter]".into(), "confirm".into()),
                ],
            },
        }
    }
}

impl AnimeSamaPage {
    // View search step
    fn _search_step(&mut self, area: Rect, frame: &mut Frame) {
        let [search_bar_layout, results_layout] =
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

        let throbber = Throbber::default()
            .label("Searching...")
            .style(Style::default().fg(Color::Gray))
            .throbber_style(
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            )
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX)
            .use_type(throbber_widgets_tui::WhichUse::Spin);
        if self.is_searching {
            let results_layout = results_layout.centered_horizontally(Constraint::Length(
                throbber.to_line(&self.throbber_state).width() as u16,
            ));
            frame.render_stateful_widget(throbber, results_layout, &mut self.throbber_state);
        }
    }

    // View select season step
    fn _select_season_step(&mut self, area: Rect, frame: &mut Frame) {}

    // View select language step
    fn _select_lang_step(&mut self, area: Rect, frame: &mut Frame) {}

    // View select episodes step
    fn _select_episodes_step(&mut self, area: Rect, frame: &mut Frame) {}

    // View downloads step
    fn _downloads_step(&mut self, area: Rect, frame: &mut Frame) {}

    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        match self.current_step {
            AnimePageStep::Search => self._search_step(area, frame),
            AnimePageStep::SelectSeason => self._select_season_step(area, frame),
            AnimePageStep::SelectLang => self._select_lang_step(area, frame),
            AnimePageStep::SelectEpisodes => self._select_episodes_step(area, frame),
            AnimePageStep::Downloads => self._downloads_step(area, frame),
        }
    }

    pub fn handle_raw_event(&mut self, event: &Event) {
        self.input.handle_event(event);
        if let Event::Key(k) = event
            && let KeyCode::Enter = k.code
        {
            self.is_searching = true;
            self.state.input_mode = Normal;
        }
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) {
        //TODO
    }
}
