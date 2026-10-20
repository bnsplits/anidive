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
    symbols::{
        self,
        scrollbar::{Set, VERTICAL},
    },
    text::Line,
    widgets::{Block, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Tabs},
};
use regex::Regex;
use reqwest::{
    blocking::Client,
    header::{HeaderMap, HeaderValue, REFERER},
};
use throbber_widgets_tui::{Throbber, ThrobberState};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    ThreadEvent,
    pages::{
        InputMode, PageState,
        animesama::{
            AnimeSamaEvent, AnimeSamaPageStep, DownloadStatus, Episode, EpisodePlayer,
            EpisodeProvider, LangResult, downloads_step::DownloadsStepState,
        },
    },
};

pub struct SelectEpisodesStepState {
    pub throbber_state: ThrobberState,
    pub preview_scroll: u16,
    pub result_list_items: Vec<EpisodePlayer>,
    pub last_episodes_fetch_error_message: String,
    pub selected_lang: LangResult,
    pub is_fetching: bool,
    pub selected_player_index: usize,
    pub input: Input,
}

impl SelectEpisodesStepState {
    /// View
    pub fn view(&mut self, input_mode: InputMode, area: Rect, frame: &mut Frame) {
        let [instruction_layout, results_layout] =
            Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).areas(area);

        /* Instructions text */
        let instructions = Line::from(if self.result_list_items.is_empty() {
            "Select episodes among the available ones".to_string()
        } else {
            format!(
                "Select episodes among the '{}' available ones",
                self.result_list_items[self.selected_player_index]
                    .episodes
                    .len()
            )
        })
        .centered()
        .dark_gray()
        .italic();
        frame.render_widget(instructions, instruction_layout);

        /* Throbber */
        if self.is_fetching {
            let throbber = Throbber::default()
                .label("Please wait...")
                .style(Style::default().fg(Color::Gray))
                .throbber_style(Style::default().fg(Color::Gray).bold())
                .throbber_set(throbber_widgets_tui::BRAILLE_SIX)
                .use_type(throbber_widgets_tui::WhichUse::Spin);
            frame.render_stateful_widget(throbber, results_layout, &mut self.throbber_state);
        }

        /* Rendering */
        if !self.is_fetching {
            if self.last_episodes_fetch_error_message.is_empty() {
                let [tab_area, preview_area, input_area] = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Fill(1),
                    Constraint::Length(3),
                ])
                .spacing(1)
                .areas(results_layout);

                let tab_area = tab_area.centered_horizontally(Constraint::Length({
                    let len = self.result_list_items.len();

                    ("Player X".chars().count() * len
                        + format!(" {} ", symbols::DOT).chars().count() * len)
                        as u16
                }));

                let selected_player = &self.result_list_items[self.selected_player_index];

                /* Players */
                let tabs = Tabs::new(self.result_list_items.iter().map(|p| p.name.clone()))
                    .style(Color::Gray)
                    .highlight_style(Style::new().fg(Color::Black).bg(Color::Blue).bold())
                    .select(self.selected_player_index)
                    .divider(symbols::DOT)
                    .padding(" ", " ");
                frame.render_widget(tabs, tab_area);

                /* Episodes preview */
                let lines = selected_player
                    .episodes
                    .iter()
                    .map(|e| {
                        Line::from(format!("[{}] {}", e.provider, e.name))
                            .gray()
                            .dim()
                            .bold()
                    })
                    .collect::<Vec<Line>>();

                let max_scroll = (lines.len() as u16).saturating_sub(preview_area.height);
                self.preview_scroll = self.preview_scroll.min(max_scroll);

                let preview = Paragraph::new(lines).scroll((self.preview_scroll, 0));
                frame.render_widget(preview, preview_area);

                let mut sb_state =
                    ScrollbarState::new(max_scroll as usize).position(self.preview_scroll as usize);
                let sb = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .track_style(Color::DarkGray)
                    .thumb_style(Color::White)
                    .begin_style(Color::DarkGray)
                    .end_style(Color::DarkGray)
                    .symbols(Set {
                        thumb: "│",
                        begin: "",
                        end: "",
                        ..VERTICAL
                    });
                frame.render_stateful_widget(sb, preview_area, &mut sb_state);

                /* Input bar */
                let placeholder_text =
                    "Enter the episode range you want (ex: 1 or 5,8 or 1-6 or A for all)";
                let input_area = input_area
                    .centered_horizontally(Constraint::Length((placeholder_text.len() + 2) as u16));
                let width = input_area.width.max(2);
                let scroll = self.input.visual_scroll(width as usize);
                let input_text = if self.input.value().is_empty()
                    && let InputMode::Normal = input_mode
                {
                    placeholder_text
                } else {
                    self.input.value()
                };

