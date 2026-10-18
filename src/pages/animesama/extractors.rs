pub mod ansembed;
pub mod embed4me;
pub mod sendvid;
pub mod uqload;
pub mod voe;

#[derive(Debug)]
pub struct DownloadLink {
    pub url: String,
    pub kind: DownloadLinkKind,
}

#[derive(Debug)]
pub enum DownloadLinkKind {
    M3U8,
    Mp4,
}
