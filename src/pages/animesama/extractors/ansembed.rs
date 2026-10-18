use regex::Regex;

use crate::pages::animesama::extractors::{DownloadLink, DownloadLinkKind};

pub fn extract_ansembed_video_source(url: &str) -> Option<DownloadLink> {
    let body = reqwest::blocking::get(url).ok()?.text().ok()?;

    let re_m3u8 = Regex::new(r#"['"](https?://[^\s'"]+\.m3u8[^\s'"]*)['"]"#).ok()?;
    let good = re_m3u8
        .captures_iter(&body)
        .map(|c| c[1].to_string())
        .find(|m| {
            let l = m.to_lowercase();
            !l.contains("bigbuckbunny") && !l.contains("sample")
        });
    if let Some(url) = good {
        return Some(DownloadLink {
            url,
            kind: DownloadLinkKind::M3U8,
        });
    }

    let re_file = Regex::new(r#"file\s*:\s*["']?(https?://[^\s'",]+)["']?"#).ok()?;
    if let Some(c) = re_file.captures(&body) {
        let url = c[1].to_string();
        let kind = if url.contains(".m3u8") {
            DownloadLinkKind::M3U8
        } else {
            DownloadLinkKind::Mp4
        };
        return Some(DownloadLink { url, kind });
    }

    let url = re_m3u8.captures(&body)?[1].to_string();
    Some(DownloadLink {
        url,
        kind: DownloadLinkKind::M3U8,
    })
}
