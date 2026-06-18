use crate::application::browser_service::{normalize_user_url, BrowserService, NavigationDecision};
use crate::application::ports::PageLoader;
use crate::application::runtime::{build_blocklist_policy, save_app_settings, RuntimeConfig};
use crate::domain::blocklist_profile::BlocklistProfile;
use crate::domain::filter::UrlPolicy;
use crate::domain::page::Page;
use crate::infrastructure::config::app_settings::AppSettings;
use crate::infrastructure::http::page_fetcher::HttpPageFetcher;
use crate::presentation::browser_shell_slint::bridge::BrowserShellSlintBridge;
use crate::presentation::browser_shell_slint::controller::{
    BrowserShellSlintController, BrowserShellStateInput, BrowserShellUiAction,
    PreviewSectionInput, TabStateInput,
};
use crate::presentation::webview_shell::templates::{
    blocked_page_html, placeholder_page_html, HEADER_HEIGHT, SETTINGS_PANEL_WIDTH,
};
use slint::winit_030::{EventResult, WinitWindowAccessor};
use slint::{ComponentHandle, Timer, TimerMode};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use wry::{Rect, WebView, WebViewBuilder};

pub fn run(
    browser_service: BrowserService,
    runtime_config: RuntimeConfig,
    settings: AppSettings,
) -> Result<(), String> {
    slint::BackendSelector::new()
        .backend_name("winit".into())
        .select()
        .map_err(|error| error.to_string())?;

    let (preview_command_tx, preview_command_rx) = mpsc::channel();
    let (preview_result_tx, preview_result_rx) = mpsc::channel();
    let (browser_event_tx, browser_event_rx) = mpsc::channel();
    spawn_preview_worker(preview_command_rx, preview_result_tx)?;

    let controller = Rc::new(RefCell::new(BrowserShellSlintController::new()));
    let bridge =
        BrowserShellSlintBridge::new(Rc::clone(&controller)).map_err(|error| error.to_string())?;

    let app = Rc::new(RefCell::new(BrowserShellApp::new(
        browser_service,
        runtime_config,
        settings,
        preview_command_tx,
        preview_result_rx,
        browser_event_tx,
        browser_event_rx,
        controller,
        bridge,
    )));

    BrowserShellApp::install_window_hooks(&app);
    BrowserShellApp::install_action_pump(&app);
    BrowserShellApp::install_preview_pump(&app);
    BrowserShellApp::install_browser_event_pump(&app);
    app.borrow_mut().sync_shell();
    app.borrow()
        .bridge
        .window()
        .show()
        .map_err(|error| error.to_string())?;
    BrowserShellApp::attach_content_after_show(&app)?;

    slint::run_event_loop().map_err(|error| error.to_string())
}

struct BrowserShellApp {
    browser_service: BrowserService,
    runtime_config: RuntimeConfig,
    settings: AppSettings,
    preview_command_tx: Sender<PreviewCommand>,
    preview_result_rx: Receiver<PreviewResult>,
    browser_event_tx: Sender<BrowserEvent>,
    browser_event_rx: Receiver<BrowserEvent>,
    controller: Rc<RefCell<BrowserShellSlintController>>,
    bridge: BrowserShellSlintBridge,
    shell_state: ShellState,
    shared_policy: Rc<RefCell<crate::infrastructure::blocklist::file_blocklist_policy::FileBlocklistPolicy>>,
    content_webview: Option<WebView>,
    action_timer: Timer,
    preview_timer: Timer,
    browser_event_timer: Timer,
}

impl BrowserShellApp {
    fn new(
        browser_service: BrowserService,
        runtime_config: RuntimeConfig,
        settings: AppSettings,
        preview_command_tx: Sender<PreviewCommand>,
        preview_result_rx: Receiver<PreviewResult>,
        browser_event_tx: Sender<BrowserEvent>,
        browser_event_rx: Receiver<BrowserEvent>,
        controller: Rc<RefCell<BrowserShellSlintController>>,
        bridge: BrowserShellSlintBridge,
    ) -> Self {
        let selected_profile = settings.blocklist_profile;
        let shared_policy = Rc::new(RefCell::new(browser_service.url_policy()));
        Self {
            browser_service,
            runtime_config,
            settings,
            preview_command_tx,
            preview_result_rx,
            browser_event_tx,
            browser_event_rx,
            controller,
            bridge,
            shell_state: ShellState::new(selected_profile),
            shared_policy,
            content_webview: None,
            action_timer: Timer::default(),
            preview_timer: Timer::default(),
            browser_event_timer: Timer::default(),
        }
    }

