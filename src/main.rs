mod constants;
mod providers;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use std::io;

use crate::{constants::enums::AnimeProviders, providers::home::HomeProvider};

struct App {
    current_page: AnimeProviders,
    do_close: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            current_page: AnimeProviders::HomePage(HomeProvider::default()),
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
            let state = self.current_page.state();

            // Main vertical layout
            let main_vert_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(1),
                    Constraint::Fill(1),
                    Constraint::Length(1),
                ])
                .split(frame.area());

            // Content layout
            let content_layout =
                main_vert_layout[1].centered_horizontally(Constraint::Percentage(75));

            // Center content
            let content_block = Block::bordered()
                .title(
                    Line::from(String::from(&state.name))
                        .style(Style::new().fg(Color::Cyan))
                        .centered(),
                )
                .border_style(Style::new().fg(Color::DarkGray));
            let content_block_inner = content_block.inner(content_layout);

            // Top bar
            let mut path_line = Vec::new();
            for p in &state.full_path {
                path_line.push(Span::styled("/", Style::new().fg(Color::DarkGray)));
                path_line.push(Span::styled(
                    p,
                    Style::new().fg(Color::White).add_modifier(Modifier::BOLD),
                ));
            }
            let top_bar = Paragraph::new(Line::from(path_line));

            // Bottom bar
            let mut help_line = Vec::new();
            for (key, desc) in &state.helps {
                help_line.push(Span::styled(
                    format!("{} ", key),
                    Style::new().fg(Color::Gray),
                ));
                help_line.push(Span::styled(
                    format!("{} ", desc),
                    Style::new().fg(Color::DarkGray),
                ));
                help_line.push(Span::styled(" ", Style::new().fg(Color::Blue)));
            }
            help_line.pop();
            let bottom_bar = Paragraph::new(Line::from(help_line));

            frame.render_widget(top_bar, main_vert_layout[0]);
            frame.render_widget(bottom_bar, main_vert_layout[2]);
            frame.render_widget(content_block, content_layout);

            self.current_page.render(content_block_inner, frame);
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
