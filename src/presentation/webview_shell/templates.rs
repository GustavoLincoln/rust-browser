use crate::presentation::webview_shell::components::{
    render_card, render_profile_selector, render_tab_strip, render_toolbar, ProfileOptionProps,
    TabStripProps, ToolbarButtonProps,
};

pub const WINDOW_WIDTH: f64 = 1440.0;
pub const WINDOW_HEIGHT: f64 = 920.0;
pub const HEADER_HEIGHT: f64 = 108.0;
pub const SIDEBAR_WIDTH: f64 = 0.0;
pub const SETTINGS_PANEL_WIDTH: f64 = 380.0;

pub fn shell_html() -> String {
    let tabstrip = render_tab_strip(TabStripProps {
        tabs_id: "tabs",
        add_button_id: "new-tab-button",
    });
    let toolbar = render_toolbar(&[
        ToolbarButtonProps {
            id: "back-button",
            label: "Back",
            variant: "alt",
            aria_label: "Go back",
        },
        ToolbarButtonProps {
            id: "forward-button",
            label: "Forward",
            variant: "alt",
            aria_label: "Go forward",
        },
        ToolbarButtonProps {
            id: "home-button",
            label: "Home",
            variant: "alt",
            aria_label: "Open home page",
        },
        ToolbarButtonProps {
            id: "open-button",
            label: "Open",
            variant: "",
            aria_label: "Open current address",
        },
        ToolbarButtonProps {
            id: "reload-button",
            label: "Reload",
            variant: "alt",
            aria_label: "Reload current page",
        },
        ToolbarButtonProps {
            id: "settings-button",
            label: "Options",
            variant: "alt",
            aria_label: "Open browser options",
        },
    ]);
    let profile_selector = render_profile_selector(&[
        ProfileOptionProps {
            id: "profile-light",
            label: "Light",
            value: "Light",
        },
        ProfileOptionProps {
            id: "profile-normal",
            label: "Normal",
            value: "Normal",
        },
        ProfileOptionProps {
            id: "profile-pro",
            label: "Pro",
            value: "Pro",
        },
    ]);
    let current_page_card = render_card(
        "Current Page",
        r#"
        <h1 id="page-title">Rust Browser</h1>
        <p id="current-url" class="muted">No page loaded yet.</p>
        <p id="page-meta" class="muted" style="margin-top:8px;"></p>
        "#,
        "",
    );
    let profile_card = render_card(
        "Protection Profile",
        r#"<p class="muted">Choose how aggressive the remote DNS blocklist should be.</p>"#,
        profile_selector.as_str(),
    );
    let preview_card = render_card(
        "Readable Preview",
        r#"
        <p id="page-summary" class="muted">Open a page to fetch a structured summary beside the embedded browser.</p>
        <div id="sections"></div>
        "#,
        "",
    );
    let history_card = render_card("History", r#"<div id="history"></div>"#, "");
    let settings_tabs = r#"
      <div class="settings-tabs" role="tablist" aria-label="Options sections">
        <button id="settings-tab-security" class="settings-tab active" type="button" role="tab" aria-selected="true" data-settings-tab="security">Security</button>
        <button id="settings-tab-history" class="settings-tab" type="button" role="tab" aria-selected="false" data-settings-tab="history">History</button>
        <button id="settings-tab-preview" class="settings-tab" type="button" role="tab" aria-selected="false" data-settings-tab="preview">Preview</button>
      </div>
    "#;
    let settings_panel = format!(
        r#"<section id="settings-panel" class="settings-panel" aria-label="Browser options" aria-hidden="true">
        <div class="settings-shell">
          <div class="settings-header">
            <div>
              <p class="settings-kicker">Browser Options</p>
              <h2 class="settings-title">Privacy, navigation, and diagnostics</h2>
            </div>
            <button id="close-settings-button" class="icon-button" type="button" aria-label="Close options">Close</button>
          </div>
          {settings_tabs}
          <div class="settings-grid">
            <div class="settings-view" data-settings-view="security">
              {current_page_card}
              {profile_card}
            </div>
            <div class="settings-view is-hidden" data-settings-view="history">
              {history_card}
            </div>
            <div class="settings-view is-hidden" data-settings-view="preview">
              {preview_card}
            </div>
          </div>
        </div>
      </section>"#,
        settings_tabs = settings_tabs,
        current_page_card = current_page_card,
        profile_card = profile_card,
        preview_card = preview_card,
        history_card = history_card
    );

    format!(
        include_str!("shell.html"),
        header_height = HEADER_HEIGHT as i32,
        sidebar_width = SIDEBAR_WIDTH as i32,
        settings_panel_width = SETTINGS_PANEL_WIDTH as i32,
        tabstrip = tabstrip,
        toolbar = toolbar,
        settings_panel = settings_panel
    )
}

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
