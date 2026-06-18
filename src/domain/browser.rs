#[derive(Default)]
pub struct BrowserState {
    pub current_url: String,
    pub is_private: bool,
}

pub struct Browser {
    state: BrowserState,
}

impl Browser {
    pub fn new(state: BrowserState) -> Self {
        Self { state }
    }

    pub fn navigate(&mut self, url: &str) {
        self.state.current_url.clear();
        self.state.current_url.push_str(url);
    }

    pub fn state(&self) -> &BrowserState {
        &self.state
    }
}
