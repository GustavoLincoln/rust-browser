use crate::application::ports::PageLoader;
use crate::domain::page::{Page, PageSection};
use reqwest::blocking::Client;
use scraper::{Html, Selector};

pub struct HttpPageFetcher {
    client: Client,
}

impl HttpPageFetcher {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .user_agent("rust-browser/0.1")
            .build()
            .map_err(|error| error.to_string())?;

        Ok(Self { client })
    }
}

impl PageLoader for HttpPageFetcher {
    fn load(&self, url: &str) -> Result<Page, String> {
        let response = self.client.get(url).send().map_err(|error| error.to_string())?;
        let status = response.status();
        let html = response.text().map_err(|error| error.to_string())?;
        let document = Html::parse_document(&html);
        let extracted = extract_readable_content(&document);

        Ok(Page {
            url: url.to_string(),
            title: extract_title(&document).unwrap_or_else(|| "Untitled page".to_string()),
            summary: extracted.summary,
            status: status.to_string(),
            sections: extracted.sections,
        })
    }
}

fn extract_title(document: &Html) -> Option<String> {
    let selector = Selector::parse("title").expect("valid selector");
    let node = document.select(&selector).next()?;
    let title = normalize_whitespace(&node.text().collect::<Vec<_>>().join(" "));

    (!title.is_empty()).then_some(title)
}

fn extract_readable_content(document: &Html) -> ExtractedContent {
    let content_selector = Selector::parse("article, main, section, body").expect("valid selector");
    let heading_selector = Selector::parse("h1, h2, h3").expect("valid selector");
    let text_selector = Selector::parse("p, li, blockquote, pre").expect("valid selector");

    let root = document
        .select(&content_selector)
        .max_by_key(|node| node.text().collect::<String>().len());

    let mut sections = Vec::new();

    if let Some(root) = root {
        let headings = root
            .select(&heading_selector)
            .map(|node| normalize_whitespace(&node.text().collect::<Vec<_>>().join(" ")))
            .filter(|text| !text.is_empty())
            .take(8)
            .collect::<Vec<_>>();

        let paragraphs = root
            .select(&text_selector)
            .map(|node| normalize_whitespace(&node.text().collect::<Vec<_>>().join(" ")))
            .filter(|text| text.len() > 30)
            .take(24)
            .collect::<Vec<_>>();

        if !headings.is_empty() && !paragraphs.is_empty() {
            for (index, chunk) in paragraphs.chunks(3).enumerate() {
                sections.push(PageSection {
                    heading: headings
                        .get(index)
                        .cloned()
                        .unwrap_or_else(|| format!("Section {}", index + 1)),
                    body: chunk.join("\n\n"),
                });
            }
        } else if !paragraphs.is_empty() {
            sections.push(PageSection {
                heading: "Page content".to_string(),
                body: paragraphs.join("\n\n"),
            });
        }
    }

    if sections.is_empty() {
        let fallback = extract_fallback_text(document);
        sections.push(PageSection {
            heading: "Page content".to_string(),
            body: fallback.clone(),
        });
    }

    let summary = sections
        .first()
        .map(|section| truncate(&section.body, 220))
        .unwrap_or_else(|| "No readable text content found for this page.".to_string());

    ExtractedContent {
        summary,
        sections,
    }
}

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_fallback_text(document: &Html) -> String {
    let body_selector = Selector::parse("body").expect("valid selector");
    let text = document
        .select(&body_selector)
        .next()
        .map(|node| normalize_whitespace(&node.text().collect::<Vec<_>>().join(" ")))
        .unwrap_or_else(|| "No readable text content found for this page.".to_string());

    truncate(&text, 8_000)
}

fn truncate(input: &str, max_chars: usize) -> String {
    let truncated = input.chars().take(max_chars).collect::<String>();
    if truncated.is_empty() {
        "No readable text content found for this page.".to_string()
    } else {
        truncated
    }
}

struct ExtractedContent {
    summary: String,
    sections: Vec<PageSection>,
}
