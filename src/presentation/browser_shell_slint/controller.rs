#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserShellViewModel {
    pub window_title: String,
    pub tabs: Vec<BrowserTabViewModel>,
    pub toolbar: ToolbarViewModel,
    pub options_dialog: OptionsDialogViewModel,
    pub status: StatusViewModel,
}

impl Default for BrowserShellViewModel {
    fn default() -> Self {
        Self {
            window_title: "Rust Browser".to_string(),
            tabs: vec![BrowserTabViewModel::default()],
            toolbar: ToolbarViewModel::default(),
            options_dialog: OptionsDialogViewModel::default(),
            status: StatusViewModel::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BrowserTabViewModel {
    pub id: u32,
    pub title: String,
    pub url: String,
    pub is_active: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolbarViewModel {
    pub address_value: String,
    pub is_loading: bool,
    pub can_interact: bool,
    pub active_tab_title: String,
    pub active_tab_url: String,
}

impl Default for ToolbarViewModel {
    fn default() -> Self {
        Self {
            address_value: String::new(),
            is_loading: false,
            can_interact: true,
            active_tab_title: "New Tab".to_string(),
            active_tab_url: "about:blank".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OptionsDialogViewModel {
    pub is_open: bool,
    pub active_section: String,
    pub selected_profile: String,
    pub page_title: String,
    pub current_url: String,
    pub page_meta: String,
    pub page_summary: String,
    pub history: Vec<String>,
    pub preview_sections: Vec<PreviewSectionViewModel>,
    pub history_text: String,
    pub preview_text: String,
}

impl Default for OptionsDialogViewModel {
    fn default() -> Self {
        Self {
            is_open: false,
            active_section: "security".to_string(),
            selected_profile: "Normal".to_string(),
            page_title: "Rust Browser".to_string(),
            current_url: "No page loaded yet.".to_string(),
            page_meta: String::new(),
            page_summary: "Open a page to fetch a structured summary beside the embedded browser."
                .to_string(),
            history: Vec::new(),
            preview_sections: Vec::new(),
            history_text: "History is empty.".to_string(),
            preview_text: "No preview sections available for this page yet.".to_string(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PreviewSectionViewModel {
    pub heading: String,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusViewModel {
    pub kind: String,
    pub message: String,
}

impl Default for StatusViewModel {
    fn default() -> Self {
        Self {
            kind: "info".to_string(),
            message: "Ready to browse with the embedded WebView2 shell.".to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserShellStateInput {
    pub tabs: Vec<TabStateInput>,
    pub active_tab_id: u32,
    pub address_input: String,
    pub current_url: Option<String>,
    pub pending_url: Option<String>,
    pub page_title: String,
    pub page_meta: String,
    pub page_summary: String,
    pub history: Vec<String>,
    pub sections: Vec<PreviewSectionInput>,
    pub is_loading: bool,
    pub is_settings_open: bool,
    pub selected_settings_tab: String,
    pub selected_profile: String,
    pub status_kind: String,
    pub status_message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabStateInput {
    pub id: u32,
    pub title: String,
    pub url: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreviewSectionInput {
    pub heading: String,
    pub body: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BrowserShellUiAction {
    Navigate(String),
    Back,
    Forward,
    Reload,
    ToggleOptions,
    CloseOptions,
    SelectProfile(String),
    SelectSettingsTab(String),
    NewTab,
    CloseTab(u32),
    SwitchTab(u32),
    MinimizeWindow,
    MaximizeWindow,
    CloseWindow,
}

pub struct BrowserShellSlintController {
    view_model: BrowserShellViewModel,
    pending_actions: Vec<BrowserShellUiAction>,
}

impl BrowserShellSlintController {
    pub fn new() -> Self {
        Self {
            view_model: BrowserShellViewModel::default(),
            pending_actions: Vec::new(),
        }
    }

    pub fn view_model(&self) -> &BrowserShellViewModel {
        &self.view_model
    }

    pub fn apply_runtime_state(&mut self, input: BrowserShellStateInput) {
        let active_url = input
            .current_url
            .clone()
            .or(input.pending_url.clone())
            .unwrap_or_else(|| "about:blank".to_string());

        self.view_model.window_title = if input.page_title.trim().is_empty() {
            "Rust Browser".to_string()
        } else {
            format!("{} - Rust Browser", input.page_title)
        };
        self.view_model.tabs = input
            .tabs
            .into_iter()
            .map(|tab| BrowserTabViewModel {
                id: tab.id,
                title: tab.title,
                url: tab.url,
                is_active: tab.id == input.active_tab_id,
            })
            .collect();
        self.view_model.toolbar = ToolbarViewModel {
            address_value: input.address_input,
            is_loading: input.is_loading,
            can_interact: !input.is_loading,
            active_tab_title: input.page_title.clone(),
            active_tab_url: active_url.clone(),
        };
        let preview_sections = input
            .sections
            .iter()
            .map(|section| PreviewSectionViewModel {
                heading: section.heading.clone(),
                body: section.body.clone(),
            })
            .collect::<Vec<_>>();

        self.view_model.options_dialog = OptionsDialogViewModel {
            is_open: input.is_settings_open,
            active_section: input.selected_settings_tab,
            selected_profile: input.selected_profile,
            page_title: input.page_title,
            current_url: active_url,
            page_meta: input.page_meta,
            page_summary: input.page_summary,
            history: input.history.clone(),
            preview_sections,
            history_text: if input.history.is_empty() {
                "History is empty.".to_string()
            } else {
                input.history.join("\n")
            },
            preview_text: if input.sections.is_empty() {
                "No preview sections available for this page yet.".to_string()
            } else {
                input
                    .sections
                    .iter()
                    .map(|section| {
                        if section.heading.trim().is_empty() {
                            section.body.clone()
                        } else {
                            format!("{}\n{}", section.heading, section.body)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n")
            },
        };
        self.view_model.status = StatusViewModel {
            kind: input.status_kind,
            message: input.status_message,
        };
    }

    pub fn enqueue_action(&mut self, action: BrowserShellUiAction) {
        self.pending_actions.push(action);
    }

    pub fn drain_actions(&mut self) -> Vec<BrowserShellUiAction> {
        std::mem::take(&mut self.pending_actions)
    }
}
