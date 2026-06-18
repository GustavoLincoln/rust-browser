#[derive(Default)]
pub struct BrowserState {
    pub current_url: String,
    pub history: Vec<String>,
}

pub struct Browser {
    state: BrowserState,
}

impl Browser {
    pub fn new(state: BrowserState) -> Self {
        Self { state }
    }

    pub fn navigate(&mut self, url: &str) {
        if self.state.current_url == url {
            return;
        }

        if !self.state.current_url.is_empty() {
            self.state.history.push(self.state.current_url.clone());
        }

        self.state.current_url = url.to_string();
    }
    pub fn history(&self) -> &[String] {
        self.state.history.as_slice()
    }
}
