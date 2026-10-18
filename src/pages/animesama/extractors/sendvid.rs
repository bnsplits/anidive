use crate::pages::animesama::extractors::{DownloadLink, DownloadLinkKind};
use regex::Regex;

pub fn extract_sendvid_video_source(url: &str) -> Option<DownloadLink> {
    let body = reqwest::blocking::get(url).ok()?.text().ok()?;
    let re = Regex::new(r#"var\s+video_source\s*=\s*"([^"]+)""#).ok()?;
    let url = re.captures(&body)?[1].to_string();

    Some(DownloadLink {
        url,
        kind: DownloadLinkKind::Mp4,
    })
}