    fn install_window_hooks(app: &Rc<RefCell<Self>>) {
        let weak = Rc::downgrade(app);
        app.borrow()
            .bridge
            .window()
            .window()
            .on_winit_window_event(move |_slint_window, event| {
                if let Some(app) = weak.upgrade() {
                    app.borrow_mut().handle_window_event(event);
                }
                EventResult::Propagate
            });
    }

    fn install_action_pump(app: &Rc<RefCell<Self>>) {
        let weak = Rc::downgrade(app);
        app.borrow().action_timer.start(TimerMode::Repeated, std::time::Duration::from_millis(16), move || {
            let Some(app) = weak.upgrade() else {
                return;
            };

            let actions = {
                let controller = app.borrow().controller.clone();
                let actions = controller.borrow_mut().drain_actions();
                actions
            };

            if actions.is_empty() {
                return;
            }

            let mut app = app.borrow_mut();
            for action in actions {
                app.handle_ui_action(action);
            }
        });
    }

    fn install_preview_pump(app: &Rc<RefCell<Self>>) {
        let weak = Rc::downgrade(app);
        app.borrow().preview_timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(60),
            move || {
                let Some(app) = weak.upgrade() else {
                    return;
                };
                app.borrow_mut().drain_preview_results();
            },
        );
    }

    fn install_browser_event_pump(app: &Rc<RefCell<Self>>) {
        let weak = Rc::downgrade(app);
        app.borrow().browser_event_timer.start(
            TimerMode::Repeated,
            std::time::Duration::from_millis(16),
            move || {
                let Some(app) = weak.upgrade() else {
                    return;
                };
                app.borrow_mut().drain_browser_events();
            },
        );
    }

    fn attach_content_after_show(app: &Rc<RefCell<Self>>) -> Result<(), String> {
        let app_window_weak = app.borrow().bridge.window().as_weak();
        let weak = Rc::downgrade(app);

        slint::spawn_local(async move {
            let Some(app_window) = app_window_weak.upgrade() else {
                return;
            };

            let Ok(winit_window) = app_window.window().winit_window().await else {
                return;
            };

            if let Some(app_rc) = weak.upgrade() {
                let mut app = app_rc.borrow_mut();
                if let Err(error) = app.attach_content_webview(winit_window.as_ref()) {
                    app.shell_state.status =
                        StatusPayload::warning(format!("Failed to attach WebView2 content: {error}"));
                    app.sync_shell();
                }
            }
        })
        .map(|_| ())
        .map_err(|error| error.to_string())
    }

    fn attach_content_webview(&mut self, window: &winit::window::Window) -> Result<(), String> {
        if self.content_webview.is_some() {
            self.update_content_bounds();
            return Ok(());
        }

        let policy = Rc::clone(&self.shared_policy);
        let browser_event_tx = self.browser_event_tx.clone();
        let content_webview = WebViewBuilder::new()
            .with_bounds(self.current_content_bounds(window))
            .with_html(placeholder_page_html())
            .with_navigation_handler({
                let policy = Rc::clone(&policy);
                let browser_event_tx = browser_event_tx.clone();
                move |url| {
                    let allowed = policy.borrow().allows(url.as_str());
                    if !allowed {
                        let _ = browser_event_tx
                            .send(BrowserEvent::NavigationBlocked(url.to_string()));
                    }
                    allowed
                }
            })
            .with_document_title_changed_handler({
                let browser_event_tx = browser_event_tx.clone();
                move |title| {
                    let _ = browser_event_tx.send(BrowserEvent::TitleChanged(title));
                }
            })
            .with_on_page_load_handler({
                let browser_event_tx = browser_event_tx.clone();
                move |event, url| {
                    let _ = match event {
                        wry::PageLoadEvent::Started => {
                            browser_event_tx.send(BrowserEvent::PageLoadStarted(url))
                        }
                        wry::PageLoadEvent::Finished => {
                            browser_event_tx.send(BrowserEvent::PageLoadFinished(url))
                        }
                    };
                }
            })
            .build_as_child(window)
            .map_err(|error| error.to_string())?;

        self.content_webview = Some(content_webview);
        self.update_content_bounds();
        Ok(())
    }

    fn handle_window_event(&mut self, event: &WindowEvent) {
        match event {
            WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. } => {
                self.update_content_bounds();
            }
            WindowEvent::CloseRequested => {
                let _ = slint::quit_event_loop();
            }
            _ => {}
        }
    }

    fn handle_ui_action(&mut self, action: BrowserShellUiAction) {
        match action {
            BrowserShellUiAction::Navigate(url) => self.navigate_requested(url),
            BrowserShellUiAction::Back => {
                self.navigate_history("history.back();", "Navigating back...")
            }
            BrowserShellUiAction::Forward => {
                self.navigate_history("history.forward();", "Navigating forward...")
            }
            BrowserShellUiAction::Reload => self.reload_requested(),
            BrowserShellUiAction::ToggleOptions => {
                self.set_settings_open(!self.shell_state.is_settings_open)
            }
            BrowserShellUiAction::CloseOptions => self.set_settings_open(false),
            BrowserShellUiAction::SelectProfile(profile) => {
                if let Some(profile) = BlocklistProfile::from_command(profile.as_str()) {
                    self.change_profile(profile);
                }
            }
            BrowserShellUiAction::SelectSettingsTab(tab) => {
                if let Some(tab) = SettingsTab::from_command(tab.as_str()) {
                    self.shell_state.selected_settings_tab = tab;
                    self.sync_shell();
                }
            }
            BrowserShellUiAction::NewTab => self.create_tab_and_activate(),
            BrowserShellUiAction::CloseTab(tab_id) => self.close_tab(tab_id),
            BrowserShellUiAction::SwitchTab(tab_id) => self.switch_tab(tab_id),
            BrowserShellUiAction::MinimizeWindow => {
                let _ = self.bridge.window().window().with_winit_window(|window| {
                    window.set_minimized(true);
                });
            }
            BrowserShellUiAction::MaximizeWindow => {
                let _ = self.bridge.window().window().with_winit_window(|window| {
                    window.set_maximized(!window.is_maximized());
                });
            }
            BrowserShellUiAction::CloseWindow => {
                let _ = slint::quit_event_loop();
            }
        }
    }

    fn reload_requested(&mut self) {
        if self.content_webview.is_none() {
            return;
        }

        let pending = self
            .shell_state
            .active_tab()
            .current_url
            .clone()
            .or_else(|| self.shell_state.active_tab().pending_url.clone());
        self.shell_state.active_tab_mut().pending_url = pending;
        self.shell_state.is_loading = true;
        self.shell_state.status = StatusPayload::info("Reloading page...".to_string());
        self.sync_shell();

        if let Some(content_webview) = self.content_webview.as_ref() {
            let _ = content_webview.reload();
        }
    }

    fn set_settings_open(&mut self, open: bool) {
        self.shell_state.is_settings_open = open;
        self.update_content_bounds();
        self.sync_shell();
    }

    fn change_profile(&mut self, profile: BlocklistProfile) {
        if self.settings.blocklist_profile == profile {
            return;
        }

        match build_blocklist_policy(&self.runtime_config, profile) {
            Ok(policy) => {
                self.browser_service.replace_policy(policy);
                *self.shared_policy.borrow_mut() = self.browser_service.url_policy();
                self.settings.blocklist_profile = profile;
                self.shell_state.selected_profile = profile.as_str().to_string();
                self.shell_state.status = StatusPayload::success(format!(
                    "Blocklist profile switched to {}.",
                    profile.as_str()
                ));

                if let Err(error) = save_app_settings(&self.runtime_config, &self.settings) {
                    self.shell_state.status = StatusPayload::warning(format!(
                        "Profile changed to {}, but settings could not be saved: {}",
                        profile.as_str(),
                        error
                    ));
                }

                self.sync_shell();

                if let Some(url) = self
                    .shell_state
                    .active_tab()
                    .current_url
                    .clone()
                    .or_else(|| self.shell_state.active_tab().pending_url.clone())
                {
                    self.navigate_requested(url);
                }
            }
            Err(error) => {
                self.shell_state.status =
                    StatusPayload::warning(format!("Failed to switch profile: {error}"));
                self.sync_shell();
            }
        }
    }

    fn navigate_history(&mut self, script: &str, message: &str) {
        if self.content_webview.is_none() {
            return;
        }

        let pending = self
            .shell_state
            .active_tab()
            .current_url
            .clone()
            .or_else(|| self.shell_state.active_tab().pending_url.clone());
        self.shell_state.active_tab_mut().pending_url = pending;
        self.shell_state.is_loading = true;
        self.shell_state.status = StatusPayload::info(message.to_string());
        self.sync_shell();

        if let Some(content_webview) = self.content_webview.as_ref() {
            let _ = content_webview.evaluate_script(script);
        }
    }

    fn navigate_requested(&mut self, requested_url: String) {
        match self.browser_service.prepare_navigation(requested_url.as_str()) {
            NavigationDecision::Allowed { url } => {
                {
                    let active_tab = self.shell_state.active_tab_mut();
                    active_tab.address_input = url.clone();
                    active_tab.pending_url = Some(url.clone());
                    if active_tab.title == DEFAULT_TAB_TITLE {
                        active_tab.title = "Loading page".to_string();
                    }
                }
                self.shell_state.is_loading = true;
                self.shell_state.status = StatusPayload::info(format!("Loading {url}..."));
                self.push_preview_clear();
                self.sync_shell();

                if let Some(content_webview) = self.content_webview.as_ref() {
                    let _ = content_webview.load_url(url.as_str());
                }

                self.spawn_preview_fetch(url);
            }
            NavigationDecision::Blocked { url } => {
                {
                    let active_tab = self.shell_state.active_tab_mut();
                    active_tab.address_input = url.clone();
                    active_tab.pending_url = None;
                }
                self.shell_state.is_loading = false;
                self.shell_state.status =
                    StatusPayload::warning(format!("Blocked by security filter: {url}"));
                self.sync_shell();
                self.show_placeholder(blocked_page_html(url.as_str()));
            }
        }
    }

    fn handle_navigation_started(&mut self, url: String) {
        {
            let active_tab = self.shell_state.active_tab_mut();
            active_tab.address_input = url.clone();
            active_tab.pending_url = Some(url.clone());
        }
        self.shell_state.is_loading = true;
        self.shell_state.status = StatusPayload::info(format!("Loading {url}..."));
        self.sync_shell();
    }

    fn handle_navigation_finished(&mut self, url: String) {
        self.browser_service.commit_navigation(url.as_str());
        {
            let active_tab = self.shell_state.active_tab_mut();
            active_tab.address_input = url.clone();
            active_tab.current_url = Some(url.clone());
            active_tab.pending_url = None;
            if active_tab.history.last() != Some(&url) {
                active_tab.history.push(url.clone());
            }
            if active_tab.title == "Loading page" || active_tab.title == DEFAULT_TAB_TITLE {
                active_tab.title = truncate_tab_title(url.as_str());
            }
        }
        self.shell_state.is_loading = false;
        self.shell_state.status = StatusPayload::success("Page loaded in WebView2.".to_string());
        self.sync_shell();
    }

    fn handle_navigation_blocked(&mut self, url: String) {
        self.shell_state.active_tab_mut().pending_url = None;
        self.shell_state.is_loading = false;
        self.shell_state.status =
            StatusPayload::warning(format!("Blocked by security filter: {url}"));
        self.sync_shell();
        self.show_placeholder(blocked_page_html(url.as_str()));
    }

    fn handle_title_changed(&mut self, title: String) {
        self.shell_state.active_tab_mut().title = if title.trim().is_empty() {
            "Untitled page".to_string()
        } else {
            title
        };
        self.sync_shell();
    }

    fn drain_preview_results(&mut self) {
        loop {
            let next = match self.preview_result_rx.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            };

            match next {
                PreviewResult::Loaded { tab_id, url, page } => {
                    self.handle_preview_loaded(tab_id, url, page)
                }
                PreviewResult::Failed { tab_id, url, error } => {
                    self.handle_preview_failed(tab_id, url, error)
                }
            }
        }
    }

    fn drain_browser_events(&mut self) {
        loop {
            let next = match self.browser_event_rx.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => break,
            };

            match next {
                BrowserEvent::PageLoadStarted(url) => self.handle_navigation_started(url),
                BrowserEvent::PageLoadFinished(url) => self.handle_navigation_finished(url),
                BrowserEvent::NavigationBlocked(url) => self.handle_navigation_blocked(url),
                BrowserEvent::TitleChanged(title) => self.handle_title_changed(title),
            }
        }
    }

    fn handle_preview_loaded(&mut self, tab_id: u32, url: String, page: Page) {
        if let Some(tab) = self.shell_state.tabs.iter_mut().find(|tab| tab.id == tab_id) {
            let current_url = tab.pending_url.as_ref().or(tab.current_url.as_ref());
            if current_url != Some(&url) {
                return;
            }

            tab.title = page.title.clone();
            tab.page_meta = format!("Preview source: {} | HTTP {}", page.url, page.status);
            tab.page_summary = page.summary.clone();
            tab.sections = page
                .sections
                .iter()
                .take(6)
                .map(|section| SectionPayload {
                    heading: section.heading.clone(),
                    body: section.body.clone(),
                })
                .collect();

            if self.shell_state.active_tab_id == tab_id {
                self.sync_shell();
            }
        }
    }

    fn handle_preview_failed(&mut self, tab_id: u32, url: String, error: String) {
        let Some(tab) = self.shell_state.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        let current_url = tab.pending_url.as_ref().or(tab.current_url.as_ref());
        if current_url != Some(&url) {
            return;
        }

        tab.page_summary = format!("Readable preview could not be extracted: {error}");
        tab.sections = Vec::new();
        if self.shell_state.active_tab_id == tab_id && self.shell_state.is_loading {
            self.shell_state.status =
                StatusPayload::warning("The page is loading, but the readable preview failed.".to_string());
        }
        if self.shell_state.active_tab_id == tab_id {
            self.sync_shell();
        }
    }

    fn create_tab_and_activate(&mut self) {
        let tab_id = self.shell_state.create_tab();
        self.shell_state.active_tab_id = tab_id;
        self.shell_state.is_loading = false;
        self.shell_state.status = StatusPayload::info("Opened a new tab.".to_string());
        self.show_placeholder(placeholder_page_html());
        self.sync_shell();
    }

    fn close_tab(&mut self, tab_id: u32) {
        if self.shell_state.tabs.len() == 1 {
            return;
        }

        let was_active = self.shell_state.active_tab_id == tab_id;
        let removed_index = self
            .shell_state
            .tabs
            .iter()
            .position(|tab| tab.id == tab_id);

        let Some(index) = removed_index else {
            return;
        };

        self.shell_state.tabs.remove(index);

        if was_active {
            let fallback_index = index.saturating_sub(1).min(self.shell_state.tabs.len() - 1);
            let fallback_id = self.shell_state.tabs[fallback_index].id;
            self.shell_state.active_tab_id = fallback_id;
            self.restore_active_tab_view();
        }

        self.shell_state.status = StatusPayload::info("Tab closed.".to_string());
        self.sync_shell();
    }

    fn switch_tab(&mut self, tab_id: u32) {
        if self.shell_state.active_tab_id == tab_id {
            return;
        }
        if !self.shell_state.tabs.iter().any(|tab| tab.id == tab_id) {
            return;
        }

        self.shell_state.active_tab_id = tab_id;
        self.restore_active_tab_view();
        self.shell_state.status = StatusPayload::info("Switched tab.".to_string());
        self.sync_shell();
    }

    fn restore_active_tab_view(&mut self) {
        self.shell_state.is_loading = false;

        let url = self
            .shell_state
            .active_tab()
            .current_url
            .clone()
            .or_else(|| self.shell_state.active_tab().pending_url.clone());

        match url {
            Some(url) => {
                if let Some(content_webview) = self.content_webview.as_ref() {
                    let _ = content_webview.load_url(url.as_str());
                }
            }
            None => self.show_placeholder(placeholder_page_html()),
        }
    }

    fn spawn_preview_fetch(&self, url: String) {
        let _ = self.preview_command_tx.send(PreviewCommand {
            tab_id: self.shell_state.active_tab_id,
            url,
        });
    }

    fn push_preview_clear(&mut self) {
        let active_tab = self.shell_state.active_tab_mut();
        active_tab.title = "Loading page".to_string();
        active_tab.page_meta.clear();
        active_tab.page_summary =
            "Fetching a structured preview while the embedded browser navigates.".to_string();
        active_tab.sections = Vec::new();
    }

    fn show_placeholder(&self, html: String) {
        if let Some(content_webview) = self.content_webview.as_ref() {
            let _ = content_webview.load_html(html.as_str());
        }
    }

    fn update_content_bounds(&self) {
        let Some(content_webview) = self.content_webview.as_ref() else {
            return;
        };

        let _ = self.bridge.window().window().with_winit_window(|window| {
            let _ = content_webview.set_bounds(self.current_content_bounds(window));
        });
    }

    fn current_content_bounds(&self, window: &winit::window::Window) -> Rect {
        let size = window.inner_size();
        let viewport_width = f64::from(size.width);
        let reserved_width = if self.shell_state.is_settings_open && viewport_width <= 980.0 {
            viewport_width
        } else if self.shell_state.is_settings_open {
            SETTINGS_PANEL_WIDTH + 30.0
        } else {
            0.0
        };
        let width = if self.shell_state.is_settings_open && viewport_width <= 980.0 {
            1.0
        } else {
            (viewport_width - reserved_width).max(320.0)
        };
        let height = (f64::from(size.height) - HEADER_HEIGHT).max(200.0);

        Rect {
            position: LogicalPosition::new(0.0, HEADER_HEIGHT).into(),
            size: LogicalSize::new(width, height).into(),
        }
    }

    fn sync_shell(&mut self) {
        let snapshot = self.shell_state.snapshot();
        self.controller
            .borrow_mut()
            .apply_runtime_state(BrowserShellStateInput {
                tabs: snapshot
                    .tabs
                    .iter()
                    .map(|tab| TabStateInput {
                        id: tab.id,
                        title: tab.title.clone(),
                        url: tab.url.clone(),
                    })
                    .collect(),
                active_tab_id: snapshot.active_tab_id,
                address_input: snapshot.address_input.clone(),
                current_url: snapshot.current_url.clone(),
                pending_url: snapshot.pending_url.clone(),
                page_title: snapshot.page_title.clone(),
                page_meta: snapshot.page_meta.clone(),
                page_summary: snapshot.page_summary.clone(),
                history: snapshot.history.clone(),
                sections: snapshot
                    .sections
                    .iter()
                    .map(|section| PreviewSectionInput {
                        heading: section.heading.clone(),
                        body: section.body.clone(),
                    })
                    .collect(),
                is_loading: snapshot.is_loading,
                is_settings_open: snapshot.is_settings_open,
                selected_settings_tab: snapshot.selected_settings_tab.clone(),
                selected_profile: snapshot.selected_profile.clone(),
                status_kind: snapshot.status.kind.to_string(),
                status_message: snapshot.status.message.clone(),
            });
        self.bridge.sync_from_controller();
    }
}

