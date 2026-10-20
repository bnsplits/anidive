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
    symbols,
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph, Tabs},
};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use throbber_widgets_tui::{Throbber, ThrobberState};
use tui_input::{Input, backend::crossterm::EventHandler};

use crate::{
    ThreadEvent,
    pages::{
        InputMode, PageState,
        nyaa::{
            NyaaEvent, NyaaPageStep, NyaaResult, NyaaResultPagination,
            downloads_step::DownloadsStepState,
        },
    },
};

pub struct SearchStepState {
    pub throbber_state: ThrobberState,
    pub result_list_state: ListState,
    pub result_list_items: Vec<NyaaResult>,
    pub pagination: Vec<NyaaResultPagination>,
    pub selected_page_index: usize,
    pub last_search_error_message: String,
    pub is_searching: bool,
    pub input: Input,
}

impl SearchStepState {
    /// View
    pub fn view(&mut self, input_mode: InputMode, area: Rect, frame: &mut Frame) {
        let [search_layout, tab_layout, results_layout] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .spacing(1)
        .areas(area);

        let search_layout = search_layout.centered_horizontally(Constraint::Percentage(50));
        let width = search_layout.width.max(2) - 3;
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

        frame.render_widget(search_field, search_layout);
        if let InputMode::Insert = input_mode {
            frame.set_cursor_position((
                search_layout.x + 1 + (self.input.visual_cursor().max(scroll) - scroll) as u16,
                search_layout.y + 1,
            ));
        }

        /* Pagination */
        if !self.pagination.is_empty() {
            let tab_width = self
                .pagination
                .iter()
                .map(|p| p.0.to_string().chars().count() + 2)
                .sum::<usize>()
                + self.pagination.len().saturating_sub(1) * symbols::DOT.chars().count();
            let tab_layout = tab_layout.centered_horizontally(Constraint::Length(tab_width as u16));

            let tabs = Tabs::new(self.pagination.iter().map(|p| format!(" {} ", p.0)))
                .style(Color::Gray)
                .highlight_style(Style::new().fg(Color::Black).bg(Color::Blue).bold())
                .select(
                    self.selected_page_index
                        .min(self.pagination.len().saturating_sub(1)),
                )
                .divider(symbols::DOT)
                .padding(" ", " ");

            frame.render_widget(tabs, tab_layout);
        }

        /* Throbber */
        if self.is_searching {
            let throbber = Throbber::default()
                .label("Searching...")
                .style(Style::default().fg(Color::Gray))
                .throbber_style(Style::default().fg(Color::Gray).bold())
                .throbber_set(throbber_widgets_tui::BRAILLE_SIX)
                .use_type(throbber_widgets_tui::WhichUse::Spin);
            let results_layout = results_layout.centered_horizontally(Constraint::Length(
                throbber.to_line(&self.throbber_state).width() as u16,
            ));
            frame.render_stateful_widget(throbber, results_layout, &mut self.throbber_state);
        } else if self.last_search_error_message.is_empty() {
            let list_items = self.result_list_items.iter().map(|item| {
                let metadata_width = 17
                    + item.2.chars().count()
                    + item.3.chars().count()
                    + item.4.chars().count()
                    + item.5.chars().count()
                    + item.6.chars().count();
                let max_name_width = (results_layout.width as usize)
                    .saturating_sub(metadata_width + 2)
                    .max(10);

                let line = Line::from(vec![
                    Span::raw(truncate_name(&item.0, max_name_width)).white(),
                    Span::raw(" • ").dark_gray(),
                    Span::raw(item.2.clone()).white(),
                    Span::raw(" • ").dark_gray(),
                    Span::raw(item.3.clone()).white(),
                    Span::raw(" • ").dark_gray(),
                    Span::raw("S:").white(),
                    Span::raw(item.4.clone()).green(),
                    Span::raw(" ").white(),
                    Span::raw("L:").white(),
                    Span::raw(item.5.clone()).red(),
                    Span::raw(" ").white(),
                    Span::raw("D:").white(),
                    Span::raw(item.6.clone()).white(),
                ]);

                ListItem::new(line).dim()
            });
            let list = List::new(list_items)
                .highlight_style(Style::new().not_dim())
                .highlight_symbol("> ".light_cyan())
                .scroll_padding(1);

            frame.render_stateful_widget(list, results_layout, &mut self.result_list_state);
        } else {
            let msg = format!("Oops : {}", self.last_search_error_message);
            let results_layout =
                results_layout.centered_horizontally(Constraint::Length(msg.len() as u16));
            frame.render_widget(Line::from(msg).centered().light_red(), results_layout);
        }
    }

