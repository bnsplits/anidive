use regex::Regex;
use std::time::Duration;

use crate::pages::animesama::extractors::{DownloadLink, DownloadLinkKind};

const ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn decode_base(token: &str, base: usize) -> Option<usize> {
    let mut n = 0usize;
    for c in token.chars() {
        let d = ALPHABET.find(c)?;
        if d >= base {
            return None;
        }
        n = n.checked_mul(base)?.checked_add(d)?;
    }
    Some(n)
}

fn unpack(body: &str) -> Option<String> {
    let re = Regex::new(
        r#"(?s)\}\('((?:[^'\\]|\\.)*)',\s*(\d+)\s*,\s*(\d+)\s*,\s*'((?:[^'\\]|\\.)*)'\.split\('\|'\)"#,
    )
    .ok()?;
    let c = re.captures(body)?;

    let packed = c[1].replace("\\'", "'");
    let base: usize = c[2].parse().ok()?;
    let words: Vec<&str> = c[4].split('|').collect();

    let re_tok = Regex::new(r"\b[0-9a-zA-Z]+\b").ok()?;
    let out = re_tok.replace_all(&packed, |t: &regex::Captures| {
        let token = &t[0];
        decode_base(token, base)
            .and_then(|i| words.get(i))
            .filter(|w| !w.is_empty())
            .map(|w| w.to_string())
            .unwrap_or_else(|| token.to_string())
    });

    Some(out.into_owned())
}

fn extract_hls_url(js: &str, domain: &str) -> Option<String> {
    for pat in [
        r#"https?://[^"']+master\.m3u8[^"']*"#,
        r#"https?://[^"']+\.m3u8[^"']*"#,
    ] {
        if let Some(m) = Regex::new(pat).ok()?.find(js) {
            return Some(m.as_str().to_string());
        }
    }

    let m = Regex::new(r#"/[^"']+master\.m3u8[^"']*"#).ok()?.find(js)?;
    Some(format!("https://{domain}{}", m.as_str()))
}

pub fn extract_uqload_video_source(embed_url: &str) -> Option<DownloadLink> {
    let code = Regex::new(r"/(?:embed-)?([a-zA-Z0-9]{8,20})(?:\.html)?")
        .ok()?
        .captures(embed_url)?[1]
        .to_string();

    let domain = Regex::new(r"https?://([^/]+)")
        .ok()?
        .captures(embed_url)
        .map(|c| c[1].to_string())
        .unwrap_or_else(|| "uqload.is".to_string());

    let clean_url = format!("https://{domain}/embed-{code}.html");

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let body = client
        .get(&clean_url)
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .ok()?;

    let re_direct = Regex::new(r#"["'](https?://[^\s"']+\.m3u8[^\s"']*)["']"#).ok()?;
    let url = match re_direct.captures(&body) {
        Some(c) => c[1].to_string(),
        None => extract_hls_url(&unpack(&body)?, &domain)?,
    };

    Some(DownloadLink {
        url,
        kind: DownloadLinkKind::M3U8,
    })
}
