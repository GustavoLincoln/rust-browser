use crate::domain::browser::{Browser, BrowserState};
use crate::domain::filter::UrlPolicy;
use crate::infrastructure::blocklist::file_blocklist_policy::FileBlocklistPolicy;

pub struct BrowserService {
    browser: Browser,
    url_policy: FileBlocklistPolicy,
}

impl BrowserService {
    pub fn bootstrap(blocklist_path: &str) -> Result<Self, String> {
        let browser = Browser::new(BrowserState::default());
        let url_policy = FileBlocklistPolicy::from_file(blocklist_path)?;

        Ok(Self {
            browser,
            url_policy,
        })
    }

    pub fn navigate(&mut self, url: &str) -> NavigationOutcome {
        if self.url_policy.allows(url) {
            self.browser.navigate(url);
            NavigationOutcome::Allowed
        } else {
            NavigationOutcome::Blocked
        }
    }
}

pub enum NavigationOutcome {
    Allowed,
    Blocked,
}