fn spawn_preview_worker(
    command_rx: Receiver<PreviewCommand>,
    result_tx: Sender<PreviewResult>,
) -> Result<(), String> {
    let loader = HttpPageFetcher::new()?;

    std::thread::spawn(move || {
        while let Ok(command) = command_rx.recv() {
            let normalized = normalize_user_url(command.url.as_str());
            let result = match loader.load(normalized.as_str()) {
                Ok(page) => PreviewResult::Loaded {
                    tab_id: command.tab_id,
                    url: normalized,
                    page,
                },
                Err(error) => PreviewResult::Failed {
                    tab_id: command.tab_id,
                    url: normalized,
                    error,
                },
            };

            if result_tx.send(result).is_err() {
                break;
            }
        }
    });

    Ok(())
}

const DEFAULT_TAB_TITLE: &str = "New Tab";

#[derive(Clone)]
struct PreviewCommand {
    tab_id: u32,
    url: String,
}

enum PreviewResult {
    Loaded { tab_id: u32, url: String, page: Page },
    Failed { tab_id: u32, url: String, error: String },
}

enum BrowserEvent {
    PageLoadStarted(String),
    PageLoadFinished(String),
    NavigationBlocked(String),
    TitleChanged(String),
}

#[derive(Clone, Copy)]
enum SettingsTab {
    Security,
    History,
    Preview,
}

