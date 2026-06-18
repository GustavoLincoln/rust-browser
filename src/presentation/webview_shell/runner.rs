use crate::application::browser_service::{normalize_user_url, BrowserService, NavigationDecision};
use crate::application::ports::PageLoader;
use crate::domain::filter::UrlPolicy;
use crate::domain::page::Page;
use crate::infrastructure::http::page_fetcher::HttpPageFetcher;
use crate::presentation::webview_shell::runner::ShellCommand::{Back, Forward, Navigate, Reload};
use serde::Serialize;
use std::sync::mpsc::{self, Receiver, Sender};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalPosition, LogicalSize};
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};
use wry::{Rect, WebView, WebViewBuilder};

const WINDOW_WIDTH: f64 = 1440.0;
const WINDOW_HEIGHT: f64 = 920.0;
const HEADER_HEIGHT: f64 = 108.0;
const SIDEBAR_WIDTH: f64 = 320.0;

pub fn run(browser_service: BrowserService) -> Result<(), String> {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|error| error.to_string())?;
    let proxy = event_loop.create_proxy();

    let (preview_command_tx, preview_command_rx) = mpsc::channel();
    spawn_preview_worker(preview_command_rx, proxy.clone())?;

    let mut app = BrowserShellApp::new(browser_service, preview_command_tx, proxy);
    event_loop.run_app(&mut app).map_err(|error| error.to_string())
}

struct BrowserShellApp {
    browser_service: BrowserService,
    preview_command_tx: Sender<PreviewCommand>,
    proxy: EventLoopProxy<UserEvent>,
    shell_state: ShellState,
    window: Option<Window>,
    shell_webview: Option<WebView>,
    content_webview: Option<WebView>,
}

impl BrowserShellApp {
    fn new(
        browser_service: BrowserService,
        preview_command_tx: Sender<PreviewCommand>,
        proxy: EventLoopProxy<UserEvent>,
    ) -> Self {
        Self {
            browser_service,
            preview_command_tx,
            proxy,
            shell_state: ShellState::default(),
            window: None,
            shell_webview: None,
            content_webview: None,
        }
    }

    fn handle_shell_command(&mut self, command: ShellCommand) {
        match command {
            Back => self.navigate_history("history.back();", "Navigating back..."),
            Forward => self.navigate_history("history.forward();", "Navigating forward..."),
            Navigate(url) => self.navigate_requested(url),
            Reload => {
                if let Some(content_webview) = self.content_webview.as_ref() {
                    self.shell_state.is_loading = true;
                    self.shell_state.pending_url = self.shell_state.current_url.clone();
                    self.shell_state.status = StatusPayload::info("Reloading page...".to_string());
                    self.sync_shell();
                    let _ = content_webview.reload();
                }
            }
        }
    }

    fn navigate_history(&mut self, script: &str, message: &str) {
        if let Some(content_webview) = self.content_webview.as_ref() {
            self.shell_state.is_loading = true;
            self.shell_state.pending_url = self.shell_state.current_url.clone();
            self.shell_state.status = StatusPayload::info(message.to_string());
            self.sync_shell();
            let _ = content_webview.evaluate_script(script);
        }
    }

    fn navigate_requested(&mut self, requested_url: String) {
        match self.browser_service.prepare_navigation(requested_url.as_str()) {
            NavigationDecision::Allowed { url } => {
                self.shell_state.address_input = url.clone();
                self.shell_state.pending_url = Some(url.clone());
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
                self.shell_state.address_input = url.clone();
                self.shell_state.pending_url = None;
                self.shell_state.is_loading = false;
                self.shell_state.status =
                    StatusPayload::warning(format!("Blocked by security filter: {url}"));
                self.sync_shell();
                self.show_placeholder(blocked_page_html(url.as_str()));
            }
        }
    }

    fn handle_navigation_started(&mut self, url: String) {
        self.shell_state.address_input = url.clone();
        self.shell_state.pending_url = Some(url.clone());
        self.shell_state.is_loading = true;
        self.shell_state.status = StatusPayload::info(format!("Loading {url}..."));
        self.sync_shell();
    }

