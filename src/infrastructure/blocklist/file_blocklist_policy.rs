use crate::domain::blocklist::Blocklist;
use crate::domain::filter::UrlPolicy;
use crate::infrastructure::blocklist::source_loader::{
    BlocklistSourceLoader, RemoteBlocklistConfig,
};

#[derive(Clone)]
pub struct FileBlocklistPolicy {
    blocklist: Blocklist,
}

impl FileBlocklistPolicy {
    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        Ok(Self {
            blocklist: Blocklist::from_text(&content),
        })
    }

    pub fn from_sources(
        loader: &BlocklistSourceLoader,
        local_path: &str,
        remote: Option<&RemoteBlocklistConfig>,
    ) -> Result<Self, String> {
        let content = loader.load(local_path, remote)?;
        Ok(Self {
            blocklist: Blocklist::from_text(&content),
        })
    }
}

impl UrlPolicy for FileBlocklistPolicy {
    fn allows(&self, url: &str) -> bool {
        !self.blocklist.blocks(url)
    }
}
