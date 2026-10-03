use crate::constants::traits::AnimeProvider;

impl AnimeProvider for AnimeSamaState {}

impl Default for AnimeSamaState {
    fn default() -> Self {
        Self {
            current_page_path: String::new(),
            current_page_help: String::new(),
        }
    }
}

pub struct AnimeSamaState {
    current_page_path: String,
    current_page_help: String,
}
