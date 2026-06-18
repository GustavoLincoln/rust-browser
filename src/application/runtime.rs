use crate::application::browser_service::BrowserService;
use crate::infrastructure::blocklist::file_blocklist_policy::FileBlocklistPolicy;
use crate::infrastructure::blocklist::source_loader::{
    BlocklistSourceLoader, RemoteBlocklistConfig,
};

pub struct RuntimeConfig {
    pub local_blocklist_path: String,
    pub remote_blocklist_url: Option<String>,
    pub remote_blocklist_cache_path: String,
}

impl RuntimeConfig {
    pub fn from_env() -> Self {
        Self {
            local_blocklist_path: std::env::var("RUST_BROWSER_BLOCKLIST_PATH")
                .unwrap_or_else(|_| "blocklist.txt".to_string()),
            remote_blocklist_url: std::env::var("RUST_BROWSER_BLOCKLIST_URL").ok(),
            remote_blocklist_cache_path: std::env::var("RUST_BROWSER_BLOCKLIST_CACHE")
                .unwrap_or_else(|_| "target/blocklists/remote-domains.txt".to_string()),
        }
    }

    pub fn remote_config(&self) -> Option<RemoteBlocklistConfig> {
        self.remote_blocklist_url.as_ref().map(|url| RemoteBlocklistConfig {
            url: url.clone(),
            cache_path: self.remote_blocklist_cache_path.clone(),
        })
    }
}

pub fn bootstrap_browser_service(config: &RuntimeConfig) -> Result<BrowserService, String> {
    let loader = BlocklistSourceLoader::new()?;
    let policy = FileBlocklistPolicy::from_sources(
        &loader,
        config.local_blocklist_path.as_str(),
        config.remote_config().as_ref(),
    )?;

    Ok(BrowserService::new(policy))
}
