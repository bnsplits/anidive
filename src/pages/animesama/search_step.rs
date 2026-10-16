use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::{Block, List, ListItem, ListState, Paragraph, Wrap},
};
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};
use throbber_widgets_tui::{Throbber, ThrobberState};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    ThreadEvent,
    pages::{
        InputMode, PageState,
        animesama::{
            AnimePageStep, AnimeResult, AnimeSamaEvent, select_season_step::SelectSeasonStepState,
        },
    },
};

pub struct SearchStepState {
    pub throbber_state: ThrobberState,
    pub result_list_state: ListState,
    pub result_list_items: Vec<AnimeResult>,
    pub last_search_error_message: String,
    pub is_searching: bool,
    pub input: Input,
}

impl SearchStepState {
    /// View
    pub fn view(&mut self, input_mode: InputMode, area: Rect, frame: &mut Frame) {
        let [search_bar_layout, results_layout, description_layout] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(5),
            Constraint::Fill(1),
        ])
        .margin(1)
        .spacing(2)
        .areas(area);
        let search_bar_layout = search_bar_layout.centered_horizontally(Constraint::Percentage(50));

        let width = search_bar_layout.width.max(2) - 3;
        let scroll = self.input.visual_scroll(width as usize);

        /* Search field */
        let search_field_text = if self.input.value().is_empty()
            && let InputMode::Normal = input_mode
        {
            "Search anything..."
        } else {
            self.input.value()
        };
        let search_field_style = match input_mode {
            InputMode::Normal | InputMode::None => Style::default().dark_gray(),
            InputMode::Insert => Style::default(),
        };
        let search_field = Paragraph::new(search_field_text)
            .block(Block::bordered())
            .style(search_field_style)
            .scroll((0, scroll as u16));

        frame.render_widget(search_field, search_bar_layout);
        if let InputMode::Insert = input_mode {
            frame.set_cursor_position((
                search_bar_layout.x + 1 + (self.input.visual_cursor().max(scroll) - scroll) as u16,
                search_bar_layout.y + 1,
            ));
        }

        /* Throbber */
        let throbber = Throbber::default()
            .label("Searching...")
            .style(Style::default().fg(Color::Gray))
            .throbber_style(Style::default().fg(Color::Gray).bold())
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX)
            .use_type(throbber_widgets_tui::WhichUse::Spin);

        /* Result list */
        let list_items = self.result_list_items.iter().map(|item| {
            ListItem::new(format!("[{}]", &item.0))
                .yellow()
                .bold()
                .dim()
        });
        let list = List::new(list_items)
            .highlight_style(Style::new().not_dim())
            .highlight_symbol("> ".light_cyan())
            .scroll_padding(1);

        /* Rendering */
        if self.is_searching {
            let results_layout = results_layout.centered_horizontally(Constraint::Length(
                throbber.to_line(&self.throbber_state).width() as u16,
            ));
            frame.render_stateful_widget(throbber, results_layout, &mut self.throbber_state);
        } else {
            if self.last_search_error_message.is_empty() {
                if !self.result_list_items.is_empty()
                    && let Some(index) = self.result_list_state.selected()
                {
                    let description = self.result_list_items[index].1.clone();
                    frame.render_widget(
                        Paragraph::new(if description.is_empty() {
                            "No description ..."
                        } else {
                            &description
                        })
                        .wrap(Wrap { trim: true })
                        .dark_gray(),
                        description_layout,
                    );
                }

                frame.render_stateful_widget(list, results_layout, &mut self.result_list_state);
            } else {
                frame.render_widget(
                    Line::from(format!("Oops : {}", self.last_search_error_message))
                        .centered()
                        .light_red(),
                    results_layout,
                );
            }
        }
    }

    /// Handle crossterm raw events
    pub fn handle_crossterm_raw_event(
        &mut self,
        page_state: &mut PageState,
        tx: Sender<ThreadEvent>,
        event: &Event,
    ) -> Option<AnimePageStep> {
        if self.is_searching {
            return None;
        }

        self.input.handle_event(event);
        if let Event::Key(k) = event {
            match k.code {
                /* Perform search */
                KeyCode::Enter => {
                    page_state.input_mode = InputMode::Normal;
                    self.search(tx);
                }

                /* Exit input mode */
                KeyCode::Esc => page_state.input_mode = InputMode::Normal,

                _ => {}
            }
        }

        None
    }

    /// Handle crossterm keys
    pub fn handle_crossterm_key(
        &mut self,
        page_state: &mut PageState,
        tx: Sender<ThreadEvent>,
        key_event: KeyEvent,
    ) -> Option<AnimePageStep> {
        match key_event.code {
            KeyCode::Char('j') => {
                if let Some(selected_index) = self.result_list_state.selected() {
                    let len = self.result_list_items.len();
                    if len == 0 {
                        return None;
                    }
                    let next_index = (selected_index + 1) % (len);
                    self.result_list_state.select(Some(next_index));
                }
            }

            KeyCode::Char('k') => {
                if let Some(selected_index) = self.result_list_state.selected() {
                    let len = self.result_list_items.len();
                    if len == 0 {
                        return None;
                    }
                    let previous_index = (selected_index + len - 1) % len;
                    self.result_list_state.select(Some(previous_index));
                }
            }

            KeyCode::Enter => {
                if self.result_list_items.is_empty() {
                    return None;
                }

                let selected_anime =
                    self.result_list_items[self.result_list_state.selected().unwrap()].clone();

                if let Some(last) = page_state.full_path.last_mut() {
                    *last = selected_anime.0.clone();
                }

                page_state.input_mode = InputMode::None;
                page_state.full_path.push("Season".into());
                page_state.helps = vec![
                    ("[j/k]".into(), "move".into()),
                    ("[enter]".into(), "confirm".into()),
                ];

                return Some(AnimePageStep::SelectSeason({
                    let mut season_state = SelectSeasonStepState {
                        throbber_state: ThrobberState::default(),
                        result_list_state: ListState::default().with_selected(Some(0)),
                        result_list_items: Vec::new(),
                        last_season_fetch_error_message: String::new(),
                        is_fetching: false,
                        selected_anime,
                    };
                    season_state.fetch_seasons(tx);
                    season_state
                }));
            }

            _ => {}
        }

        None
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &AnimeSamaEvent) {
        if let AnimeSamaEvent::Search {
            poll,
            search_result,
            search_error_message,
        } = event
        {
            if *poll {
                self.throbber_state.calc_next();
            }

            if search_result.is_some() {
                self.result_list_items = search_result.clone().unwrap();
                self.result_list_state.select(Some(0));
                self.is_searching = false;
            }

            if !search_error_message.is_empty() {
                self.last_search_error_message = search_error_message.clone();
                self.is_searching = false;
            }
        }
    }

    /// Search
    pub fn search(&mut self, tx: mpsc::Sender<ThreadEvent>) {
        self.is_searching = true;
        self.last_search_error_message = String::new();

        let (done_tx, done_rx) = mpsc::channel::<()>();

        /* Make the Throbber spin in another thread */
        let poll_tx = tx.clone();
        thread::spawn(move || {
            while done_rx.try_recv().is_err() {
                thread::sleep(Duration::from_millis(100));
                poll_tx
                    .send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Search {
                        poll: true,
                        search_result: None,
                        search_error_message: String::new(),
                    }))
                    .unwrap();
            }
        });

        /* Perform the actual search on another thread */
        let query_tx = tx.clone();
        let search_text: String = self.input.value().into();
        thread::spawn(move || {
            let res = Client::new()
                .post("https://anime-sama.to/template-php/defaut/fetch.php")
                .form(&[("query", search_text)])
                .send()
                .map_err(|_e| "network error".into())
                .and_then(|r| {
                    r.error_for_status().map_err(|e| {
                        format!("bad status: {}", e.status().map_or(0, |s| s.as_u16()))
                    })
                })
                .and_then(|r| r.text().map_err(|_e| "could not read result body".into()));

            if let Ok(res) = &res {
                let fragment = Html::parse_fragment(res);

                let div_sel = Selector::parse("a div").unwrap();
                let h3_sel = Selector::parse("h3").unwrap();
                let p_sel = Selector::parse("p").unwrap();

                let items = fragment
                    .select(&div_sel)
                    .filter_map(|div| {
                        let link = div
                            .parent()
                            .and_then(ElementRef::wrap)
                            .and_then(|a| a.value().attr("href"))
                            .unwrap_or_default()
                            .to_string();
                        let title = div
                            .select(&h3_sel)
                            .next()?
                            .text()
                            .collect::<String>()
                            .trim()
                            .to_string();
                        let desc = div
                            .select(&p_sel)
                            .next()?
                            .text()
                            .collect::<String>()
                            .trim()
                            .to_string();
                        Some((title, desc, link))
                    })
                    .collect();

                let _ = query_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Search {
                    poll: false,
                    search_result: Some(items),
                    search_error_message: String::new(),
                }));
            }

            if let Err(e) = &res {
                let _ = query_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Search {
                    poll: false,
                    search_result: None,
                    search_error_message: e.to_string(),
                }));
            }

            let _ = done_tx.send(());
        });
    }
}
