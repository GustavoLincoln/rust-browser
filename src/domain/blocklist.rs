use url::Url;

#[derive(Clone)]
pub struct Blocklist {
    rules: Vec<DomainRule>,
}

impl Blocklist {
    pub fn from_text(content: &str) -> Self {
        let rules = content.lines().filter_map(DomainRule::parse).collect();
        Self { rules }
    }

    pub fn blocks(&self, url: &str) -> bool {
        let parsed = match Self::parse_web_url(url) {
            Ok(parsed) => parsed,
            Err(_) => return true,
        };

        let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
        self.rules.iter().any(|rule| rule.matches(host.as_str()))
    }

    fn parse_web_url(url: &str) -> Result<Url, String> {
        let parsed = Url::parse(url).map_err(|_| "invalid url".to_string())?;
        match parsed.scheme() {
            "http" | "https" => Ok(parsed),
            _ => Err("unsupported scheme".to_string()),
        }
    }
}

#[derive(Clone)]
struct DomainRule {
    host: String,
}

impl DomainRule {
    fn parse(line: &str) -> Option<Self> {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with('!')
            || trimmed.starts_with('[')
        {
            return None;
        }

        let candidate = trimmed
            .split_whitespace()
            .last()
            .unwrap_or(trimmed)
            .trim_start_matches("||")
            .trim_start_matches("*.")
            .trim_end_matches('^')
            .trim_end_matches('.')
            .to_ascii_lowercase();

        if candidate.is_empty()
            || candidate == "localhost"
            || candidate.parse::<std::net::IpAddr>().is_ok()
        {
            return None;
        }

        Some(Self { host: candidate })
    }

    fn matches(&self, host: &str) -> bool {
        host == self.host || host.ends_with(&format!(".{}", self.host))
    }
}

#[cfg(test)]
mod tests {
    use super::Blocklist;

    #[test]
    fn blocks_matching_hosts_and_subdomains() {
        let list = Blocklist::from_text("example.com\nads.example.net");
        assert!(list.blocks("https://ads.example.com/banner"));
        assert!(list.blocks("https://safe.example.com/path"));
        assert!(list.blocks("https://ads.example.net/script.js"));
    }

    #[test]
    fn does_not_match_partial_substrings() {
        let list = Blocklist::from_text("example.com");
        assert!(!list.blocks("https://goodexample.com"));
        assert!(!list.blocks("https://another-example.com"));
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

    #[test]
    fn parses_hosts_and_domain_formats() {
        let list = Blocklist::from_text(
            r#"
            # comment
            0.0.0.0 metrics.example.com
            ||tracker.example.org^
            *.ads.example.net
            "#,
        );

        assert!(list.blocks("https://metrics.example.com"));
        assert!(list.blocks("https://tracker.example.org/collect"));
        assert!(list.blocks("https://sub.ads.example.net"));
    }
}
