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
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use throbber_widgets_tui::{Throbber, ThrobberState};
use tui_input::Input;

use crate::{
    ThreadEvent,
    pages::{
        InputMode, PageState,
        animesama::{
            AnimePageStep, AnimeSamaEvent, SeasonResult,
            select_episodes_step::SelectEpisodesStepState,
        },
    },
};

pub struct SelectLangStepState {
    pub throbber_state: ThrobberState,
    pub result_list_state: ListState,
    pub result_list_items: Vec<SeasonResult>,
    pub last_language_fetch_error_message: String,
    pub selected_season: SeasonResult,
    pub is_fetching: bool,
}

impl SelectLangStepState {
    /// View
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        let [instruction_layout, results_layout] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area);

        /* Instructions text */
        let instructions = Line::from("Select a language among the available ones")
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
            if self.last_language_fetch_error_message.is_empty() {
                frame.render_stateful_widget(list, results_layout, &mut self.result_list_state);
            } else {
                let msg = format!("Oops : {}", self.last_language_fetch_error_message);
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

                let selected_lang =
                    self.result_list_items[self.result_list_state.selected().unwrap()].clone();

                if let Some(last) = page_state.full_path.last_mut() {
                    *last = selected_lang.0.clone();
                }

                page_state.input_mode = InputMode::Normal;
                page_state.full_path.push("Episodes".into());
                page_state.helps = vec![
                    ("[h/j/k/l]".into(), "move".into()),
                    ("[/]".into(), "insert mode".into()),
                    ("[escape]".into(), "normal mode".into()),
                    ("[enter]".into(), "confirm".into()),
                ];

                return Some(AnimePageStep::SelectEpisodes({
                    let mut episodes_state = SelectEpisodesStepState {
                        throbber_state: ThrobberState::default(),
                        preview_list_state: ListState::default().with_selected(Some(0)),
                        result_list_items: Vec::new(),
                        last_episodes_fetch_error_message: String::new(),
                        is_fetching: false,
                        selected_player_index: 0,
                        input: Input::default(),
                        selected_lang,
                    };
                    episodes_state.fetch_episodes(tx);
                    episodes_state
                }));
            }

            _ => {}
        }

        None
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &AnimeSamaEvent) {
        if let AnimeSamaEvent::Langs {
            poll,
            fetch_langs_result,
            fetch_langs_error_message,
        } = event
        {
            if *poll {
                self.throbber_state.calc_next();
            }

            if fetch_langs_result.is_some() {
                self.result_list_items = fetch_langs_result.clone().unwrap();
                self.result_list_state.select(Some(0));
                self.is_fetching = false;
            }

            if !fetch_langs_error_message.is_empty() {
                self.last_language_fetch_error_message = fetch_langs_error_message.clone();
                self.is_fetching = false;
            }
        }
    }

    /// Fetch langs
    pub fn fetch_langs(&mut self, tx: mpsc::Sender<ThreadEvent>) {
        self.is_fetching = true;
        self.last_language_fetch_error_message = String::new();

        let (done_tx, done_rx) = mpsc::channel::<()>();

        /* Make the Throbber spin in another thread */
        let poll_tx = tx.clone();
        thread::spawn(move || {
            while done_rx.try_recv().is_err() {
                thread::sleep(Duration::from_millis(100));
                poll_tx
                    .send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Langs {
                        poll: true,
                        fetch_langs_result: None,
                        fetch_langs_error_message: String::new(),
                    }))
                    .unwrap();
            }
        });

        /* Perform the actual fetch on another thread */
        let fetch_tx = tx.clone();
        let selected_season = self.selected_season.clone();
        thread::spawn(move || {
            let url = selected_season.1.clone();
            let client = Client::new();

            let res = client
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
                let document = Html::parse_document(res);
                let a_sel = Selector::parse(r#"a[id^="switch"]"#).unwrap();

                let items = document
                    .select(&a_sel)
                    .filter_map(|a| {
                        let id = a
                            .value()
                            .attr("id")
                            .and_then(|a| a.strip_prefix("switch"))
                            .unwrap_or_default();

                        let href = format!(
                            "{}/{}",
                            url
                                /* Remove a possible triming '/' */
                                .trim_end_matches('/')
                                /* Remove the original /lang at the end */
                                .rsplit_once('/')
                                .unwrap()
                                .0,
                            a.value()
                                .attr("href")
                                .and_then(|a| a.strip_prefix("../"))
                                .unwrap_or_default(),
                        );

                        /* Return the lang only if it actually exists */
                        client
                            .head(&href)
                            .send()
                            .ok()?
                            .status()
                            .is_success()
                            .then_some((id.to_string(), href))
                    })
                    .collect();

                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Langs {
                    poll: false,
                    fetch_langs_result: Some(items),
                    fetch_langs_error_message: String::new(),
                }));
            }

            if let Err(e) = &res {
                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Langs {
                    poll: false,
                    fetch_langs_result: None,
                    fetch_langs_error_message: e.to_string(),
                }));
            }

            let _ = done_tx.send(());
        });
    }
}
