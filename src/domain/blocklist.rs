use url::Url;

pub struct Blocklist {
    rules: Vec<String>,
}

impl Blocklist {
    pub fn from_text(content: &str) -> Self {
        let rules = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| line.to_ascii_lowercase())
            .collect();

        Self { rules }
    }

    pub fn blocks(&self, url: &str) -> bool {
        let parsed = match Self::parse_web_url(url) {
            Ok(parsed) => parsed,
            Err(_) => return true,
        };

        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        let full_url = parsed.as_str().to_ascii_lowercase();

        self.rules
            .iter()
            .any(|rule| host.contains(rule) || full_url.contains(rule))
    }

    fn parse_web_url(url: &str) -> Result<Url, String> {
        let parsed = Url::parse(url).map_err(|_| "invalid url".to_string())?;
        match parsed.scheme() {
            "http" | "https" => Ok(parsed),
            _ => Err("unsupported scheme".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Blocklist;

    #[test]
    fn blocks_matching_hosts() {
        let list = Blocklist::from_text("ads.example.com\ntracker");
        assert!(list.blocks("https://ads.example.com/banner"));
        assert!(list.blocks("https://safe.example.com/tracker.js"));
    }

    #[test]
    fn invalid_urls_are_blocked() {
        let list = Blocklist::from_text("anything");
        assert!(list.blocks("not-a-url"));
        assert!(list.blocks("ftp://example.com/file"));
    }

    #[test]
    fn allows_clean_urls() {
        let list = Blocklist::from_text("ads.example.com");
        assert!(!list.blocks("https://www.rust-lang.org"));
    }
}
