pub struct TabStripProps<'a> {
    pub tabs_id: &'a str,
    pub add_button_id: &'a str,
}

pub struct ToolbarButtonProps<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub variant: &'a str,
    pub aria_label: &'a str,
}

pub struct ProfileOptionProps<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub value: &'a str,
}

pub fn render_tab_strip(props: TabStripProps<'_>) -> String {
    format!(
        r#"<div class="tabstrip-shell">
        <div id="{tabs_id}" class="tabstrip" role="tablist" aria-label="Open tabs"></div>
        <button id="{add_button_id}" class="tab-action" type="button" aria-label="Open new tab">+</button>
      </div>"#,
        tabs_id = props.tabs_id,
        add_button_id = props.add_button_id
    )
}

pub fn render_toolbar(buttons: &[ToolbarButtonProps<'_>]) -> String {
    let nav_buttons = buttons
        .iter()
        .filter(|button| matches!(button.id, "back-button" | "forward-button" | "home-button"))
        .map(render_toolbar_button)
        .collect::<Vec<_>>()
        .join("");

    let action_buttons = buttons
        .iter()
        .filter(|button| {
            matches!(
                button.id,
                "open-button" | "reload-button" | "settings-button"
            )
        })
        .map(render_toolbar_button)
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="toolbar" role="toolbar" aria-label="Navigation controls">
        <div class="toolbar-group">{nav_buttons}</div>
        <div class="address-shell">
          <label class="sr-only" for="address">Address bar</label>
          <input id="address" autocomplete="off" spellcheck="false" placeholder="Search or enter address" aria-label="Address bar" />
        </div>
        <div class="toolbar-group action-shell">{action_buttons}</div>
      </div>"#,
        nav_buttons = nav_buttons,
        action_buttons = action_buttons
    )
}

pub fn render_toolbar_button(props: &ToolbarButtonProps<'_>) -> String {
    let class = if props.variant.is_empty() {
        String::new()
    } else {
        format!(" class=\"{}\"", props.variant)
    };

    format!(
        r#"<button id="{id}"{class} type="button" aria-label="{aria_label}">{label}</button>"#,
        id = props.id,
        class = class,
        aria_label = props.aria_label,
        label = props.label
    )
}

pub fn render_profile_selector(options: &[ProfileOptionProps<'_>]) -> String {
    let chips = options
        .iter()
        .map(|option| {
            format!(
                r#"<button id="{id}" class="profile-chip" type="button" aria-pressed="false" data-profile="{value}">{label}</button>"#,
                id = option.id,
                value = option.value,
                label = option.label
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="profile-row" role="group" aria-label="Blocklist profile selector">{chips}</div>"#,
        chips = chips
    )
}

pub fn render_card(title: &str, content: &str, footer: &str) -> String {
    format!(
        r#"<section class="card">
        <h2>{title}</h2>
        {content}
        {footer}
      </section>"#,
        title = title,
        content = content,
        footer = footer
    )
}