    /// Handle crossterm raw events
    pub fn handle_crossterm_raw_event(
        &mut self,
        page_state: &mut PageState,
        tx: Sender<ThreadEvent>,
        event: &Event,
    ) -> Option<NyaaPageStep> {
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
    ) -> Option<NyaaPageStep> {
        if self.is_searching {
            return None;
        }

        match key_event.code {
            KeyCode::Char('r') => {
                self.refresh(tx);
            }

            KeyCode::Char('j') => {
                if let Some(selected_index) = self.result_list_state.selected() {
                    let len = self.result_list_items.len();
                    if len == 0 {
                        return None;
                    }
                    let next_index = (selected_index + 1) % len;
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

            KeyCode::Char('h') if self.pagination.len() > 1 => {
                let index =
                    (self.selected_page_index + self.pagination.len() - 1) % self.pagination.len();
                self.go_to_page(index, tx);
            }

            KeyCode::Char('l') if self.pagination.len() > 1 => {
                let index = (self.selected_page_index + 1) % self.pagination.len();
                self.go_to_page(index, tx);
            }

            KeyCode::Char(c @ '1'..='9') => {
                let page = c.to_digit(10).unwrap();
                self.go_to_page_number(page, tx);
            }

            KeyCode::Enter => {
                if self.result_list_items.is_empty() {
                    return None;
                }

                let selected_index = self.result_list_state.selected().unwrap_or(0);
                let torrent = self.result_list_items.get(selected_index).cloned()?;

                if let Some(last) = page_state.full_path.last_mut() {
                    *last = torrent.0.clone();
                }

                page_state.input_mode = InputMode::None;
                page_state.full_path.push("Downloads".into());
                page_state.helps = vec![("[enter]".into(), "open folder".into())];

                return Some(NyaaPageStep::Downloads({
                    let mut downloads_state = DownloadsStepState {
                        torrent,
                        status: None,
                        download_dir: crate::download_dir().join("Nyaa"),
                    };
                    downloads_state.start_downloads(tx);
                    downloads_state
                }));
            }

            _ => {}
        }

        None
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &NyaaEvent) {
        if let NyaaEvent::Search {
            poll,
            search_result,
            pagination,
            search_error_message,
        } = event
        {
            if *poll {
                self.throbber_state.calc_next();
            }

            if let Some(search_result) = search_result {
                self.result_list_items = search_result.clone();
                self.result_list_state.select(Some(0));
                self.is_searching = false;
            }

            if let Some(pagination) = pagination {
                self.pagination = pagination.clone();
                self.selected_page_index = pagination.iter().position(|p| p.1).unwrap_or(0);
            }

            if !search_error_message.is_empty() {
                self.last_search_error_message = search_error_message.clone();
                self.is_searching = false;
            }
        }
    }

    /// Search
    pub fn search(&mut self, tx: Sender<ThreadEvent>) {
        if self.is_searching {
            return;
        }

        self.selected_page_index = 0;
        self.result_list_state.select(Some(0));

        let url = self.page_url(1);
        self.fetch(tx, url);
    }

    /// Refresh current page
    fn refresh(&mut self, tx: Sender<ThreadEvent>) {
        if self.is_searching {
            return;
        }

        let page = self
            .pagination
            .iter()
            .find(|p| p.1)
            .map(|p| p.0)
            .unwrap_or(1);
        let url = self.page_url(page);
        self.fetch(tx, url);
    }

    /// Navigate to a pagination page
    fn go_to_page(&mut self, index: usize, tx: Sender<ThreadEvent>) {
        if self.is_searching {
            return;
        }

        let Some(page) = self.pagination.get(index) else {
            return;
        };
        if page.2.is_empty() {
            return;
        }

        let url = page.2.clone();
        self.selected_page_index = index;
        self.result_list_state.select(Some(0));
        self.fetch(tx, url);
    }

    /// Navigate to a page number
    fn go_to_page_number(&mut self, page: u32, tx: Sender<ThreadEvent>) {
        if self.is_searching {
            return;
        }

        if !self.pagination.iter().any(|p| p.0 == page && !p.1) {
            return;
        }

        if let Some(index) = self.pagination.iter().position(|p| p.0 == page) {
            self.go_to_page(index, tx);
            return;
        }

        self.result_list_state.select(Some(0));
        let url = self.page_url(page);
        self.fetch(tx, url);
    }

    fn page_url(&self, page: u32) -> String {
        let search_text = self
            .input
            .value()
            .split(" ")
            .map(|w| w.trim())
            .collect::<Vec<&str>>()
            .join("+");

        format!("https://nyaa.si/?f=0&c=1_0&q={}&p={}", search_text, page)
    }

    fn fetch(&mut self, tx: Sender<ThreadEvent>, url: String) {
        if self.is_searching {
            return;
        }

        self.is_searching = true;
        self.last_search_error_message = String::new();

        let (done_tx, done_rx) = mpsc::channel::<()>();

        /* Make the Throbber spin in another thread */
        let poll_tx = tx.clone();
        thread::spawn(move || {
            while done_rx.try_recv().is_err() {
                thread::sleep(Duration::from_millis(100));
                poll_tx
                    .send(ThreadEvent::NyaaEvent(NyaaEvent::Search {
                        poll: true,
                        search_result: None,
                        pagination: None,
                        search_error_message: String::new(),
                    }))
                    .unwrap();
            }
        });

        /* Perform the actual search on another thread */
        let query_tx = tx.clone();
        thread::spawn(move || {
            let res = Client::new()
                .get(url)
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

                let row_sel = Selector::parse("table.torrent-list tbody tr").unwrap();
                let td_sel = Selector::parse("td").unwrap();
                let a_sel = Selector::parse("a").unwrap();
                let name_sel = Selector::parse(r##"a[href^="/view/"]:not([href*="#"])"##).unwrap();
                let magnet_sel = Selector::parse(r#"a[href^="magnet:"]"#).unwrap();
                let pagination_sel = Selector::parse("ul.pagination li").unwrap();

                let items = document
                    .select(&row_sel)
                    .filter_map(|row| {
                        let tds = row.select(&td_sel).collect::<Vec<_>>();

                        let name = tds
                            .get(1)?
                            .select(&name_sel)
                            .next()?
                            .text()
                            .collect::<String>()
                            .trim()
                            .to_string();
                        let magnet_link = tds
                            .get(2)?
                            .select(&magnet_sel)
                            .next()?
                            .value()
                            .attr("href")?
                            .to_string();
                        let size = tds.get(3)?.text().collect::<String>().trim().to_string();
                        let date = tds.get(4)?.text().collect::<String>().trim().to_string();
                        let seeders = tds.get(5)?.text().collect::<String>().trim().to_string();
                        let leechers = tds.get(6)?.text().collect::<String>().trim().to_string();
                        let downloads = tds.get(7)?.text().collect::<String>().trim().to_string();

                        Some((name, magnet_link, size, date, seeders, leechers, downloads))
                    })
                    .collect::<Vec<NyaaResult>>();

                let pagination = document
                    .select(&pagination_sel)
                    .filter_map(|li| {
                        let is_previous = li.value().classes().any(|c| c == "previous");
                        let is_next = li.value().classes().any(|c| c == "next");
                        if is_previous || is_next {
                            return None;
                        }

                        let a = li.select(&a_sel).next()?;
                        let number = a.text().collect::<String>().trim().parse::<u32>().ok()?;
                        let is_active = li.value().classes().any(|c| c == "active");
                        let full_href = a
                            .value()
                            .attr("href")
                            .map(|href| format!("https://nyaa.si{}", href))
                            .unwrap_or_default();

                        Some((number, is_active, full_href))
                    })
                    .collect::<Vec<NyaaResultPagination>>();

                let _ = query_tx.send(ThreadEvent::NyaaEvent(NyaaEvent::Search {
                    poll: false,
                    search_result: Some(items),
                    pagination: Some(pagination),
                    search_error_message: String::new(),
                }));
            }

            if let Err(e) = &res {
                let _ = query_tx.send(ThreadEvent::NyaaEvent(NyaaEvent::Search {
                    poll: false,
                    search_result: None,
                    pagination: None,
                    search_error_message: e.to_string(),
                }));
            }

            let _ = done_tx.send(());
        });
    }
}

fn truncate_name(name: &str, max_chars: usize) -> String {
    if name.chars().count() <= max_chars {
        return name.to_string();
    }

    let keep = max_chars.saturating_sub(3);
    format!("{}...", name.chars().take(keep).collect::<String>())
}
