use crate::domain::browser::{Browser, BrowserState};
use crate::domain::filter::UrlPolicy;
use crate::infrastructure::blocklist::file_blocklist_policy::FileBlocklistPolicy;
use crate::infrastructure::storage::bookmark_store::BookmarkStore;

pub struct BrowserService {
    browser: Browser,
    url_policy: FileBlocklistPolicy,
    bookmark_store: BookmarkStore,
}

impl BrowserService {
    pub fn bootstrap(blocklist_path: &str, storage_path: &str) -> Result<Self, String> {
        let browser = Browser::new(BrowserState::default());
        let url_policy = FileBlocklistPolicy::from_file(blocklist_path)?;
        let bookmark_store = BookmarkStore::open(storage_path)?;

        Ok(Self {
            browser,
            url_policy,
            bookmark_store,
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

    pub fn current_url(&self) -> &str {
        self.browser.state().current_url.as_str()
    }

    pub fn add_bookmark(&self, url: &str, title: &str) -> Result<(), String> {
        self.bookmark_store.save(url, title)
    }
}

pub enum NavigationOutcome {
    Allowed,
    Blocked,
}