    fn handle_navigation_finished(&mut self, url: String) {
        self.browser_service.commit_navigation(url.as_str());
        self.shell_state.address_input = url.clone();
        self.shell_state.current_url = Some(url);
        self.shell_state.pending_url = None;
        self.shell_state.is_loading = false;
        self.shell_state.history = self.browser_service.history().iter().rev().cloned().collect();
        self.shell_state.status = StatusPayload::success("Page loaded in WebView2.".to_string());
        self.sync_shell();
    }

    fn handle_navigation_blocked(&mut self, url: String) {
        self.shell_state.pending_url = None;
        self.shell_state.is_loading = false;
        self.shell_state.status =
            StatusPayload::warning(format!("Blocked by security filter: {url}"));
        self.sync_shell();
        self.show_placeholder(blocked_page_html(url.as_str()));
    }

    fn handle_title_changed(&mut self, title: String) {
        self.shell_state.page_title = if title.trim().is_empty() {
            "Untitled page".to_string()
        } else {
            title
        };
        self.sync_shell();
    }

    fn handle_preview_loaded(&mut self, url: String, page: Page) {
        let current_url = self
            .shell_state
            .pending_url
            .as_ref()
            .or(self.shell_state.current_url.as_ref());

        if current_url != Some(&url) {
            return;
        }

        self.shell_state.page_title = page.title.clone();
        self.shell_state.page_meta = format!("Preview source: {} | HTTP {}", page.url, page.status);
        self.shell_state.page_summary = page.summary.clone();
        self.shell_state.sections = page
            .sections
            .iter()
            .take(6)
            .map(|section| SectionPayload {
                heading: section.heading.clone(),
                body: section.body.clone(),
            })
            .collect();
        self.sync_shell();
    }

    fn handle_preview_failed(&mut self, url: String, error: String) {
        let current_url = self
            .shell_state
            .pending_url
            .as_ref()
            .or(self.shell_state.current_url.as_ref());

        if current_url != Some(&url) {
            return;
        }

        self.shell_state.page_summary =
            format!("Readable preview could not be extracted: {error}");
        self.shell_state.sections = Vec::new();
        if self.shell_state.is_loading {
            self.shell_state.status =
                StatusPayload::warning("The page is loading, but the readable preview failed.".to_string());
        }
        self.sync_shell();
    }

    fn spawn_preview_fetch(&self, url: String) {
        let _ = self.preview_command_tx.send(PreviewCommand { url });
    }

    fn push_preview_clear(&mut self) {
        self.shell_state.page_title = "Loading page".to_string();
        self.shell_state.page_meta.clear();
        self.shell_state.page_summary =
            "Fetching a structured preview while the embedded browser navigates.".to_string();
        self.shell_state.sections = Vec::new();
    }

    fn show_placeholder(&self, html: String) {
        if let Some(content_webview) = self.content_webview.as_ref() {
            let _ = content_webview.load_html(html.as_str());
        }
    }

    fn update_content_bounds(&self) {
        let (Some(window), Some(content_webview)) =
            (self.window.as_ref(), self.content_webview.as_ref())
        else {
            return;
        };

        let size = window.inner_size();
        let width = (f64::from(size.width) - SIDEBAR_WIDTH).max(320.0);
        let height = (f64::from(size.height) - HEADER_HEIGHT).max(200.0);

        let _ = content_webview.set_bounds(Rect {
            position: LogicalPosition::new(SIDEBAR_WIDTH, HEADER_HEIGHT).into(),
            size: LogicalSize::new(width, height).into(),
        });
    }

    fn sync_shell(&self) {
        let Some(shell_webview) = self.shell_webview.as_ref() else {
            return;
        };

        let serialized = match serde_json::to_string(&self.shell_state) {
            Ok(value) => value,
            Err(_) => return,
        };

        let script = format!("window.rustBrowserUpdate({serialized});");
        let _ = shell_webview.evaluate_script(script.as_str());
    }
}

