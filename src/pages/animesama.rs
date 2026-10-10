use std::{sync::mpsc, thread, time::Duration};

use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use throbber_widgets_tui::{Throbber, ThrobberState};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    ThreadEvent,
    pages::{
        InputMode::{self, Normal},
        PageState,
    },
};

struct SearchStepState {
    throbber_state: ThrobberState,
    result_list_state: ListState,
    result_list_items: Vec<(String, String)>,
    last_search_error_message: String,
    is_searching: bool,
    input: Input,
}

struct SelectSeasonStepState {
    is_fetching: bool,
}

struct SelectLangStepState {}

struct SelectEpisodesStepState {}

struct DownloadsStepState {}

enum AnimePageStep {
    Search(SearchStepState),
    SelectSeason(SelectSeasonStepState),
    SelectLang(SelectLangStepState),
    SelectEpisodes(SelectEpisodesStepState),
    Downloads(DownloadsStepState),
}

#[derive(Default)]
pub struct AnimeSamaEvent {
    poll: bool,
    search_result: Option<Vec<(String, String)>>,
    search_error_message: String,
}

pub struct AnimeSamaPage {
    pub state: PageState,
    tx: mpsc::Sender<ThreadEvent>,
    current_step: AnimePageStep,
}

impl AnimeSamaPage {
    // New
    pub fn new(tx: mpsc::Sender<ThreadEvent>) -> Self {
        Self {
            tx,
            current_step: AnimePageStep::Search(SearchStepState {
                input: Input::default(),
                throbber_state: ThrobberState::default(),
                result_list_state: ListState::default().with_selected(Some(0)),
                result_list_items: Vec::new(),
                last_search_error_message: String::new(),
                is_searching: false,
            }),
            state: PageState {
                title: " AnimeSama ".into(),
                full_path: vec!["Search".into()],
                input_mode: Some(InputMode::Normal),
                helps: vec![
                    ("[j/k]".into(), "move".into()),
                    ("[/]".into(), "insert mode".into()),
                    ("[escape]".into(), "normal mode".into()),
                    ("[enter]".into(), "confirm".into()),
                ],
            },
        }
    }

    // View search step
    fn search_step(&mut self, area: Rect, frame: &mut Frame) {
        if let AnimePageStep::Search(step_state) = &mut self.current_step {
            let [search_bar_layout, results_layout] =
                Layout::vertical([Constraint::Length(6), Constraint::Fill(1)]).areas(area);
            let search_bar_layout = search_bar_layout
                .centered_horizontally(Constraint::Percentage(50))
                .centered_vertically(Constraint::Length(3));

            let width = search_bar_layout.width.max(2) - 3;
            let scroll = step_state.input.visual_scroll(width as usize);

            /* Search field */
            let search_field_text = if step_state.input.value().is_empty()
                && let InputMode::Normal = self.state.input_mode.unwrap()
            {
                "Search anything..."
            } else {
                step_state.input.value()
            };
            let search_field_style = match self.state.input_mode.unwrap() {
                InputMode::Normal => Style::default().dark_gray(),
                InputMode::Insert => Style::default(),
            };
            let search_field = Paragraph::new(search_field_text)
                .block(Block::bordered())
                .style(search_field_style)
                .scroll((0, scroll as u16));

            frame.render_widget(search_field, search_bar_layout);
            if let InputMode::Insert = self.state.input_mode.unwrap() {
                frame.set_cursor_position((
                    search_bar_layout.x
                        + 1
                        + (step_state.input.visual_cursor().max(scroll) - scroll) as u16,
                    search_bar_layout.y + 1,
                ));
            }

            /* Throbber */
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

            /* Result list */
            let list_items = step_state.result_list_items.iter().map(|item| {
                ListItem::new(Line::from(vec![
                    Span::raw(format!("[{}]", &item.0)).yellow().bold(),
                    Span::raw(" "),
                    Span::raw(&item.1).white(),
                ]))
                .dim()
            });
            let list = List::new(list_items)
                .highlight_style(Style::new().not_dim())
                .highlight_symbol("> ".light_cyan())
                .scroll_padding(1);

            /* Rendering */
            if step_state.is_searching {
                let results_layout = results_layout.centered_horizontally(Constraint::Length(
                    throbber.to_line(&step_state.throbber_state).width() as u16,
                ));
                frame.render_stateful_widget(
                    throbber,
                    results_layout,
                    &mut step_state.throbber_state,
                );
            } else {
                if step_state.last_search_error_message.is_empty() {
                    frame.render_stateful_widget(
                        list,
                        results_layout,
                        &mut step_state.result_list_state,
                    );
                } else {
                    frame.render_widget(
                        Line::from(format!("Oops : {}", step_state.last_search_error_message))
                            .centered()
                            .light_red(),
                        results_layout,
                    );
                }
            }
        }
    }