impl SettingsTab {
    fn from_command(value: &str) -> Option<Self> {
        match value {
            "security" => Some(Self::Security),
            "history" => Some(Self::History),
            "preview" => Some(Self::Preview),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Security => "security",
            Self::History => "history",
            Self::Preview => "preview",
        }
    }
}

struct ShellState {
    tabs: Vec<BrowserTab>,
    active_tab_id: u32,
    next_tab_id: u32,
    is_loading: bool,
    is_settings_open: bool,
    selected_settings_tab: SettingsTab,
    selected_profile: String,
    status: StatusPayload,
}

impl ShellState {
    fn new(profile: BlocklistProfile) -> Self {
        Self {
            tabs: vec![BrowserTab::new(1)],
            active_tab_id: 1,
            next_tab_id: 2,
            is_loading: false,
            is_settings_open: false,
            selected_settings_tab: SettingsTab::Security,
            selected_profile: profile.as_str().to_string(),
            status: StatusPayload::default(),
        }
    }

    fn active_tab(&self) -> &BrowserTab {
        self.tabs
            .iter()
            .find(|tab| tab.id == self.active_tab_id)
            .expect("active tab should exist")
    }

    fn active_tab_mut(&mut self) -> &mut BrowserTab {
        self.tabs
            .iter_mut()
            .find(|tab| tab.id == self.active_tab_id)
            .expect("active tab should exist")
    }

