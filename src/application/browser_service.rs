use crate::domain::browser::{Browser, BrowserState};
use crate::domain::filter::UrlPolicy;
use crate::infrastructure::blocklist::file_blocklist_policy::FileBlocklistPolicy;

pub struct BrowserService {
    browser: Browser,
    url_policy: FileBlocklistPolicy,
}

impl BrowserService {
    pub fn new(url_policy: FileBlocklistPolicy) -> Self {
        let browser = Browser::new(BrowserState::default());
        Self { browser, url_policy }
    }

    pub fn prepare_navigation(&self, url: &str) -> NavigationDecision {
        let candidate = normalize_url(url);

        if !self.url_policy.allows(&candidate) {
            return NavigationDecision::Blocked { url: candidate };
        }

        NavigationDecision::Allowed { url: candidate }
    }

    pub fn commit_navigation(&mut self, url: &str) {
        self.browser.navigate(url);
    }

    pub fn replace_policy(&mut self, url_policy: FileBlocklistPolicy) {
        self.url_policy = url_policy;
    }

    pub fn url_policy(&self) -> FileBlocklistPolicy {
        self.url_policy.clone()
    }

    pub fn history(&self) -> &[String] {
        self.browser.history()
    }
}

pub enum NavigationDecision {
    Allowed { url: String },
    Blocked { url: String },
}

fn normalize_url(url: &str) -> String {
    normalize_user_url(url)
}

pub fn normalize_user_url(url: &str) -> String {
    let trimmed = url.trim();

    if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    }
}