impl ApplicationHandler<UserEvent> for BrowserShellApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Rust Browser")
            .with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
            .with_min_inner_size(LogicalSize::new(1100.0, 720.0));

        let window = match event_loop.create_window(window_attributes) {
            Ok(window) => window,
            Err(error) => {
                let _ = self
                    .proxy
                    .send_event(UserEvent::Fatal(format!("Failed to create window: {error}")));
                return;
            }
        };

        let shell_proxy = self.proxy.clone();
        let shell_webview = match WebViewBuilder::new()
            .with_html(shell_html())
            .with_ipc_handler(move |request| {
                let body = request.body().to_string();
                let command = if let Some(url) = body.strip_prefix("navigate:") {
                    Some(UserEvent::ShellCommand(Navigate(url.to_string())))
                } else if body == "back" {
                    Some(UserEvent::ShellCommand(Back))
                } else if body == "forward" {
                    Some(UserEvent::ShellCommand(Forward))
                } else if body == "reload" {
                    Some(UserEvent::ShellCommand(Reload))
                } else {
                    None
                };

                if let Some(command) = command {
                    let _ = shell_proxy.send_event(command);
                }
            })
            .build(&window)
        {
            Ok(webview) => webview,
            Err(error) => {
                let _ = self.proxy.send_event(UserEvent::Fatal(format!(
                    "Failed to create shell webview: {error}"
                )));
                return;
            }
        };

        let navigation_proxy = self.proxy.clone();
        let page_load_proxy = self.proxy.clone();
        let title_proxy = self.proxy.clone();
        let policy = self.browser_service.url_policy();

        let initial_bounds = Rect {
            position: LogicalPosition::new(SIDEBAR_WIDTH, HEADER_HEIGHT).into(),
            size: LogicalSize::new(WINDOW_WIDTH - SIDEBAR_WIDTH, WINDOW_HEIGHT - HEADER_HEIGHT).into(),
        };

        let content_webview = match WebViewBuilder::new()
            .with_bounds(initial_bounds)
            .with_html(placeholder_page_html())
            .with_navigation_handler(move |url| {
                let allowed = policy.allows(url.as_str());
                if !allowed {
                    let _ = navigation_proxy.send_event(UserEvent::NavigationBlocked(url));
                }
                allowed
            })
            .with_document_title_changed_handler(move |title| {
                let _ = title_proxy.send_event(UserEvent::TitleChanged(title));
            })
            .with_on_page_load_handler(move |event, url| {
                let user_event = match event {
                    wry::PageLoadEvent::Started => UserEvent::PageLoadStarted(url),
                    wry::PageLoadEvent::Finished => UserEvent::PageLoadFinished(url),
                };
                let _ = page_load_proxy.send_event(user_event);
            })
            .build_as_child(&window)
        {
            Ok(webview) => webview,
            Err(error) => {
                let _ = self.proxy.send_event(UserEvent::Fatal(format!(
                    "Failed to create content webview: {error}"
                )));
                return;
            }
        };

        self.window = Some(window);
        self.shell_webview = Some(shell_webview);
        self.content_webview = Some(content_webview);
        self.update_content_bounds();
        self.sync_shell();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(_) => self.update_content_bounds(),
            WindowEvent::CloseRequested => event_loop.exit(),
            _ => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::ShellCommand(command) => self.handle_shell_command(command),
            UserEvent::PageLoadStarted(url) => self.handle_navigation_started(url),
            UserEvent::PageLoadFinished(url) => self.handle_navigation_finished(url),
            UserEvent::NavigationBlocked(url) => self.handle_navigation_blocked(url),
            UserEvent::TitleChanged(title) => self.handle_title_changed(title),
            UserEvent::PreviewLoaded { url, page } => self.handle_preview_loaded(url, page),
            UserEvent::PreviewFailed { url, error } => self.handle_preview_failed(url, error),
            UserEvent::Fatal(message) => {
                eprintln!("{message}");
                event_loop.exit();
            }
        }
    }
}

