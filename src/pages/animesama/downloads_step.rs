use std::{
    fs,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::Gauge,
};

use crate::{
    ThreadEvent,
    pages::animesama::{
        AnimePageStep, AnimeSamaEvent, DownloadStatus, Episode, EpisodeProvider, downloaders,
        extractors::{DownloadLinkKind, ansembed, embed4me, sendvid, uqload, voe},
    },
};

#[allow(unused)]
pub struct DownloadsStepState {
    pub episodes: Vec<Episode>,
    pub current_episode: Option<Episode>,
    pub range: String,
    pub full_dir_path: Vec<String>,
}

impl DownloadsStepState {
    /// View
    pub fn view(&mut self, area: Rect, frame: &mut Frame) {
        let [completed_area, gauge_area] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
                .spacing(1)
                .areas(area);

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

        let all_done = completed.len() + failed.len() == self.episodes.len();

        if all_done {
            let text = Text::from(
                "
   █████████   ████  ████         █████                                 ███
  ███░░░░░███ ░░███ ░░███        ░░███                                 ░███
 ░███    ░███  ░███  ░███      ███████   ██████  ████████    ██████    ░███
 ░███████████  ░███  ░███     ███░░███  ███░░███░░███░░███  ███░░███   ░███
 ░███░░░░░███  ░███  ░███    ░███ ░███ ░███ ░███ ░███ ░███ ░███████    ░███
 ░███    ░███  ░███  ░███    ░███ ░███ ░███ ░███ ░███ ░███ ░███░░░     ░░░
 █████   █████ █████ █████   ░░████████░░██████  ████ █████░░██████     ███
░░░░░   ░░░░░ ░░░░░ ░░░░░     ░░░░░░░░  ░░░░░░  ░░░░ ░░░░░  ░░░░░░     ░░░
",
            )
            .cyan();

            let area = area.centered(
                Constraint::Length(text.width() as u16),
                Constraint::Length(text.height() as u16),
            );
            frame.render_widget(text, area);
        } else {
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
                    .label(format!(
                        "[{}]  [{:.0}%]  [{}]  [Eta: {}]",
                        episode.name,
                        percent * 100.0,
                        if speed.is_empty() { "0B/s" } else { speed },
                        eta
                    ));
                frame.render_widget(gauge, gauge_area);
            }
        }

        frame.render_widget(completed_row, completed_area);
    }

    /// Handle crossterm keys
    pub fn handle_crossterm_key(&mut self, key_event: KeyEvent) -> Option<AnimePageStep> {
        if key_event.code == KeyCode::Enter {
            let _ = Command::new("xdg-open")
                .arg(crate::download_dir().join(self.full_dir_path.join("/")))
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        None
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &AnimeSamaEvent) {
        if let AnimeSamaEvent::Downloads { current_episode } = event {
            self.current_episode = current_episode.clone();

            /* If the arrived episode is Done | Failed, then it updates the episodes list */
            if let Some(episode) = &self.current_episode
                && matches!(
                    episode.status,
                    DownloadStatus::Done | DownloadStatus::Failed
                )
                && let Some(slot) = self.episodes.iter_mut().find(|e| e.name == episode.name)
            {
                *slot = episode.clone();
            }
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