    // View select season step
    fn select_season_step(&mut self, area: Rect, frame: &mut Frame) {}

    // View select language step
    fn select_lang_step(&mut self, area: Rect, frame: &mut Frame) {}

    // View select episodes step
    fn select_episodes_step(&mut self, area: Rect, frame: &mut Frame) {}

    // View downloads step
    fn downloads_page_step(&mut self, area: Rect, frame: &mut Frame) {}

    /// Handle views
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        match self.current_step {
            AnimePageStep::Search(_) => self.search_step(area, frame),
            AnimePageStep::SelectSeason(_) => self.select_season_step(area, frame),
            AnimePageStep::SelectLang(_) => self.select_lang_step(area, frame),
            AnimePageStep::SelectEpisodes(_) => self.select_episodes_step(area, frame),
            AnimePageStep::Downloads(_) => self.downloads_page_step(area, frame),
        }
    }

    /// Handle crossterm raw events
    pub fn handle_crossterm_raw_event(&mut self, event: &Event) {
        match &mut self.current_step {
            AnimePageStep::Search(step_state) => {
                if step_state.is_searching {
                    return;
                }

                step_state.input.handle_event(event);
                if let Event::Key(k) = event {
                    match k.code {
                        /* Perform search */
                        KeyCode::Enter => {
                            step_state.is_searching = true;
                            step_state.last_search_error_message = String::new();
                            self.state.input_mode = Some(Normal);

                            let (done_tx, done_rx) = mpsc::channel::<()>();

                            /* Make the Throbber spin in another thread */
                            let poll_tx = self.tx.clone();
                            thread::spawn(move || {
                                while done_rx.try_recv().is_err() {
                                    thread::sleep(Duration::from_millis(100));
                                    poll_tx
                                        .send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent {
                                            poll: true,
                                            ..Default::default()
                                        }))
                                        .unwrap();
                                }
                            });

                            /* Perform the actual search on another thread */
                            let query_tx = self.tx.clone();
                            let search_text: String = step_state.input.value().into();
                            thread::spawn(move || {
                                let res = Client::new()
                                    .post("https://anime-sama.to/template-php/defaut/fetch.php")
                                    .form(&[("query", search_text)])
                                    .send()
                                    .map_err(|_e| "network error".into())
                                    .and_then(|r| {
                                        r.error_for_status().map_err(|e| {
                                            format!(
                                                "bad status: {}",
                                                e.status().map_or(0, |s| s.as_u16())
                                            )
                                        })
                                    })
                                    .and_then(|r| {
                                        r.text().map_err(|_e| "could not read result body".into())
                                    });

                                if let Ok(res) = &res {
                                    let fragment = Html::parse_fragment(res);

                                    let div_sel = Selector::parse("a div").unwrap();
                                    let h3_sel = Selector::parse("h3").unwrap();
                                    let p_sel = Selector::parse("p").unwrap();

                                    let items = fragment
                                        .select(&div_sel)
                                        .filter_map(|div| {
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
                                            Some((title, desc))
                                        })
                                        .collect();

                                    let _ = query_tx.send(ThreadEvent::AnimeSamaEvent(
                                        AnimeSamaEvent {
                                            search_result: Some(items),
                                            ..Default::default()
                                        },
                                    ));
                                }

                                if let Err(e) = &res {
                                    let _ = query_tx.send(ThreadEvent::AnimeSamaEvent(
                                        AnimeSamaEvent {
                                            search_error_message: e.to_string(),
                                            ..Default::default()
                                        },
                                    ));
                                }

                                let _ = done_tx.send(());
                            });
                        }

                        /* Exit input mode */
                        KeyCode::Esc => self.state.input_mode = Some(InputMode::Normal),

                        _ => {}
                    }
                }
            }

            AnimePageStep::SelectSeason(step_state) => todo!(),
            AnimePageStep::SelectLang(step_state) => todo!(),
            AnimePageStep::SelectEpisodes(step_state) => todo!(),
            AnimePageStep::Downloads(step_state) => todo!(),
        }
    }

    /// Handle crossterm keys
    pub fn handle_crossterm_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('j') => match &mut self.current_step {
                AnimePageStep::Search(step_state) => {
                    if let Some(selected_index) = step_state.result_list_state.selected() {
                        let len = step_state.result_list_items.len();
                        let next_index = (selected_index + 1) % (len);
                        step_state.result_list_state.select(Some(next_index));
                    }
                }

                AnimePageStep::SelectSeason(step_state) => todo!(),
                AnimePageStep::SelectLang(step_state) => todo!(),
                AnimePageStep::SelectEpisodes(step_state) => todo!(),
                AnimePageStep::Downloads(step_state) => todo!(),
            },

            KeyCode::Char('k') => match &mut self.current_step {
                AnimePageStep::Search(step_state) => {
                    if let Some(selected_index) = step_state.result_list_state.selected() {
                        let len = step_state.result_list_items.len();
                        let previous_index =
                            ((selected_index as i8 - 1) + len as i8) as usize % len;
                        step_state.result_list_state.select(Some(previous_index));
                    }
                }

                AnimePageStep::SelectSeason(step_state) => todo!(),
                AnimePageStep::SelectLang(step_state) => todo!(),
                AnimePageStep::SelectEpisodes(step_state) => todo!(),
                AnimePageStep::Downloads(step_state) => todo!(),
            },

            KeyCode::Enter => match &mut self.current_step {
                AnimePageStep::Search(_) => {
                    self.current_step =
                        AnimePageStep::SelectSeason(SelectSeasonStepState { is_fetching: false });
                    self.state.full_path.push("Season".into());
                }
                AnimePageStep::SelectSeason(_) => {
                    self.current_step = AnimePageStep::SelectLang(SelectLangStepState {})
                }

                AnimePageStep::SelectLang(_) => {
                    self.current_step = AnimePageStep::SelectEpisodes(SelectEpisodesStepState {})
                }
                AnimePageStep::SelectEpisodes(_) => {
                    self.current_step = AnimePageStep::Downloads(DownloadsStepState {})
                }
                AnimePageStep::Downloads(_) => {}
            },

            _ => {}
        }
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &AnimeSamaEvent) {
        match &mut self.current_step {
            AnimePageStep::Search(step_state) => {
                if event.poll {
                    step_state.throbber_state.calc_next();
                }

                if event.search_result.is_some() {
                    step_state.result_list_items = event.search_result.clone().unwrap();
                    step_state.result_list_state.select(Some(0));
                    step_state.is_searching = false;
                }

                if !event.search_error_message.is_empty() {
                    step_state.last_search_error_message = event.search_error_message.clone();
                    step_state.is_searching = false;
                }
            }

            AnimePageStep::SelectSeason(step_state) => todo!(),
            AnimePageStep::SelectLang(step_state) => todo!(),
            AnimePageStep::SelectEpisodes(step_state) => todo!(),
            AnimePageStep::Downloads(step_state) => todo!(),
        };
    }
}