fn spawn_preview_worker(
    command_rx: Receiver<PreviewCommand>,
    proxy: EventLoopProxy<UserEvent>,
) -> Result<(), String> {
    let loader = HttpPageFetcher::new()?;

    std::thread::spawn(move || {
        while let Ok(command) = command_rx.recv() {
            let normalized = normalize_user_url(command.url.as_str());
            let event = match loader.load(normalized.as_str()) {
                Ok(page) => UserEvent::PreviewLoaded {
                    url: normalized,
                    page,
                },
                Err(error) => UserEvent::PreviewFailed {
                    url: normalized,
                    error,
                },
            };

            if proxy.send_event(event).is_err() {
                break;
            }
        }
    });

    Ok(())
}

#[derive(Clone)]
struct PreviewCommand {
    url: String,
}

enum UserEvent {
    ShellCommand(ShellCommand),
    PageLoadStarted(String),
    PageLoadFinished(String),
    NavigationBlocked(String),
    TitleChanged(String),
    PreviewLoaded { url: String, page: Page },
    PreviewFailed { url: String, error: String },
    Fatal(String),
}

enum ShellCommand {
    Back,
    Forward,
    Navigate(String),
    Reload,
}

#[derive(Default, Serialize)]
struct ShellState {
    address_input: String,
    current_url: Option<String>,
    pending_url: Option<String>,
    page_title: String,
    page_meta: String,
    page_summary: String,
    history: Vec<String>,
    sections: Vec<SectionPayload>,
    is_loading: bool,
    status: StatusPayload,
}

#[derive(Default, Serialize)]
struct SectionPayload {
    heading: String,
    body: String,
}

#[derive(Serialize)]
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