                let search_field_style = match input_mode {
                    InputMode::Insert => Style::default(),
                    InputMode::Normal | InputMode::None => Style::default().dark_gray(),
                };
                let input_field = Paragraph::new(input_text)
                    .block(Block::bordered())
                    .style(search_field_style)
                    .scroll((0, scroll as u16));
                frame.render_widget(input_field, input_area);
                if let InputMode::Insert = input_mode {
                    frame.set_cursor_position((
                        input_area.x + 1 + (self.input.visual_cursor().max(scroll) - scroll) as u16,
                        input_area.y + 1,
                    ));
                }
            } else {
                let msg = format!("Oops : {}", self.last_episodes_fetch_error_message);
                let results_layout =
                    results_layout.centered_horizontally(Constraint::Length(msg.len() as u16));
                frame.render_widget(Line::from(msg).centered().light_red(), results_layout);
            }
        }
    }

    /// Handle crossterm raw events
    pub fn handle_crossterm_raw_event(
        &mut self,
        page_state: &mut PageState,
        tx: Sender<ThreadEvent>,
        event: &Event,
    ) -> Option<AnimeSamaPageStep> {
        self.input.handle_event(event);
        if let Event::Key(k) = event {
            match k.code {
                /* Confirm selection */
                KeyCode::Enter => self.handle_crossterm_key(page_state, tx, *k),

                /* Exit input mode */
                KeyCode::Esc => {
                    page_state.input_mode = InputMode::Normal;
                    None
                }

                _ => None,
            }
        } else {
            None
        }
    }

    /// Handle crossterm keys
    pub fn handle_crossterm_key(
        &mut self,
        page_state: &mut PageState,
        tx: Sender<ThreadEvent>,
        key_event: KeyEvent,
    ) -> Option<AnimeSamaPageStep> {
        match key_event.code {
            KeyCode::Char('r') => {
                self.fetch_episodes(tx);
            }

            KeyCode::Char('j') => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
            }

            KeyCode::Char('J') => {
                self.preview_scroll = self.preview_scroll.saturating_add(5);
            }

            KeyCode::Char('k') => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
            }

            KeyCode::Char('K') => {
                self.preview_scroll = self.preview_scroll.saturating_sub(5);
            }

            KeyCode::Char('h') => {
                self.selected_player_index = (self.selected_player_index
                    + (self.result_list_items.len() - 1))
                    % self.result_list_items.len();
            }

            KeyCode::Char('l') => {
                self.selected_player_index =
                    (self.selected_player_index + 1) % self.result_list_items.len();
            }

            KeyCode::Enter => {
                if self.result_list_items.is_empty() {
                    return None;
                }

                /* Try to parse the selection */
                let input = self.input.value().trim();
                let selected_episodes = (|| {
                    let episodes = self.result_list_items[self.selected_player_index]
                        .episodes
                        .clone();
                    let len = episodes.len();

                    if input.eq_ignore_ascii_case("a") {
                        return Some(episodes);
                    }

                    let mut out = Vec::new();

                    for part in input.split(',') {
                        let part = part.trim();
                        let (a, b): (usize, usize) = match part.split_once('-') {
                            Some((a, b)) => (a.trim().parse().ok()?, b.trim().parse().ok()?),
                            None => {
                                let n = part.parse().ok()?;
                                (n, n)
                            }
                        };

                        if a < 1 || b > len || a > b {
                            return None;
                        }

                        out.extend_from_slice(&episodes[a - 1..b]);
                    }

                    Some(out)
                })();

                if let Some(selected_episodes) = selected_episodes {
                    let range = input
                        .to_ascii_lowercase()
                        .replace("a", &format!("All (1-{})", selected_episodes.len()));

                    if let Some(last) = page_state.full_path.last_mut() {
                        *last = format!("Range: {}", range);
                    }

                    page_state.input_mode = InputMode::None;
                    page_state.full_path.push("Downloads".into());
                    page_state.helps = vec![
                        ("[enter]".into(), "open folder".into()),
                        ("[j/k]".into(), "scroll".into()),
                    ];

                    return Some(AnimeSamaPageStep::Downloads({
                        let mut full_path = [
                            ["AnimeSama".to_string()].to_vec(),
                            page_state.full_path.clone(),
                        ]
                        .concat();
                        full_path.pop();
                        full_path.pop();

                        let mut downloads_state = DownloadsStepState {
                            episodes: selected_episodes,
                            current_episode: None,
                            full_dir_path: full_path,
                            range,
                            preview_scroll: 0,
                        };
                        downloads_state.start_downlaods(tx);
                        downloads_state
                    }));
                }
            }

            _ => {}
        }

        None
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &AnimeSamaEvent) {
        if let AnimeSamaEvent::Episodes {
            poll,
            fetch_episodes_result,
            fetch_episodes_error_message,
        } = event
        {
            if *poll {
                self.throbber_state.calc_next();
            }

            if fetch_episodes_result.is_some() {
                self.result_list_items = fetch_episodes_result.clone().unwrap();
                self.preview_scroll = 0;
                self.is_fetching = false;
            }

            if !fetch_episodes_error_message.is_empty() {
                self.last_episodes_fetch_error_message = fetch_episodes_error_message.clone();
                self.is_fetching = false;
            }
        }
    }

    /// Fetch episodes
    pub fn fetch_episodes(&mut self, tx: mpsc::Sender<ThreadEvent>) {
        if self.is_fetching {
            return;
        }

        self.is_fetching = true;
        self.last_episodes_fetch_error_message = String::new();

        let (done_tx, done_rx) = mpsc::channel::<()>();

        /* Make the Throbber spin in another thread */
        let poll_tx = tx.clone();
        thread::spawn(move || {
            while done_rx.try_recv().is_err() {
                thread::sleep(Duration::from_millis(100));
                poll_tx
                    .send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Episodes {
                        poll: true,
                        fetch_episodes_result: None,
                        fetch_episodes_error_message: String::new(),
                    }))
                    .unwrap();
            }
        });

        /* Perform the actual fetch on another thread */
        let fetch_tx = tx.clone();
        let selected_lang = self.selected_lang.clone();
        thread::spawn(move || {
            let url = format!("{}/episodes.js", selected_lang.1.clone());

            let res: Result<String, String> = (|| {
                let mut headers = HeaderMap::new();

                if let Ok(referer) = HeaderValue::from_str(&selected_lang.1) {
                    headers.insert(REFERER, referer);
                }

                let client = Client::builder()
                    .user_agent(
                        "Mozilla/5.0 (X11; Linux x86_64; rv:157.0) Gecko/20100101 Firefox/157.0",
                    )
                    .default_headers(headers)
                    .build()
                    .map_err(|_| "could not build client".to_string())?;

                let resp = client
                    .get(&url)
                    .send()
                    .map_err(|_| "network error".to_string())?
                    .error_for_status()
                    .map_err(|e| format!("bad status: {}", e.status().map_or(0, |s| s.as_u16())))?;

                resp.text()
                    .map_err(|_| "could not read result body".to_string())
            })();

            if let Ok(res) = &res {
                /* Format the episodes.js content into a list of players each having episodes */
                let block = Regex::new(r"(?sm)^[ \t]*var\s+eps(\d+)\s*=\s*\[(.*?)\]").unwrap();
                let item = Regex::new(r#"['"]([^'"]*)['"]"#).unwrap();

                let mut players: Vec<EpisodePlayer> = block
                    .captures_iter(res)
                    .map(|c| {
                        let episodes = item
                            .captures_iter(&c[2])
                            .enumerate()
                            .map(|(i, m)| {
                                let url = m[1].to_string();
                                Episode {
                                    name: format!("Episode {}", i + 1),
                                    provider: EpisodeProvider::from_url(&url),
                                    status: DownloadStatus::Pending,
                                    url,
                                }
                            })
                            .collect();

                        EpisodePlayer {
                            name: format!("Player {}", &c[1]),
                            episodes,
                        }
                    })
                    .collect();

                players.sort_by(|a, b| a.name.cmp(&b.name));

                /* Intervert 'Player 1' and 'Player 2' because that's what anime-sama do */
                let p1 = players.iter().position(|p| p.name == "Player 1");
                let p2 = players.iter().position(|p| p.name == "Player 2");
                if let (Some(p1), Some(p2)) = (p1, p2) {
                    let tmp = std::mem::take(&mut players[p1].episodes);
                    players[p1].episodes = std::mem::replace(&mut players[p2].episodes, tmp);
                }

                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Episodes {
                    poll: false,
                    fetch_episodes_result: Some(players),
                    fetch_episodes_error_message: String::new(),
                }));
            }

            if let Err(e) = &res {
                let _ = fetch_tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Episodes {
                    poll: false,
                    fetch_episodes_result: None,
                    fetch_episodes_error_message: e.to_string(),
                }));
            }

            let _ = done_tx.send(());
        });
    }
}
