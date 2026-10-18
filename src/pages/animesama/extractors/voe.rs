use base64::{Engine, engine::general_purpose::STANDARD};
use regex::Regex;
use serde_json::Value;
use std::time::Duration;

use crate::pages::animesama::extractors::{DownloadLink, DownloadLinkKind};

fn rot13(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' => ((c as u8 - b'A' + 13) % 26 + b'A') as char,
            'a'..='z' => ((c as u8 - b'a' + 13) % 26 + b'a') as char,
            _ => c,
        })
        .collect()
}

fn b64_to_string(s: &str) -> Option<String> {
    String::from_utf8(STANDARD.decode(s).ok()?).ok()
}

fn voe_decode_payload(payload: &str) -> Option<Value> {
    let mut s = rot13(payload);
    for op in ["@$", "^^", "~@", "%?", "*~", "!!", "#&"] {
        s = s.replace(op, "");
    }

    let s = b64_to_string(&s)?;

    let s: String = s
        .chars()
        .map(|c| char::from_u32((c as u32).checked_sub(3)?))
        .collect::<Option<Vec<char>>>()?
        .into_iter()
        .rev()
        .collect();

    serde_json::from_str(&b64_to_string(&s)?).ok()
}

fn is_sample(url: &str) -> bool {
    let l = url.to_lowercase();
    l.contains("bigbuckbunny") || l.contains("sample")
}

pub fn extract_voe_video_source(single_url: &str) -> Option<DownloadLink> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(8))
        .build()
        .ok()?;

    let re_loc = Regex::new(r#"window\.location\.href\s*=\s*["']([^"']+)["']"#).ok()?;

    let mut url = single_url.to_string();
    let mut html = String::new();
    for _ in 0..5 {
        let res = client
            .get(&url)
            .header("Referer", "https://nakanime.tv/")
            .send()
            .ok()?
            .error_for_status()
            .ok()?;
        let final_url = res.url().to_string();
        html = res.text().ok()?;

        if let Some(c) = re_loc.captures(&html) {
            let next = &c[1];
            if next != url && next != final_url {
                url = next.to_string();
                continue;
            }
        }
        break;
    }

    let re_json =
        Regex::new(r#"<script type="application/json">\s*(\[[\s\S]*?\])\s*</script>"#).ok()?;
    if let Some(c) = re_json.captures(&html) {
        let stream = serde_json::from_str::<Value>(&c[1])
            .ok()
            .and_then(|arr| voe_decode_payload(arr.get(0)?.as_str()?))
            .and_then(|p| {
                ["source", "direct_access_url"]
                    .iter()
                    .filter_map(|k| p[k].as_str())
                    .find(|s| !s.is_empty())
                    .map(str::to_string)
            })
            .filter(|s| !is_sample(s));

        if let Some(url) = stream {
            let kind = if url.contains(".m3u8") {
                DownloadLinkKind::M3U8
            } else {
                DownloadLinkKind::Mp4
            };
            return Some(DownloadLink { url, kind });
        }
    }

    let re_m3u8 = Regex::new(r#"['"](https?://[^\s'"]+\.m3u8[^\s'"]*)['"]"#).ok()?;
    let url = re_m3u8
        .captures_iter(&html)
        .map(|c| c[1].to_string())
        .find(|m| !is_sample(m))?;

    Some(DownloadLink {
        url,
        kind: DownloadLinkKind::M3U8,
    })
}