    fn create_tab(&mut self) -> u32 {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;
        self.tabs.push(BrowserTab::new(tab_id));
        tab_id
    }

    fn snapshot(&self) -> ShellStateSnapshot {
        let active_tab = self.active_tab();
        ShellStateSnapshot {
            tabs: self
                .tabs
                .iter()
                .map(|tab| TabPayload {
                    id: tab.id,
                    title: tab.title.clone(),
                    url: tab
                        .current_url
                        .clone()
                        .or_else(|| tab.pending_url.clone())
                        .unwrap_or_else(|| "about:blank".to_string()),
                })
                .collect(),
            active_tab_id: self.active_tab_id,
            address_input: active_tab.address_input.clone(),
            current_url: active_tab.current_url.clone(),
            pending_url: active_tab.pending_url.clone(),
            page_title: active_tab.title.clone(),
            page_meta: active_tab.page_meta.clone(),
            page_summary: active_tab.page_summary.clone(),
            history: active_tab.history.iter().rev().cloned().collect(),
            sections: active_tab.sections.clone(),
            is_loading: self.is_loading,
            is_settings_open: self.is_settings_open,
            selected_settings_tab: self.selected_settings_tab.as_str().to_string(),
            selected_profile: self.selected_profile.clone(),
            status: self.status.clone(),
        }
    }
}

