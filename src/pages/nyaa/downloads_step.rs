use std::{
    fs,
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    thread::{self, sleep},
    time::Duration,
};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::Line,
    widgets::Gauge,
};
use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::{
    AppEvent, ThreadEvent,
    pages::{
        InputMode,
        nyaa::{NyaaDownloadStatus, NyaaEvent, NyaaPageStep, NyaaResult},
    },
};

const RPC_URL: &str = "http://127.0.0.1:6800/jsonrpc";

fn fmt_eta(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}h{m:02}m{s:02}s")
    } else {
        format!("{m:02}m{s:02}s")
    }
}

fn rpc(client: &Client, method: &str, params: Value) -> Option<Value> {
    let body = json!({ "jsonrpc": "2.0", "id": "1", "method": method, "params": params });
    let resp: Value = client.post(RPC_URL).json(&body).send().ok()?.json().ok()?;
    resp.get("result").cloned()
}

pub struct DownloadsStepState {
    pub torrent: NyaaResult,
    pub status: Option<NyaaDownloadStatus>,
    pub download_dir: PathBuf,
}

impl DownloadsStepState {
    /// View
    pub fn view(&mut self, _input_mode: InputMode, area: Rect, frame: &mut Frame) {
        let [name_layout, gauge_layout] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)])
                .spacing(1)
                .areas(area);

        frame.render_widget(
            Line::from(self.torrent.0.clone()).white().bold(),
            name_layout,
        );

        match &self.status {
            None => frame.render_widget(
                Line::from("Starting download...")
                    .centered()
                    .dark_gray()
                    .italic(),
                gauge_layout,
            ),

            Some(NyaaDownloadStatus::FetchingMetadata) => frame.render_widget(
                Line::from("Fetching torrent metadata...")
                    .centered()
                    .dark_gray()
                    .italic(),
                gauge_layout,
            ),

            Some(NyaaDownloadStatus::Running {
                percent,
                speed,
                eta,
            }) => {
                let gauge = Gauge::default()
                    .gauge_style(Style::new().fg(Color::Blue).bg(Color::Black))
                    .ratio(*percent as f64)
                    .label(
                        format!(
                            "[{:.0}%] - [{}] - [ETA: {}]",
                            percent * 100.0,
                            if speed.is_empty() { "0B/s" } else { speed },
                            if eta.is_empty() { "--:--" } else { eta }
                        )
                        .bold(),
                    );
                frame.render_widget(gauge, gauge_layout);
            }

            Some(NyaaDownloadStatus::Done) => frame.render_widget(
                Line::from("Download complete").centered().light_green(),
                gauge_layout,
            ),

            Some(NyaaDownloadStatus::Failed) => {
                frame.render_widget(Line::from("Download failed").centered().red(), gauge_layout)
            }
        }
    }

    /// Handle crossterm keys
    pub fn handle_crossterm_key(&mut self, key_event: KeyEvent) -> Option<NyaaPageStep> {
        if key_event.code == KeyCode::Enter {
            let _ = Command::new("xdg-open")
                .arg(&self.download_dir)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn();
        }

        None
    }

    /// Handle thread events
    pub fn handle_thread_event(&mut self, event: &NyaaEvent) {
        if let NyaaEvent::Downloads { status } = event {
            self.status = Some(status.clone());
        }
    }

    /// Start download
    pub fn start_downloads(&mut self, tx: mpsc::Sender<ThreadEvent>) {
        let magnet_link = self.torrent.1.clone();
        let download_dir = self.download_dir.clone();
        let send_tx = tx.clone();

        let _ = tx.send(ThreadEvent::AppEvent(AppEvent::DisableRestart));

        thread::spawn(move || {
            let dir_created = fs::create_dir_all(&download_dir).is_ok();
            let mut child = if dir_created {
                Command::new("aria2c")
                    .args([
                        "--enable-rpc",
                        "--rpc-listen-port=6800",
                        "--quiet",
                        "--seed-time=0",
                    ])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                    .ok()
            } else {
                None
            };

            if child.is_none() {
                let _ = send_tx.send(ThreadEvent::NyaaEvent(NyaaEvent::Downloads {
                    status: NyaaDownloadStatus::Failed,
                }));
                let _ = send_tx.send(ThreadEvent::AppEvent(AppEvent::EnableRestart));
                return;
            }

            let child = child.as_mut().unwrap();
            let client = Client::new();
            let dir = download_dir.to_string_lossy().to_string();

            let result = (|| -> Option<()> {
                /* Wait for the RPC server to come up (max ~5 s) */
                let mut up = false;
                for _ in 0..50 {
                    if rpc(&client, "aria2.getVersion", json!([])).is_some() {
                        up = true;
                        break;
                    }
                    sleep(Duration::from_millis(100));
                }
                if !up {
                    return None;
                }

                let mut gid = rpc(
                    &client,
                    "aria2.addUri",
                    json!([[magnet_link], { "dir": dir, "seed-time": "0" }]),
                )?
                .as_str()?
                .to_string();
                let mut metadata_done = false;

                let _ = send_tx.send(ThreadEvent::NyaaEvent(NyaaEvent::Downloads {
                    status: NyaaDownloadStatus::FetchingMetadata,
                }));

                loop {
                    sleep(Duration::from_millis(250));

                    let st = rpc(
                        &client,
                        "aria2.tellStatus",
                        json!([
                            gid,
                            [
                                "status",
                                "totalLength",
                                "completedLength",
                                "downloadSpeed",
                                "followedBy"
                            ]
                        ]),
                    )?;

                    /* Magnet downloads first fetch metadata, then create a new GID for the files */
                    if !metadata_done {
                        let followed_gid = st["followedBy"]
                            .as_array()
                            .and_then(|gids| gids.first())
                            .and_then(|gid| gid.as_str())
                            .or_else(|| st["followedBy"].as_str())
                            .map(str::to_string);

                        if let Some(followed_gid) = followed_gid {
                            gid = followed_gid;
                            metadata_done = true;
                            continue;
                        }
                    }

                    let num = |k: &str| {
                        st[k]
                            .as_str()
                            .and_then(|s| s.parse::<f64>().ok())
                            .unwrap_or(0.0)
                    };
                    let total = num("totalLength");
                    let percent = if total > 0.0 {
                        (num("completedLength") / total) as f32
                    } else {
                        0.0
                    };
                    let speed = num("downloadSpeed");
                    let eta = if speed > 0.0 {
                        fmt_eta(((total - num("completedLength")) / speed) as u64)
                    } else {
                        "--:--".to_string()
                    };

                    let _ = send_tx.send(ThreadEvent::NyaaEvent(NyaaEvent::Downloads {
                        status: NyaaDownloadStatus::Running {
                            percent,
                            speed: format!("{:.1} MB/s", speed / 1_000_000.0),
                            eta,
                        },
                    }));

                    match st["status"].as_str()? {
                        "complete" if metadata_done => return Some(()),
                        "complete" | "error" | "removed" => return None,
                        _ => {}
                    }
                }
            })();

            let _ = child.kill();
            let _ = child.wait();

            let status = if result.is_some() {
                NyaaDownloadStatus::Done
            } else {
                NyaaDownloadStatus::Failed
            };

            let _ = send_tx.send(ThreadEvent::NyaaEvent(NyaaEvent::Downloads { status }));
            let _ = send_tx.send(ThreadEvent::AppEvent(AppEvent::EnableRestart));
        });
    }
}
