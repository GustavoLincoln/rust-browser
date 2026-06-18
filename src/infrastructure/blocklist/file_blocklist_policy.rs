use crate::domain::blocklist::Blocklist;
use crate::domain::filter::UrlPolicy;

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
}

impl UrlPolicy for FileBlocklistPolicy {
    fn allows(&self, url: &str) -> bool {
        !self.blocklist.blocks(url)
    }
}
