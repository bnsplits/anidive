mod providers;

use crossterm::event::{Event, KeyCode, KeyModifiers};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
};
use std::io;

use crate::{
    providers::AnimeProviderNames, providers::AnimeProviders,
    providers::animesama::AnimeSamaProvider, providers::home::HomeProvider,
};

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
    fn view(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        terminal.draw(|frame| {
            let state = self.current_page.state();

            // Main vertical layout
            let [top_bar_layout, content_layout, bottom_bar_layout] = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(1),
                    Constraint::Fill(1),
                    Constraint::Length(1),
                ])
                .areas(frame.area());

            // Content layout
            let content_layout = content_layout.centered_horizontally(Constraint::Percentage(75));

            // Center content
            let content_block = Block::bordered()
                .title(
                    Line::from(String::from(&state.title))
                        .style(Style::new().fg(Color::Gray))
                        .centered(),
                )
                .padding(Padding::uniform(1))
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
            let helps = [
                vec![("[ctrl+d]".into(), "exit".into())],
                state.helps.clone(),
            ]
            .concat();
            for (key, desc) in helps {
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

            frame.render_widget(top_bar, top_bar_layout);
            frame.render_widget(bottom_bar, bottom_bar_layout);
            frame.render_widget(content_block, content_layout);

            self.current_page.render(content_block_inner, frame);
        })?;

        Ok(())
    }

    /// Update function
    fn update(&mut self) -> io::Result<()> {
        // Set the new current page if home have selected one
        if let AnimeProviders::HomePage(home) = &self.current_page
            && let Some(selected_page_name) = &home.selected_provider
        {
            match selected_page_name {
                AnimeProviderNames::AnimeSama => {
                    self.current_page = AnimeProviders::AnimeSama(AnimeSamaProvider::default())
                }
            }
            return Ok(());
        }

        let state = self.current_page.state();

        if state.want_poll
            && let Some(timeout) = state.poll_timeout
        {
            if crossterm::event::poll(timeout)? {
                self._handle_event()?
            }
            self.current_page.handle_tick();
        }
        self._handle_event()
    }

    fn _handle_event(&mut self) -> io::Result<()> {
        if let Event::Key(key_event) = crossterm::event::read()? {
            if key_event.code == KeyCode::Char('d') && key_event.modifiers == KeyModifiers::CONTROL
            {
                self.do_close = true;
                return Ok(());
            }
            self.current_page.handle_key(key_event);
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}
