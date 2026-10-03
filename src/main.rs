mod constants;
mod providers;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Flex, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use std::io;

use crate::{constants::traits::AnimeProvider, providers::animesama::AnimeSamaState};

struct App {
    current_page: Box<dyn AnimeProvider>,
    do_close: bool,
}

impl AnimeProvider for SelectionPage {}
impl Default for SelectionPage {
    fn default() -> Self {
        Self {}
    }
}
struct SelectionPage {}

impl Default for App {
    fn default() -> Self {
        Self {
            current_page: Box::from(SelectionPage::default()),
            do_close: false,
        }
    }
}

impl App {
    /// Run function
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.do_close {
            self.view(terminal)?;
            self.update()?;
        }
        Ok(())
    }

    /// View function
    fn view(&self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        terminal.draw(|frame| {
            // Main layouts
            let main_vert_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(2),
                    Constraint::Fill(1),
                    Constraint::Length(2),
                ])
                .split(frame.area());

            let content_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(vec![Constraint::Percentage(75)])
                .flex(Flex::Center)
                .split(main_vert_layout[1]);

            // Main widgets
            let top_row = Paragraph::new("Top");
            frame.render_widget(top_row, main_vert_layout[0]);

            let content = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(Color::DarkGray));
            frame.render_widget(content, content_layout[0]);

            let bottom_row = Paragraph::new("Bottom");
            frame.render_widget(bottom_row, main_vert_layout[2]);
        })?;

        Ok(())
    }

    /// Update function
    fn update(&mut self) -> io::Result<()> {
        if let Event::Key(key_event) = crossterm::event::read()?
            && key_event.kind == KeyEventKind::Press
        {
            self.handle_key_event(key_event);
        }

        Ok(())
    }

    /// Handle key event function
    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('d') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.close();
            }
            _ => {}
        }
    }

    // Close the app with double check
    fn close(&mut self) {
        self.do_close = true;
    }
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}
