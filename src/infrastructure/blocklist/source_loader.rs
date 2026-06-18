use reqwest::blocking::Client;
use std::path::Path;

pub struct BlocklistSourceLoader {
    client: Client,
}

impl BlocklistSourceLoader {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("rust-browser/0.1")
            .build()
            .map_err(|error| error.to_string())?;

        Ok(Self { client })
    }

    pub fn load(
        &self,
        local_path: &str,
        remote: Option<&RemoteBlocklistConfig>,
    ) -> Result<String, String> {
        let mut sources = Vec::new();

        if Path::new(local_path).exists() {
            let content = std::fs::read_to_string(local_path).map_err(|error| error.to_string())?;
            sources.push(content);
        }

        if let Some(remote) = remote {
            match self.fetch_remote(remote) {
                Ok(content) => sources.push(content),
                Err(remote_error) => {
                    if Path::new(remote.cache_path.as_str()).exists() {
                        let cached = std::fs::read_to_string(remote.cache_path.as_str())
                            .map_err(|error| error.to_string())?;
                        eprintln!(
                            "Remote blocklist fetch failed ({}). Falling back to cache at {}.",
                            remote_error, remote.cache_path
                        );
                        sources.push(cached);
                    } else if sources.is_empty() {
                        return Err(remote_error);
                    } else {
                        eprintln!(
                            "Remote blocklist fetch failed ({}). Continuing with local sources only.",
                            remote_error
                        );
                    }
                }
            }
        }

        if sources.is_empty() {
            return Err("No blocklist sources could be loaded.".to_string());
        }

        Ok(sources.join("\n"))
    }

    fn fetch_remote(&self, remote: &RemoteBlocklistConfig) -> Result<String, String> {
        let response = self
            .client
            .get(remote.url.as_str())
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|error| error.to_string())?;

        let content = response.text().map_err(|error| error.to_string())?;
        cache_remote_copy(remote.cache_path.as_str(), content.as_str())?;
        Ok(content)
    }
}

#[derive(Clone)]
pub struct RemoteBlocklistConfig {
    pub url: String,
    pub cache_path: String,
}

fn cache_remote_copy(path: &str, content: &str) -> Result<(), String> {
    let cache_path = Path::new(path);

    if let Some(parent) = cache_path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }

    std::fs::write(cache_path, content).map_err(|error| error.to_string())
}