struct BrowserTab {
    id: u32,
    title: String,
    address_input: String,
    current_url: Option<String>,
    pending_url: Option<String>,
    page_meta: String,
    page_summary: String,
    history: Vec<String>,
    sections: Vec<SectionPayload>,
}

impl BrowserTab {
    fn new(id: u32) -> Self {
        Self {
            id,
            title: DEFAULT_TAB_TITLE.to_string(),
            address_input: String::new(),
            current_url: None,
            pending_url: None,
            page_meta: String::new(),
            page_summary: "Open a page to fetch a structured summary beside the embedded browser."
                .to_string(),
            history: Vec::new(),
            sections: Vec::new(),
        }
    }
}

struct ShellStateSnapshot {
    tabs: Vec<TabPayload>,
    active_tab_id: u32,
    address_input: String,
    current_url: Option<String>,
    pending_url: Option<String>,
    page_title: String,
    page_meta: String,
    page_summary: String,
    history: Vec<String>,
    sections: Vec<SectionPayload>,
    is_loading: bool,
    is_settings_open: bool,
    selected_settings_tab: String,
    selected_profile: String,
    status: StatusPayload,
}

struct TabPayload {
    id: u32,
    title: String,
    url: String,
}

#[derive(Clone, Default)]
struct SectionPayload {
    heading: String,
    body: String,
}

#[derive(Clone)]
struct StatusPayload {
    kind: &'static str,
    message: String,
}

impl Default for StatusPayload {
    fn default() -> Self {
        Self::info("Ready to browse with the embedded WebView2 shell.".to_string())
    }
}

impl StatusPayload {
    fn success(message: String) -> Self {
        Self {
            kind: "success",
            message,
        }
    }

    fn warning(message: String) -> Self {
        Self {
            kind: "warning",
            message,
        }
    }

    fn info(message: String) -> Self {
        Self {
            kind: "info",
            message,
        }
    }
}

fn truncate_tab_title(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return DEFAULT_TAB_TITLE.to_string();
    }

    const MAX_LEN: usize = 38;
    let mut shortened = trimmed.chars().take(MAX_LEN).collect::<String>();
    if trimmed.chars().count() > MAX_LEN {
        shortened.push_str("...");
    }
    shortened
}