fn shell_html() -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>Rust Browser Shell</title>
    <style>
      :root {{
        --header-height: {header_height}px;
        --sidebar-width: {sidebar_width}px;
        --sand: #f4eee4;
        --paper: #fcf8f1;
        --panel: #e6d8c3;
        --ink: #1f2933;
        --muted: #5f6b76;
        --success: #2f7e60;
        --warning: #aa6b22;
        --info: #586785;
        --border: rgba(31, 41, 51, 0.08);
      }}

      * {{
        box-sizing: border-box;
      }}

      body {{
        margin: 0;
        font-family: "Segoe UI", "Aptos", sans-serif;
        color: var(--ink);
        background:
          radial-gradient(circle at top left, rgba(242, 215, 160, 0.45), transparent 36%),
          linear-gradient(180deg, #f9f2e8 0%, #f5efe6 58%, #efe4d4 100%);
        overflow: hidden;
      }}

      header {{
        position: fixed;
        inset: 0 0 auto 0;
        height: var(--header-height);
        padding: 20px 24px 16px;
        border-bottom: 1px solid var(--border);
        backdrop-filter: blur(12px);
        background: rgba(252, 248, 241, 0.9);
        z-index: 10;
      }}

      .toolbar {{
        display: grid;
        grid-template-columns: auto auto 1fr auto auto;
        gap: 12px;
        align-items: center;
      }}

      input {{
        width: 100%;
        height: 44px;
        padding: 0 16px;
        border: 1px solid rgba(95, 107, 118, 0.16);
        border-radius: 14px;
        font-size: 15px;
        background: rgba(255, 255, 255, 0.85);
      }}

      button {{
        height: 44px;
        border: 0;
        padding: 0 18px;
        border-radius: 14px;
        font-weight: 600;
        cursor: pointer;
        color: white;
        background: linear-gradient(135deg, #3d5a88, #26416c);
      }}

      button.alt {{
        color: var(--ink);
        background: rgba(31, 41, 51, 0.08);
      }}

      button:disabled,
      input:disabled {{
        opacity: 0.55;
        cursor: not-allowed;
      }}

      .status {{
        margin-top: 12px;
        border-radius: 14px;
        padding: 10px 14px;
        font-size: 14px;
        font-weight: 600;
        background: rgba(88, 103, 133, 0.12);
        color: var(--info);
      }}

      .status.success {{
        background: rgba(47, 126, 96, 0.12);
        color: var(--success);
      }}

      .status.warning {{
        background: rgba(170, 107, 34, 0.14);
        color: var(--warning);
      }}

      .status.loading {{
        background: rgba(88, 103, 133, 0.16);
        color: var(--info);
      }}

      aside {{
        position: fixed;
        inset: var(--header-height) auto 0 0;
        width: var(--sidebar-width);
        padding: 22px 18px 24px;
        overflow: auto;
        border-right: 1px solid var(--border);
        background: rgba(230, 216, 195, 0.84);
      }}

      .card {{
        background: rgba(252, 248, 241, 0.78);
        border: 1px solid rgba(31, 41, 51, 0.08);
        border-radius: 18px;
        padding: 16px;
        box-shadow: 0 18px 40px rgba(61, 48, 32, 0.08);
      }}

      .card + .card {{
        margin-top: 14px;
      }}

      h1, h2, h3, p {{
        margin: 0;
      }}

      h1 {{
        font-size: 18px;
        margin-bottom: 6px;
      }}

      h2 {{
        font-size: 14px;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: var(--muted);
        margin-bottom: 12px;
      }}

      .muted {{
        color: var(--muted);
        font-size: 13px;
        line-height: 1.5;
      }}

      .history-item {{
        display: block;
        width: 100%;
        margin-top: 8px;
        padding: 12px 14px;
        border: 0;
        text-align: left;
        color: var(--ink);
        background: rgba(255, 255, 255, 0.66);
      }}

      .history-item:hover {{
        background: rgba(255, 255, 255, 0.94);
      }}

      main {{
        position: fixed;
        inset: var(--header-height) 0 0 var(--sidebar-width);
        pointer-events: none;
        padding: 18px;
      }}

      .frame-label {{
        position: absolute;
        top: 16px;
        left: 18px;
        border-radius: 999px;
        padding: 8px 12px;
        font-size: 12px;
        letter-spacing: 0.08em;
        text-transform: uppercase;
        color: rgba(31, 41, 51, 0.75);
        background: rgba(255, 255, 255, 0.56);
        backdrop-filter: blur(10px);
      }}

      .section {{
        margin-top: 14px;
      }}

      .section h3 {{
        margin-bottom: 8px;
        font-size: 15px;
      }}

      .section p {{
        font-size: 13px;
        line-height: 1.55;
        color: var(--muted);
        white-space: pre-wrap;
      }}
    </style>
  </head>
  <body>
    <header>
      <div class="toolbar">
        <button id="back-button" class="alt">Back</button>
        <button id="forward-button" class="alt">Forward</button>
        <input id="address" autocomplete="off" spellcheck="false" placeholder="Enter a URL" />
        <button id="open-button">Open</button>
        <button id="reload-button" class="alt">Reload</button>
      </div>
      <div id="status" class="status">Ready.</div>
    </header>

    <aside>
      <section class="card">
        <h2>Current Page</h2>
        <h1 id="page-title">Rust Browser</h1>
        <p id="current-url" class="muted">No page loaded yet.</p>
        <p id="page-meta" class="muted" style="margin-top:8px;"></p>
      </section>

      <section class="card">
        <h2>Readable Preview</h2>
        <p id="page-summary" class="muted">Open a page to fetch a structured summary beside the embedded browser.</p>
        <div id="sections"></div>
      </section>

      <section class="card">
        <h2>History</h2>
        <div id="history"></div>
      </section>
    </aside>

    <main>
      <div class="frame-label">Embedded WebView2 Content</div>
    </main>

    <script>
      const state = {{
        history: [],
        sections: []
      }};

      const address = document.getElementById("address");
      const backButton = document.getElementById("back-button");
      const forwardButton = document.getElementById("forward-button");
      const openButton = document.getElementById("open-button");
      const reloadButton = document.getElementById("reload-button");
      const status = document.getElementById("status");
      const pageTitle = document.getElementById("page-title");
      const currentUrl = document.getElementById("current-url");
      const pageMeta = document.getElementById("page-meta");
      const pageSummary = document.getElementById("page-summary");
      const history = document.getElementById("history");
      const sections = document.getElementById("sections");

      function post(message) {{
        window.ipc.postMessage(message);
      }}

      function renderHistory(items) {{
        history.innerHTML = "";
        if (!items.length) {{
          const empty = document.createElement("p");
          empty.className = "muted";
          empty.textContent = "History is empty.";
          history.appendChild(empty);
          return;
        }}

        items.forEach((url) => {{
          const button = document.createElement("button");
          button.className = "history-item";
          button.textContent = url;
          button.addEventListener("click", () => {{
            address.value = url;
            post("navigate:" + url);
          }});
          history.appendChild(button);
        }});
      }}

      function renderSections(items) {{
        sections.innerHTML = "";
        items.forEach((section) => {{
          const wrapper = document.createElement("div");
          wrapper.className = "section";

          if (section.heading) {{
            const heading = document.createElement("h3");
            heading.textContent = section.heading;
            wrapper.appendChild(heading);
          }}

          const body = document.createElement("p");
          body.textContent = section.body;
          wrapper.appendChild(body);
          sections.appendChild(wrapper);
        }});
      }}

      function sendNavigate() {{
        const url = address.value.trim();
        if (!url) return;
        post("navigate:" + url);
      }}

      openButton.addEventListener("click", sendNavigate);
      backButton.addEventListener("click", () => post("back"));
      forwardButton.addEventListener("click", () => post("forward"));
      reloadButton.addEventListener("click", () => post("reload"));
      address.addEventListener("keydown", (event) => {{
        if (event.key === "Enter") {{
          sendNavigate();
        }}
      }});

      window.rustBrowserUpdate = function(next) {{
        Object.assign(state, next);

        address.value = state.address_input || "";
        pageTitle.textContent = state.page_title || "Rust Browser";
        currentUrl.textContent = state.current_url || state.pending_url || "No page loaded yet.";
        pageMeta.textContent = state.page_meta || "";
        pageSummary.textContent = state.page_summary || "No preview available.";
        status.textContent = state.status.message;
        status.className = "status " + state.status.kind;
        openButton.disabled = !!state.is_loading;
        address.disabled = !!state.is_loading;
        backButton.disabled = !!state.is_loading;
        forwardButton.disabled = !!state.is_loading;

        renderHistory(state.history || []);
        renderSections(state.sections || []);
      }};
    </script>
  </body>
</html>"#,
        header_height = HEADER_HEIGHT as i32,
        sidebar_width = SIDEBAR_WIDTH as i32
    )
}

