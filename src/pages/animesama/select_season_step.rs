use std::{
    sync::mpsc::{self, Sender},
    thread,
    time::Duration,
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{List, ListItem, ListState},
};
use regex::Regex;
use reqwest::blocking::Client;
use throbber_widgets_tui::{Throbber, ThrobberState};

use crate::{
    ThreadEvent,
    pages::{
        InputMode, PageState,
        animesama::{
            AnimePageStep, AnimeResult, AnimeSamaEvent, SeasonResult,
            select_lang_step::SelectLangStepState,
        },
    },
};

pub struct SelectSeasonStepState {
    pub throbber_state: ThrobberState,
    pub result_list_state: ListState,
    pub result_list_items: Vec<SeasonResult>,
    pub last_season_fetch_error_message: String,
    pub selected_anime: AnimeResult,
    pub is_fetching: bool,
}

impl SelectSeasonStepState {
    /// View
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        let [instruction_layout, results_layout] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area);

        /* Instructions text */
        let instructions = Line::from("Select a season among the available ones")
            .centered()
            .dark_gray()
            .italic();
        frame.render_widget(instructions, instruction_layout);

        /* Throbber */
        let throbber = Throbber::default()
            .label("Please wait...")
            .style(Style::default().fg(Color::Gray))
            .throbber_style(Style::default().fg(Color::Gray).bold())
            .throbber_set(throbber_widgets_tui::BRAILLE_SIX)
            .use_type(throbber_widgets_tui::WhichUse::Spin);

        /* Result list */
        let list_items = self.result_list_items.iter().map(|item| {
            ListItem::new(Line::from(vec![
                Span::raw(format!("[{}]", &item.0)).yellow().bold(),
            ]))
            .dim()
        });
        let list = List::new(list_items)
            .highlight_style(Style::new().not_dim())
            .highlight_symbol("> ".light_cyan())
            .scroll_padding(1);

        /* Rendering */
        if self.is_fetching {
            frame.render_stateful_widget(throbber, results_layout, &mut self.throbber_state);
        } else {
            if self.last_season_fetch_error_message.is_empty() {
                frame.render_stateful_widget(list, results_layout, &mut self.result_list_state);
            } else {
                let msg = format!("Oops : {}", self.last_season_fetch_error_message);
                let results_layout =
                    results_layout.centered_horizontally(Constraint::Length(msg.len() as u16));
                frame.render_widget(Line::from(msg).centered().light_red(), results_layout);
            }
        }
    }

    /// Handle crossterm keys
    pub fn handle_crossterm_key(
        &mut self,
        page_state: &mut PageState,
        tx: Sender<ThreadEvent>,
        key_event: KeyEvent,
    ) -> Option<AnimePageStep> {
        match key_event.code {
            KeyCode::Char('r') => {
                self.fetch_seasons(tx);
            }

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

                let selected_season =
                    self.result_list_items[self.result_list_state.selected().unwrap()].clone();

                if let Some(last) = page_state.full_path.last_mut() {
                    *last = selected_season.0.clone();
                }

                page_state.input_mode = InputMode::None;
                page_state.full_path.push("Language".into());
                page_state.helps = vec![
                    ("[j/k]".into(), "move".into()),
                    ("[r]".into(), "refresh".into()),
                    ("[enter]".into(), "confirm".into()),
                ];

                return Some(AnimePageStep::SelectLang({
                    let mut lang_state = SelectLangStepState {
                        throbber_state: ThrobberState::default(),
                        result_list_state: ListState::default().with_selected(Some(0)),
                        result_list_items: Vec::new(),
                        last_language_fetch_error_message: String::new(),
                        is_fetching: false,
                        selected_season,
                    };
                    lang_state.fetch_langs(tx);
                    lang_state
                }));
            }

            _ => {}
        }

        None
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &AnimeSamaEvent) {
        if let AnimeSamaEvent::Seasons {
            poll,
            fetch_seasons_result,
            fetch_seasons_error_message,
        } = event
        {
            if *poll {
                self.throbber_state.calc_next();
            }

            if fetch_seasons_result.is_some() {
                self.result_list_items = fetch_seasons_result.clone().unwrap();
                self.result_list_state.select(Some(0));
                self.is_fetching = false;
            }

            if !fetch_seasons_error_message.is_empty() {
                self.last_season_fetch_error_message = fetch_seasons_error_message.clone();
                self.is_fetching = false;
            }
        }
    }

    /// Fetch seasons
    pub fn fetch_seasons(&mut self, tx: mpsc::Sender<ThreadEvent>) {
        if self.is_fetching {
            return;
        }

        self.is_fetching = true;
        self.last_season_fetch_error_message = String::new();

        let (done_tx, done_rx) = mpsc::channel::<()>();

        /* Make the Throbber spin in another thread */
        let poll_tx = tx.clone();
        thread::spawn(move || {
            while done_rx.try_recv().is_err() {
                thread::sleep(Duration::from_millis(100));
                poll_tx
                    .send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Seasons {
                        poll: true,
                        fetch_seasons_result: None,
                        fetch_seasons_error_message: String::new(),
                    }))
                    .unwrap();
            }
        });

        /* Perform the actual fetch on another thread */
        let fetch_tx = tx.clone();
        let selected_anime = self.selected_anime.clone();
        thread::spawn(move || {
            let url = selected_anime.2.clone();
            let res = Client::new()
                .get(&url)
                .send()
                .map_err(|_e| "network error".into())
                .and_then(|r| {
                    r.error_for_status().map_err(|e| {
                        format!("bad status: {}", e.status().map_or(0, |s| s.as_u16()))
                    })
                })
                .and_then(|r| r.text().map_err(|_e| "could not read result body".into()));

            if let Ok(res) = &res {
                /* Target lines that looks like that : panneauAnime("Saison 1", "saison1/vostfr"); */
                let re_comment = Regex::new(r"(?s)/\*.*?\*/|//[^\n]*").unwrap();
                let clean_res = re_comment.replace_all(res, "");
                let re = Regex::new(r#"panneauAnime\("([^"]*)",\s*"([^"]*)"\)"#).unwrap();

                let items = re
                    .captures_iter(&clean_res)
                    .map(|c| {
                        (
                            c[1].to_string(),
                            format!("{}/{}", url.trim_end_matches('/'), &c[2]),
                        )
                    })
                    .collect();

                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Seasons {
                    poll: false,
                    fetch_seasons_result: Some(items),
                    fetch_seasons_error_message: String::new(),
                }));
            }

            if let Err(e) = &res {
                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Seasons {
                    poll: false,
                    fetch_seasons_result: None,
                    fetch_seasons_error_message: e.to_string(),
                }));
            }

            let _ = done_tx.send(());
        });
    }
}
