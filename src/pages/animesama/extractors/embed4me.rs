use aes::Aes128;
use cbc::{
    Decryptor,
    cipher::{BlockModeDecrypt, KeyIvInit, block_padding::Pkcs7},
};
use regex::Regex;
use serde_json::Value;
use std::time::Duration;

use crate::pages::animesama::extractors::{DownloadLink, DownloadLinkKind};

const KEY: &[u8; 16] = b"kiemtienmua911ca";
const IV: &[u8; 16] = b"1234567890oiuytr";

fn decrypt_data(hex_str: &str) -> Option<String> {
    let mut data = hex::decode(hex_str).ok()?;
    let plain = Decryptor::<Aes128>::new(KEY.into(), IV.into())
        .decrypt_padded::<Pkcs7>(&mut data)
        .ok()?;
    String::from_utf8(plain.to_vec()).ok()
}

pub fn extract_embed4me_video_source(embed_url: &str) -> Option<DownloadLink> {
    let video_id = match Regex::new(r"#([a-zA-Z0-9]+)").ok()?.captures(embed_url) {
        Some(c) => c[1].to_string(),
        None => Regex::new(r"[?&]id=([a-zA-Z0-9]+)")
            .ok()?
            .captures(embed_url)?[1]
            .to_string(),
    };

    let api_url = format!(
        "https://lpayer.embed4me.com/api/v1/video?id={video_id}&w=1920&h=1080&r=https://lpayer.embed4me.com/"
    );

    let client = reqwest::blocking::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(Duration::from_secs(10))
        .build()
        .ok()?;

    let body = client
        .get(&api_url)
        .header("Referer", "https://lpayer.embed4me.com/")
        .send()
        .ok()?
        .error_for_status()
        .ok()?
        .text()
        .ok()?;

    let hex_data = body.trim().trim_matches('"');
    let decrypted = decrypt_data(hex_data)?;
    let data: Value = serde_json::from_str(&decrypted).ok()?;

    let url = ["cfNative", "cf", "source"]
        .iter()
        .filter_map(|k| data[k].as_str())
        .find(|s| !s.is_empty())?
        .to_string();

    let kind = if url.contains(".m3u8") {
        DownloadLinkKind::M3U8
    } else {
        DownloadLinkKind::Mp4
    };

    Some(DownloadLink { url, kind })
}