fn placeholder_page_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <style>
      body {
        margin: 0;
        font-family: "Segoe UI", sans-serif;
        display: grid;
        place-items: center;
        min-height: 100vh;
        background: linear-gradient(180deg, #fffaf2, #f2e4d1);
        color: #23303b;
      }
      article {
        max-width: 560px;
        text-align: center;
        padding: 24px;
      }
      h1 { margin-bottom: 12px; }
      p { line-height: 1.6; color: #5c6873; }
    </style>
  </head>
  <body>
    <article>
      <h1>Embedded browser ready</h1>
      <p>Use the address bar in the shell to open a page. The content will render here using WebView2, while the sidebar keeps history and readable extraction.</p>
    </article>
  </body>
</html>"#
        .to_string()
}

fn blocked_page_html(url: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <style>
      body {{
        margin: 0;
        font-family: "Segoe UI", sans-serif;
        display: grid;
        place-items: center;
        min-height: 100vh;
        background: linear-gradient(180deg, #fff6ef, #f2ddd1);
        color: #4f271d;
      }}
      article {{
        max-width: 580px;
        text-align: center;
        padding: 28px;
        border-radius: 20px;
        background: rgba(255,255,255,0.74);
        box-shadow: 0 20px 48px rgba(97, 46, 33, 0.12);
      }}
      p {{
        line-height: 1.6;
        color: #7f584a;
      }}
      code {{
        display: inline-block;
        margin-top: 12px;
        padding: 8px 12px;
        border-radius: 999px;
        background: rgba(97, 46, 33, 0.08);
      }}
    </style>
  </head>
  <body>
    <article>
      <h1>Navigation blocked</h1>
      <p>The requested address was rejected by the browser security filter before WebView2 could open it.</p>
      <code>{}</code>
    </article>
  </body>
</html>"#,
        escape_html(url)
    )
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
