pub const HEADER_HEIGHT: f64 = 108.0;
pub const SETTINGS_PANEL_WIDTH: f64 = 380.0;

pub fn placeholder_page_html() -> String {
    include_str!("placeholder.html").to_string()
}

pub fn blocked_page_html(url: &str) -> String {
    format!(include_str!("blocked.html"), escape_html(url))
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
