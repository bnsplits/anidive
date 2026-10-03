use std::time::Duration;

pub struct AnimeProviderState {
    pub name: String,
    pub full_path: Vec<String>,
    pub helps: Vec<(String, String)>,
    pub want_poll: bool,
    pub poll_timeout: Duration,
}
