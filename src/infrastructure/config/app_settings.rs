use crate::domain::blocklist_profile::BlocklistProfile;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub blocklist_profile: BlocklistProfile,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            blocklist_profile: BlocklistProfile::Normal,
        }
    }
}

pub struct AppSettingsStore {
    path: String,
}

impl AppSettingsStore {
    pub fn new(path: String) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<AppSettings, String> {
        let path = Path::new(self.path.as_str());
        if !path.exists() {
            return Ok(AppSettings::default());
        }

        let content = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        serde_json::from_str(&content).map_err(|error| error.to_string())
    }

    pub fn save(&self, settings: &AppSettings) -> Result<(), String> {
        let path = Path::new(self.path.as_str());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }

        let content = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;
        std::fs::write(path, content).map_err(|error| error.to_string())
    }
}
