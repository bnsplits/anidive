use std::{sync::mpsc, thread, time::Duration};

use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use regex::Regex;
use reqwest::blocking::Client;
use scraper::{ElementRef, Html, Selector};
use throbber_widgets_tui::{Throbber, ThrobberState};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    ThreadEvent,
    pages::{
        InputMode::{self, Normal},
        PageState,
    },
};

type AnimeResult = (String, String, String);
type SeasonResult = (String, String);
type LangResult = (String, String);
type EpisodesResult = Vec<(String, String, String)>;

struct SearchStepState {
    throbber_state: ThrobberState,
    result_list_state: ListState,
    result_list_items: Vec<AnimeResult>,
    last_search_error_message: String,
    is_searching: bool,
    input: Input,
}

impl SearchStepState {
    fn search(&mut self, tx: mpsc::Sender<ThreadEvent>) {
        self.is_searching = true;
        self.last_search_error_message = String::new();

        let (done_tx, done_rx) = mpsc::channel::<()>();

        /* Make the Throbber spin in another thread */
        let poll_tx = tx.clone();
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

                let _ = query_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent {
                    search_result: Some(items),
                    ..Default::default()
                }));
            }

            if let Err(e) = &res {
                let _ = query_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent {
                    search_error_message: e.to_string(),
                    ..Default::default()
                }));
            }

            let _ = done_tx.send(());
        });
    }
}

struct SelectSeasonStepState {
    throbber_state: ThrobberState,
    result_list_state: ListState,
    result_list_items: Vec<SeasonResult>,
    last_season_fetch_error_message: String,
    selected_anime: AnimeResult,
    is_fetching: bool,
}

impl SelectSeasonStepState {
    fn fetch_seasons(&mut self, tx: mpsc::Sender<ThreadEvent>) {
        self.is_fetching = true;
        self.last_season_fetch_error_message = String::new();

        let (done_tx, done_rx) = mpsc::channel::<()>();

        /* Make the Throbber spin in another thread */
        let poll_tx = tx.clone();
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
                // Target lines that looks like that : panneauAnime("Saison 1", "saison1/vostfr");
                let re = Regex::new(r#"(?m)^[ \t]*panneauAnime\("([^"]*)",[ \t]*"([^"]*)"\);?"#)
                    .unwrap();

                let items = re
                    .captures_iter(res)
                    .map(|c| {
                        (
                            c[1].to_string(),
                            format!("{}/{}", url.trim_end_matches('/'), &c[2]),
                        )
                    })
                    .collect();

                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent {
                    fetch_seasons_result: Some(items),
                    ..Default::default()
                }));
            }

            if let Err(e) = &res {
                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent {
                    fetch_seasons_error_message: e.to_string(),
                    ..Default::default()
                }));
            }

            let _ = done_tx.send(());
        });
    }
}

struct SelectLangStepState {
    throbber_state: ThrobberState,
    result_list_state: ListState,
    result_list_items: Vec<SeasonResult>,
    last_language_fetch_error_message: String,
    selected_season: SeasonResult,
    is_fetching: bool,
}

impl SelectLangStepState {
    fn fetch_langs(&mut self, tx: mpsc::Sender<ThreadEvent>) {
        self.is_fetching = true;
        self.last_language_fetch_error_message = String::new();

        let (done_tx, done_rx) = mpsc::channel::<()>();

        /* Make the Throbber spin in another thread */
        let poll_tx = tx.clone();
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
                                // Remove a possible triming '/'
                                .trim_end_matches('/')
                                // Remove the original /lang at the end
                                .rsplit_once('/')
                                .unwrap()
                                .0,
                            a.value()
                                .attr("href")
                                .and_then(|a| a.strip_prefix("../"))
                                .unwrap_or_default(),
                        );

                        // Return the lang only if it actually exists
                        client
                            .head(&href)
                            .send()
                            .ok()?
                            .status()
                            .is_success()
                            .then_some((id.to_string(), href))
                    })
                    .collect();

                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent {
                    fetch_langs_result: Some(items),
                    ..Default::default()
                }));
            }

            if let Err(e) = &res {
                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent {
                    fetch_langs_error_message: e.to_string(),
                    ..Default::default()
                }));
            }

            let _ = done_tx.send(());
        });
    }
}

struct SelectEpisodesStepState {
    throbber_state: ThrobberState,
    result_list_state: ListState,
    result_list_items: Vec<EpisodesResult>,
    last_episodes_fetch_error_message: String,
    selected_lang: LangResult,
    is_fetching: bool,
}

