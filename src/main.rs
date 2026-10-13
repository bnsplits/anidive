mod pages;

use crossterm::{
    cursor::SetCursorStyle,
    event::{Event, KeyCode, KeyModifiers},
    execute,
};
use ratatui::{
    DefaultTerminal,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Padding, Paragraph},
};
use std::{
    io::{self, stdout},
    sync::mpsc,
    thread,
};

use crate::{
    ThreadEvent::CrosstermEvent,
    pages::{
        AnimePageNames, AnimePages, InputMode,
        animesama::{self, AnimeSamaPage},
        home::HomePage,
        nyaa::{self, NyaaPage},
    },
};

pub enum ThreadEvent {
    CrosstermEvent(crossterm::event::Event),
    AnimeSamaEvent(animesama::AnimeSamaEvent),
    NyaaEvent(nyaa::NyaaEvent),
}

struct App {
    current_page: AnimePages,
    do_close: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            current_page: AnimePages::HomePage(HomePage::default()),
            do_close: false,
        }
    }
}

impl App {
    /// Run function
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let (tx, rx) = mpsc::channel::<ThreadEvent>();

        /* Thread that catchs crossterm events */
        let crossterm_tx = tx.clone();
        thread::spawn(move || {
            loop {
                let crossterm_event = crossterm::event::read().unwrap();
                crossterm_tx
                    .send(ThreadEvent::CrosstermEvent(crossterm_event))
                    .unwrap();
            }
        });

        while !self.do_close {
            self.view(terminal)?;
            self.update(tx.clone(), &rx);
        }

        Ok(())
    }

    /// View function
    fn view(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        terminal.draw(|frame| {
            let state = self.current_page.state();

            let [top_bar_layout, content_layout, bottom_bar_layout] = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![
                    Constraint::Length(1),
                    Constraint::Fill(1),
                    Constraint::Length(1),
                ])
                .areas(frame.area());

            /* Center content */
            let content_layout = content_layout.centered_horizontally(Constraint::Percentage(75));
            let content_block = Block::bordered()
                .title(
                    Line::from(String::from(&state.title))
                        .style(Style::new().fg(Color::Gray))
                        .centered(),
                )
                .padding(Padding::uniform(1))
                .border_style(Style::new().fg(Color::DarkGray));
            let content_block_inner = content_block.inner(content_layout);

            /* Top bar */
            let mut path_line = Vec::new();
            let full_path_len = state.full_path.len();
            for (i, p) in state.full_path.iter().enumerate() {
                path_line.push(Span::from("/").dark_gray());

                let is_last = i == full_path_len - 1;
                if is_last {
                    path_line.push(Span::from(format!(" {} ", p)).white().bold());
                } else {
                    path_line.push(Span::from(format!(" {} ", p)).dark_gray().bold());
                }
            }
            let top_bar = Paragraph::new(Line::from(path_line));

            /* Bottom bar */
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
    fn update(&mut self, tx: mpsc::Sender<ThreadEvent>, rx: &mpsc::Receiver<ThreadEvent>) {
        self.handle_event(rx);

        /* Set the new current page if home have selected one */
        if let AnimePages::HomePage(home) = &self.current_page
            && let Some(selected_page_name) = &home.selected_page
        {
            match selected_page_name {
                AnimePageNames::AnimeSama => {
                    self.current_page = AnimePages::AnimeSama(AnimeSamaPage::new(tx));
                }

                AnimePageNames::Nyaa => {
                    self.current_page = AnimePages::Nyaa(NyaaPage::new(tx));
                }
            }
        }
    }

    /// Handle all kind of events
    fn handle_event(&mut self, rx: &mpsc::Receiver<ThreadEvent>) {
        let rx_event = rx.recv().unwrap();

        if let CrosstermEvent(crossterm_event) = rx_event {
            self.handle_crossterm_event(crossterm_event);
        } else {
            self.current_page.handle_thread_event(&rx_event);
        }
    }

    /// Handle crossterm events
    fn handle_crossterm_event(&mut self, crossterm_event: Event) {
        if let Event::Key(key_event) = crossterm_event
            && key_event.code == KeyCode::Char('d')
            && key_event.modifiers == KeyModifiers::CONTROL
        {
            self.do_close = true;
            return;
        }

        if self.current_page.state().input_mode.is_some()
            && let InputMode::Insert = self.current_page.state().input_mode.unwrap()
        {
            self.current_page
                .handle_crossterm_raw_event(&crossterm_event);
            return;
        }

        if let Event::Key(key_event) = crossterm_event {
            if key_event.code == KeyCode::Char('d') && key_event.modifiers == KeyModifiers::CONTROL
            {
                self.do_close = true;
                return;
            }

            if self.current_page.state().input_mode.is_some() {
                if key_event.code == KeyCode::Char('/') {
                    self.current_page.state_mut().input_mode = Some(InputMode::Insert);
                }

                if key_event.code == KeyCode::Esc {
                    self.current_page.state_mut().input_mode = Some(InputMode::Normal);
                }
            }

            self.current_page.handle_crossterm_key(key_event);
        }
    }
}

fn main() -> io::Result<()> {
    execute!(stdout(), SetCursorStyle::SteadyBlock)?;
    ratatui::run(|terminal| App::default().run(terminal))?;
    execute!(stdout(), SetCursorStyle::DefaultUserShape)?;
    println!("Bye !");
    Ok(())
}
