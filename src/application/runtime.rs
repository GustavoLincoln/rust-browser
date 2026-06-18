use crate::application::browser_service::BrowserService;
use crate::domain::blocklist_profile::BlocklistProfile;
use crate::infrastructure::blocklist::file_blocklist_policy::FileBlocklistPolicy;
use crate::infrastructure::blocklist::hagezi_catalog;
use crate::infrastructure::blocklist::source_loader::{
    BlocklistSourceLoader, RemoteBlocklistConfig,
};
use crate::infrastructure::config::app_settings::{AppSettings, AppSettingsStore};

pub struct RuntimeConfig {
    pub local_blocklist_path: String,
    pub remote_blocklist_url: Option<String>,
    pub remote_blocklist_cache_path: String,
    pub settings_path: String,
}

impl RuntimeConfig {
    pub fn from_env() -> Self {
        Self {
            local_blocklist_path: std::env::var("RUST_BROWSER_BLOCKLIST_PATH")
                .unwrap_or_else(|_| "blocklist.txt".to_string()),
            remote_blocklist_url: std::env::var("RUST_BROWSER_BLOCKLIST_URL").ok(),
            remote_blocklist_cache_path: std::env::var("RUST_BROWSER_BLOCKLIST_CACHE")
                .unwrap_or_else(|_| "target/blocklists/remote-domains.txt".to_string()),
            settings_path: std::env::var("RUST_BROWSER_SETTINGS_PATH")
                .unwrap_or_else(|_| "target/settings/app-settings.json".to_string()),
        }
    }

    pub fn remote_config(&self, profile: BlocklistProfile) -> RemoteBlocklistConfig {
        let url = self
            .remote_blocklist_url
            .clone()
            .unwrap_or_else(|| hagezi_catalog::profile_url(profile).to_string());

        RemoteBlocklistConfig {
            url,
            cache_path: self.remote_blocklist_cache_path.clone(),
        }
    }

    pub fn settings_store(&self) -> AppSettingsStore {
        AppSettingsStore::new(self.settings_path.clone())
    }
}

pub fn load_app_settings(config: &RuntimeConfig) -> Result<AppSettings, String> {
    config.settings_store().load()
}

pub fn save_app_settings(config: &RuntimeConfig, settings: &AppSettings) -> Result<(), String> {
    config.settings_store().save(settings)
}

pub fn bootstrap_browser_service(
    config: &RuntimeConfig,
    settings: &AppSettings,
) -> Result<BrowserService, String> {
    let policy = build_blocklist_policy(config, settings.blocklist_profile)?;
    Ok(BrowserService::new(policy))
}

pub fn build_blocklist_policy(
    config: &RuntimeConfig,
    profile: BlocklistProfile,
) -> Result<FileBlocklistPolicy, String> {
    let loader = BlocklistSourceLoader::new()?;
    FileBlocklistPolicy::from_sources(
        &loader,
        config.local_blocklist_path.as_str(),
        Some(&config.remote_config(profile)),
    )
}
