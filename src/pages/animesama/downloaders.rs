use crate::{
    ThreadEvent,
    pages::animesama::{AnimeSamaEvent, DownloadStatus, Episode},
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc::Sender,
    thread::sleep,
    time::Duration,
};

const RPC_URL: &str = "http://127.0.0.1:6800/jsonrpc";

fn fmt_eta(secs: u64) -> String {
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m:02}:{s:02}")
    }
}

fn rpc(client: &reqwest::blocking::Client, method: &str, params: Value) -> Option<Value> {
    let body = json!({ "jsonrpc": "2.0", "id": "1", "method": method, "params": params });
    let resp: Value = client.post(RPC_URL).json(&body).send().ok()?.json().ok()?;
    resp.get("result").cloned()
}

pub fn download_mp4(episode: &Episode, extracted_url: &str, tx: Sender<ThreadEvent>) -> Option<()> {
    let DownloadStatus::Running { path, .. } = &episode.status else {
        return None;
    };

    let mut child: Child = Command::new("aria2c")
        .args(["--enable-rpc", "--rpc-listen-port=6800", "--quiet"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let client = reqwest::blocking::Client::new();

    let result = (|| {
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

        let file_name = format!("{}.mp4", path.file_name()?.to_str()?);
        let gid = rpc(
            &client,
            "aria2.addUri",
            json!([
                [extracted_url],
                {
                    "dir": path.parent()?.to_str()?,
                    "out": file_name,
                    "split": "16",
                    "max-connection-per-server": "16",
                    "min-split-size": "1M",
                    "allow-overwrite": "true"
                }
            ]),
        )?
        .as_str()?
        .to_string();

        loop {
            sleep(Duration::from_millis(250));

            let st = rpc(
                &client,
                "aria2.tellStatus",
                json!([
                    gid,
                    ["status", "totalLength", "completedLength", "downloadSpeed"]
                ]),
            )?;

            /* aria2 returns every number as a string */
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

            let e = Episode {
                name: episode.name.clone(),
                url: episode.url.clone(),
                provider: episode.provider,
                status: DownloadStatus::Running {
                    percent,
                    speed: format!("{:.1} MB/s", num("downloadSpeed") / 1_000_000.0),
                    path: path.clone(),
                    eta,
                },
            };

            tx.send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Downloads {
                current_episode: Some(e),
            }))
            .ok()?;

            match st["status"].as_str()? {
                "complete" => return Some(()),
                "error" | "removed" => return None,
                _ => {}
            }
        }
    })();

    let _ = child.kill();
    let _ = child.wait();
    result
}

pub fn download_m3u8(episode: &Episode, m3u8_url: &str, tx: Sender<ThreadEvent>) -> Option<()> {
    let DownloadStatus::Running { path, .. } = &episode.status else {
        return Some(());
    };

    let tmp_dir = std::env::temp_dir().join("yt-dlp-tmp");
    std::fs::create_dir_all(&tmp_dir).ok()?;

    let file_path = path.with_extension("mp4");
    let dir = file_path.parent()?;
    let file_name = file_path.file_name()?.to_str()?.replace('%', "%%");

    let mut child = Command::new("yt-dlp")
    .args([
        "--quiet",
        "--progress",
        "--newline",
        "--no-colors",
        "--force-overwrites",
        "--concurrent-fragments",
        "8",
        "--progress-template",
        "download:PROGRESS|%(progress._percent_str)s|%(progress._speed_str)s|%(progress._eta_str)s",
    ])
    .arg("--paths")
    .arg(format!("home:{}", dir.display()))
    .arg("--paths")
    .arg(format!("temp:{}", tmp_dir.display()))
    .arg("-o")
    .arg(file_name)
    .arg(m3u8_url)
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::null())
    .spawn()
    .ok()?;

    let stdout = child.stdout.take()?;

    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        let Some(rest) = line.strip_prefix("PROGRESS|") else {
            continue;
        };

        let mut parts = rest.split('|').map(str::trim);
        let percent = parts
            .next()
            .unwrap_or("0")
            .trim_end_matches('%')
            .parse::<f32>()
            .unwrap_or(0.0)
            / 100.0;
        let speed = parts.next().unwrap_or("").to_string();
        let eta = parts.next().unwrap_or("").to_string();

        let e = Episode {
            name: episode.name.clone(),
            url: episode.url.clone(),
            provider: episode.provider,
            status: DownloadStatus::Running {
                percent,
                speed,
                eta,
                path: path.clone(),
            },
        };

        if tx
            .send(ThreadEvent::AnimeSamaEvent(AnimeSamaEvent::Downloads {
                current_episode: Some(e),
            }))
            .is_err()
        {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        }
    }

    child.wait().ok()?.success().then_some(())
}
