use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{List, ListItem, ListState},
};

use crate::providers::{AnimeProviderNames, AnimeProviderState};

pub struct HomeProvider {
    pub state: AnimeProviderState,
    pub selected_provider: Option<AnimeProviderNames>,
    list_state: ListState,
    list_items: [(String, String, AnimeProviderNames); 2],
}

impl Default for HomeProvider {
    fn default() -> Self {
        Self {
            selected_provider: None,
            list_state: ListState::default().with_selected(Some(0)),
            list_items: [
                (
                    "Anime Sama".into(),
                    "The best french anime streaming website anime-sama.to".into(),
                    AnimeProviderNames::AnimeSama,
                ),
                (
                    "Nyaa".into(),
                    "A cat with a keyboard at nyaa.si".into(),
                    AnimeProviderNames::AnimeSama,
                ),
            ],
            state: AnimeProviderState {
                title: " Select an anime provider ".into(),
                full_path: vec!["".into()],
                want_poll: false,
                poll_timeout: Option::None,
                helps: vec![
                    ("[j/k]".into(), "move".into()),
                    ("[enter]".into(), "select".into()),
                ],
            },
        }
    }
}

impl HomeProvider {
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        let [banner_layout, selection_layout] =
            Layout::vertical([Constraint::Length(12), Constraint::Fill(1)]).areas(area);

        let big_banner = Text::from(
            "
   █████████   ██████   █████ █████ ██████   ██████ ██████████    ██████████   █████ █████   █████ ██████████
  ███░░░░░███ ░░██████ ░░███ ░░███ ░░██████ ██████ ░░███░░░░░█   ░░███░░░░███ ░░███ ░░███   ░░███ ░░███░░░░░█
 ░███    ░███  ░███░███ ░███  ░███  ░███░█████░███  ░███  █ ░     ░███   ░░███ ░███  ░███    ░███  ░███  █ ░
 ░███████████  ░███░░███░███  ░███  ░███░░███ ░███  ░██████       ░███    ░███ ░███  ░███    ░███  ░██████
 ░███░░░░░███  ░███ ░░██████  ░███  ░███ ░░░  ░███  ░███░░█       ░███    ░███ ░███  ░░███   ███   ░███░░█
 ░███    ░███  ░███  ░░█████  ░███  ░███      ░███  ░███ ░   █    ░███    ███  ░███   ░░░█████░    ░███ ░   █
 █████   █████ █████  ░░█████ █████ █████     █████ ██████████    ██████████   █████    ░░███      ██████████
░░░░░   ░░░░░ ░░░░░    ░░░░░ ░░░░░ ░░░░░     ░░░░░ ░░░░░░░░░░    ░░░░░░░░░░   ░░░░░      ░░░      ░░░░░░░░░░
                                         Anime downloader by BnSplit
",
        )
        .cyan();

        let small_banner = Text::from(
            "
   █████████   ██████████
  ███░░░░░███ ░░███░░░░███
 ░███    ░███  ░███   ░░███
 ░███████████  ░███    ░███
 ░███░░░░░███  ░███    ░███
 ░███    ░███  ░███    ███
 █████   █████ ██████████
░░░░░   ░░░░░ ░░░░░░░░░░
Anime downloader by BnSplit
",
        )
        .cyan();

        let banner = if banner_layout.width < big_banner.width() as u16 {
            small_banner
        } else {
            big_banner
        };

        let banner_layout =
            banner_layout.centered_horizontally(Constraint::Length(banner.width() as u16));

        let list_items = self
            .list_items
            .iter()
            .map(|item| {
                ListItem::new(Line::from(vec![
                    Span::raw(format!("[{}]", &item.0)).white().bold(),
                    Span::raw(" "),
                    Span::raw(&item.1).dark_gray().italic(),
                ]))
                .dim()
            })
            .collect::<Vec<ListItem>>();

        let list = List::new(list_items)
            .style(Color::DarkGray)
            .highlight_style(Style::new().not_dim())
            .highlight_symbol("> ".light_cyan())
            .scroll_padding(1);

        frame.render_widget(banner, banner_layout);
        frame.render_stateful_widget(list, selection_layout, &mut self.list_state);
    }

    pub fn handle_key(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('j') => {
                if let Some(selected_index) = self.list_state.selected() {
                    let len = self.list_items.len();
                    let next_index = (selected_index + 1) % (len);
                    self.list_state.select(Some(next_index));
                }
            }
            KeyCode::Char('k') => {
                if let Some(selected_index) = self.list_state.selected() {
                    let len = self.list_items.len();
                    let previous_index = ((selected_index as i8 - 1) + len as i8) as usize % len;
                    self.list_state.select(Some(previous_index));
                }
            }
            KeyCode::Enter => self.selected_provider = Some(self.selected_list_item().2),
            _ => {}
        }
    }

    fn selected_list_item(&self) -> &(String, String, AnimeProviderNames) {
        &self.list_items[self.list_state.selected().unwrap_or_default()]
    }
}