impl SelectLangStepState {
    fn fetch_episodes(&mut self, tx: mpsc::Sender<ThreadEvent>) {}
}

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

    search_result: Option<Vec<AnimeResult>>,
    search_error_message: String,

    fetch_seasons_result: Option<Vec<SeasonResult>>,
    fetch_seasons_error_message: String,

    fetch_langs_result: Option<Vec<LangResult>>,
    fetch_langs_error_message: String,

    fetch_episodes_result: Option<Vec<EpisodesResult>>,
    fetch_episodes_error_message: String,
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
                .throbber_style(Style::default().fg(Color::Gray).bold())
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
    fn select_season_step(&mut self, area: Rect, frame: &mut Frame) {
        if let AnimePageStep::SelectSeason(step_state) = &mut self.current_step {
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
            let list_items = step_state.result_list_items.iter().map(|item| {
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
            if step_state.is_fetching {
                frame.render_stateful_widget(
                    throbber,
                    results_layout,
                    &mut step_state.throbber_state,
                );
            } else {
                if step_state.last_season_fetch_error_message.is_empty() {
                    frame.render_stateful_widget(
                        list,
                        results_layout,
                        &mut step_state.result_list_state,
                    );
                } else {
                    let msg = format!("Oops : {}", step_state.last_season_fetch_error_message);
                    let results_layout =
                        results_layout.centered_horizontally(Constraint::Length(msg.len() as u16));
                    frame.render_widget(Line::from(msg).centered().light_red(), results_layout);
                }
            }
        }
    }

    // View select languages step
    fn select_lang_step(&mut self, area: Rect, frame: &mut Frame) {
        if let AnimePageStep::SelectLang(step_state) = &mut self.current_step {
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
            let list_items = step_state.result_list_items.iter().map(|item| {
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
            if step_state.is_fetching {
                frame.render_stateful_widget(
                    throbber,
                    results_layout,
                    &mut step_state.throbber_state,
                );
            } else {
                if step_state.last_language_fetch_error_message.is_empty() {
                    frame.render_stateful_widget(
                        list,
                        results_layout,
                        &mut step_state.result_list_state,
                    );
                } else {
                    let msg = format!("Oops : {}", step_state.last_language_fetch_error_message);
                    let results_layout =
                        results_layout.centered_horizontally(Constraint::Length(msg.len() as u16));
                    frame.render_widget(Line::from(msg).centered().light_red(), results_layout);
                }
            }
        }
    }

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
                            self.state.input_mode = Some(Normal);
                            step_state.search(self.tx.clone());
                        }

                        /* Exit input mode */
                        KeyCode::Esc => self.state.input_mode = Some(InputMode::Normal),

                        _ => {}
                    }
                }
            }

            AnimePageStep::SelectSeason(step_state) => {}
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

                AnimePageStep::SelectSeason(step_state) => {
                    if let Some(selected_index) = step_state.result_list_state.selected() {
                        let len = step_state.result_list_items.len();
                        let next_index = (selected_index + 1) % (len);
                        step_state.result_list_state.select(Some(next_index));
                    }
                }

                AnimePageStep::SelectLang(step_state) => {
                    if let Some(selected_index) = step_state.result_list_state.selected() {
                        let len = step_state.result_list_items.len();
                        let next_index = (selected_index + 1) % (len);
                        step_state.result_list_state.select(Some(next_index));
                    }
                }

                AnimePageStep::SelectEpisodes(step_state) => {
                    if let Some(selected_index) = step_state.result_list_state.selected() {
                        let len = step_state.result_list_items.len();
                        let next_index = (selected_index + 1) % (len);
                        step_state.result_list_state.select(Some(next_index));
                    }
                }

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

                AnimePageStep::SelectSeason(step_state) => {
                    if let Some(selected_index) = step_state.result_list_state.selected() {
                        let len = step_state.result_list_items.len();
                        let previous_index =
                            ((selected_index as i8 - 1) + len as i8) as usize % len;
                        step_state.result_list_state.select(Some(previous_index));
                    }
                }

                AnimePageStep::SelectLang(step_state) => {
                    if let Some(selected_index) = step_state.result_list_state.selected() {
                        let len = step_state.result_list_items.len();
                        let previous_index =
                            ((selected_index as i8 - 1) + len as i8) as usize % len;
                        step_state.result_list_state.select(Some(previous_index));
                    }
                }

                AnimePageStep::SelectEpisodes(step_state) => {
                    if let Some(selected_index) = step_state.result_list_state.selected() {
                        let len = step_state.result_list_items.len();
                        let previous_index =
                            ((selected_index as i8 - 1) + len as i8) as usize % len;
                        step_state.result_list_state.select(Some(previous_index));
                    }
                }

                AnimePageStep::Downloads(step_state) => todo!(),
            },

            KeyCode::Enter => match &mut self.current_step {
                AnimePageStep::Search(step_state) => {
                    self.current_step = AnimePageStep::SelectSeason({
                        let mut season_state = SelectSeasonStepState {
                            throbber_state: ThrobberState::default(),
                            result_list_state: ListState::default().with_selected(Some(0)),
                            result_list_items: Vec::new(),
                            last_season_fetch_error_message: String::new(),
                            is_fetching: false,
                            selected_anime: step_state.result_list_items
                                [step_state.result_list_state.selected().unwrap()]
                            .clone(),
                        };
                        season_state.fetch_seasons(self.tx.clone());
                        season_state
                    });
                    self.state.full_path.push("Season".into());
                }

                AnimePageStep::SelectSeason(step_state) => {
                    self.current_step = AnimePageStep::SelectLang({
                        let mut lang_state = SelectLangStepState {
                            throbber_state: ThrobberState::default(),
                            result_list_state: ListState::default().with_selected(Some(0)),
                            result_list_items: Vec::new(),
                            last_language_fetch_error_message: String::new(),
                            is_fetching: false,
                            selected_season: step_state.result_list_items
                                [step_state.result_list_state.selected().unwrap()]
                            .clone(),
                        };
                        lang_state.fetch_langs(self.tx.clone());
                        lang_state
                    });
                    self.state.full_path.push("Language".into());
                }

                AnimePageStep::SelectLang(step_state) => {
                    self.current_step = AnimePageStep::SelectEpisodes({
                        let mut episodes_state = SelectEpisodesStepState {
                            throbber_state: ThrobberState::default(),
                            result_list_state: ListState::default().with_selected(Some(0)),
                            result_list_items: Vec::new(),
                            last_episodes_fetch_error_message: String::new(),
                            is_fetching: false,
                            selected_lang: step_state.result_list_items
                                [step_state.result_list_state.selected().unwrap()]
                            .clone(),
                        };
                        // episodes_state.fetch_episodes(self.tx.clone());
                        episodes_state
                    });
                    self.state.full_path.push("Episodes".into());
                }

                AnimePageStep::SelectEpisodes(step_state) => {
                    self.current_step = AnimePageStep::Downloads({
                        let mut downloads_state = DownloadsStepState {};
                        // downloads_state.start_downlaods(self.tx.clone());
                        downloads_state
                    });
                    self.state.full_path.push("Downloads".into());
                }

                AnimePageStep::Downloads(step_state) => {}
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

            AnimePageStep::SelectSeason(step_state) => {
                if event.poll {
                    step_state.throbber_state.calc_next();
                }

                if event.fetch_seasons_result.is_some() {
                    step_state.result_list_items = event.fetch_seasons_result.clone().unwrap();
                    step_state.result_list_state.select(Some(0));
                    step_state.is_fetching = false;
                }

                if !event.fetch_seasons_error_message.is_empty() {
                    step_state.last_season_fetch_error_message =
                        event.fetch_seasons_error_message.clone();
                    step_state.is_fetching = false;
                }
            }

            AnimePageStep::SelectLang(step_state) => {
                if event.poll {
                    step_state.throbber_state.calc_next();
                }

                if event.fetch_langs_result.is_some() {
                    step_state.result_list_items = event.fetch_langs_result.clone().unwrap();
                    step_state.result_list_state.select(Some(0));
                    step_state.is_fetching = false;
                }

                if !event.fetch_seasons_error_message.is_empty() {
                    step_state.last_language_fetch_error_message =
                        event.fetch_langs_error_message.clone();
                    step_state.is_fetching = false;
                }
            }
            AnimePageStep::SelectEpisodes(step_state) => {
                if event.poll {
                    step_state.throbber_state.calc_next();
                }

                if event.fetch_episodes_result.is_some() {
                    step_state.result_list_items = event.fetch_episodes_result.clone().unwrap();
                    step_state.result_list_state.select(Some(0));
                    step_state.is_fetching = false;
                }

                if !event.fetch_seasons_error_message.is_empty() {
                    step_state.last_episodes_fetch_error_message =
                        event.fetch_episodes_error_message.clone();
                    step_state.is_fetching = false;
                }
            }
            AnimePageStep::Downloads(step_state) => todo!(),
        };
    }
}
