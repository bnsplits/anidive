use std::{
    fs,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::scrollbar::{Set, VERTICAL},
    text::{Line, Span},
    widgets::{Gauge, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
};

use crate::{
    ThreadEvent,
    pages::animesama::{
        AnimeSamaEvent, AnimeSamaPageStep, DownloadStatus, Episode, EpisodeProvider, downloaders,
        extractors::{DownloadLinkKind, ansembed, embed4me, sendvid, uqload, voe},
    },
};

#[allow(unused)]
pub struct DownloadsStepState {
    pub episodes: Vec<Episode>,
    pub current_episode: Option<Episode>,
    pub range: String,
    pub full_dir_path: Vec<String>,
    pub preview_scroll: u16,
}

impl DownloadsStepState {
    /// View
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        let [completed_area, gauge_area, preview_area] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .spacing(1)
        .areas(area);

        /* Completion status line */
        let completed = self
            .episodes
            .iter()
            .filter(|e| matches!(&e.status, DownloadStatus::Done))
            .collect::<Vec<&Episode>>();

        let failed = self
            .episodes
            .iter()
            .filter(|e| matches!(&e.status, DownloadStatus::Failed))
            .collect::<Vec<&Episode>>();

        let completed_row = Line::from(format!(
            "Completed: {}/{} {}",
            completed.len(),
            self.episodes.len(),
            if failed.is_empty() {
                "".to_string()
            } else {
                format!("| Failed: {}/{}", failed.len(), self.episodes.len())
            }
        ))
        .gray();

        frame.render_widget(completed_row, completed_area);

        if completed.len() + failed.len() == self.episodes.len() {
            frame.render_widget(Line::from("All done !").centered(), gauge_area);
        } else {
            /* Gauge */
            if let Some(episode) = &self.current_episode
                && let DownloadStatus::Running {
                    percent,
                    speed,
                    path: _,
                    eta,
                } = &episode.status
            {
                let gauge = Gauge::default()
                    .gauge_style(Style::new().fg(Color::Blue).bg(Color::Black))
                    .ratio(Clone::clone(percent) as f64)
                    .label(
                        format!(
                            "[{}] - [{:.0}%] - [{}] - [ETA: {}]",
                            episode.name,
                            percent * 100.0,
                            if speed.is_empty() { "0B/s" } else { speed },
                            if eta.is_empty() { "--:--" } else { eta }
                        )
                        .bold(),
                    );
                frame.render_widget(gauge, gauge_area);
            }
        }

        /* Preview */
        let status_color = |s: &DownloadStatus| match s {
            DownloadStatus::Pending => Color::DarkGray,
            DownloadStatus::Running { .. } => Color::LightYellow,
            DownloadStatus::Done => Color::LightGreen,
            DownloadStatus::Failed => Color::Red,
        };

        let name_w = self
            .episodes
            .iter()
            .map(|e| e.name.chars().count())
            .max()
            .unwrap_or(0);
        let sep = " • ";
        let sep_w = sep.chars().count();

        /* name + space + icon == (1 column) */
        let entry_w = name_w + 2;

        /* Leave one column for the scrollbar */
        let cols = ((preview_area.width as usize + sep_w) / (entry_w + sep_w))
            .max(1)
            .min(self.episodes.len().max(1));
        let block_w = cols * entry_w + (cols - 1) * sep_w;

        let lines: Vec<Line> = self
            .episodes
            .chunks(cols)
            .map(|row| {
                let mut spans: Vec<Span> = Vec::new();
                for (i, e) in row.iter().enumerate() {
                    let color = status_color(&e.status);
                    let icon = match &e.status {
                        DownloadStatus::Pending => "",
                        DownloadStatus::Running { .. } => "",
                        DownloadStatus::Done => "",
                        DownloadStatus::Failed => "",
                    };

                    spans.push(Span::from(format!("{:<name_w$} ", e.name)).fg(color));
                    spans.push(Span::from(icon).fg(color));

                    if i + 1 != row.len() {
                        spans.push(Span::from(sep).dark_gray());
                    }
                }
                Line::from(spans)
            })
            .collect();

        let max_scroll = (lines.len() as u16).saturating_sub(preview_area.height);
        self.preview_scroll = self.preview_scroll.min(max_scroll);

        let grid_area = preview_area.centered_horizontally(Constraint::Length(block_w as u16));

        frame.render_widget(
            Paragraph::new(lines).scroll((self.preview_scroll, 0)),
            grid_area,
        );

        /* Scrollbar */
        if max_scroll > 0 {
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
        }
    }

    /// Handle crossterm keys
    pub fn handle_crossterm_key(&mut self, key_event: KeyEvent) -> Option<AnimeSamaPageStep> {
        match key_event.code {
            KeyCode::Enter => {
                let _ = Command::new("xdg-open")
                    .arg(crate::download_dir().join(self.full_dir_path.join("/")))
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .process_group(0)
                    .spawn();
            }

            KeyCode::Char('j') => {
                self.preview_scroll = self.preview_scroll.saturating_add(1);
            }

            KeyCode::Char('k') => {
                self.preview_scroll = self.preview_scroll.saturating_sub(1);
            }

            _ => {}
        }
        None
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &AnimeSamaEvent) {
        if let AnimeSamaEvent::Downloads { current_episode } = event {
            /* If the arrived episode's status changed, then it updates the episodes list */
            if let Some(episode) = current_episode
                && matches!(
                    episode.status,
                    DownloadStatus::Done | DownloadStatus::Failed | DownloadStatus::Running { .. }
                )
                && let Some(slot) = self.episodes.iter_mut().find(|e| e.name == episode.name)
            {
                *slot = episode.clone();
            }

            self.current_episode = current_episode.clone();
        }
    }

    /// Start downloads
    pub fn start_downlaods(&mut self, tx: mpsc::Sender<ThreadEvent>) {
        let mut episode_queue = self.episodes.clone();
        let send_tx = tx.clone();
        let full_dir_path = self.full_dir_path.clone();

        let _ = tx.send(ThreadEvent::AppEvent(crate::AppEvent::DisableRestart));

        thread::spawn(move || {
            for episode in &mut episode_queue {
                let path = crate::download_dir()
                    .join(full_dir_path.join("/"))
                    .join(&episode.name);

                let path_created = if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).ok()
                } else {
                    None
                };

                episode.status = DownloadStatus::Running {
                    percent: 0.0,
                    speed: "".into(),
                    path: path.clone(),
                    eta: "".into(),
                };
                let url = &episode.url;
                let link = match episode.provider {
                    EpisodeProvider::Sendvid => sendvid::extract_sendvid_video_source(url),
                    EpisodeProvider::Ansembed => ansembed::extract_ansembed_video_source(url),
                    EpisodeProvider::Uqload => uqload::extract_uqload_video_source(url),
                    EpisodeProvider::Embed4me => embed4me::extract_embed4me_video_source(url),
                    EpisodeProvider::Voe => voe::extract_voe_video_source(url),
                    _ => None,
                };

                if let Some(_) = path_created
                    && let Some(link) = link
                {
                    let _ = send_tx.clone().send(ThreadEvent::AnimeSamaEvent(
                        AnimeSamaEvent::Downloads {
                            current_episode: Some(episode.clone()),
                        },
                    ));

                    let status = match link.kind {
                        DownloadLinkKind::Mp4 => {
                            downloaders::download_mp4(episode, &link.url, send_tx.clone())
                        }

                        DownloadLinkKind::M3U8 => {
                            downloaders::download_m3u8(episode, &link.url, send_tx.clone())
                        }
                    };

                    if status.is_some() {
                        episode.status = DownloadStatus::Done
                    } else {
                        episode.status = DownloadStatus::Failed
                    }

                    let _ = send_tx.clone().send(ThreadEvent::AnimeSamaEvent(
                        AnimeSamaEvent::Downloads {
                            current_episode: Some(episode.clone()),
                        },
                    ));
                } else {
                    episode.status = DownloadStatus::Failed;

                    let _ = send_tx.clone().send(ThreadEvent::AnimeSamaEvent(
                        AnimeSamaEvent::Downloads {
                            current_episode: Some(episode.clone()),
                        },
                    ));
                }
            }

            let _ = send_tx.send(ThreadEvent::AppEvent(crate::AppEvent::EnableRestart));
        });
    }
}
