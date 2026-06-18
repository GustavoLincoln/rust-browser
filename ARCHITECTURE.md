# Clean Architecture Overview

## Folder structure

```text
src/
  application/
    browser_service.rs    # Orchestrates navigation policy and browser state
    ports.rs              # Application boundary traits
    runtime.rs            # Startup configuration and service bootstrap
  domain/
    blocklist.rs          # Pure blocking rules
    blocklist_profile.rs  # Light/Normal/Pro profile selection
    browser.rs            # Browser state and behavior
    filter.rs             # Policy abstraction
    page.rs               # Domain model for loaded pages
  infrastructure/
    blocklist/
      file_blocklist_policy.rs   # Reads rules from blocklist.txt
      hagezi_catalog.rs          # Maps profiles to HaGeZi domains-format lists
      source_loader.rs           # Merges local and optional remote DNS blocklists with cache fallback
    config/
      app_settings.rs            # Persists selected profile and UI settings
    http/
      page_fetcher.rs            # Fetches pages and extracts structured readable content
  presentation/
    browser_shell_slint/
      app_window.slint          # Phase 1 Slint shell scaffold and target layout
      bridge.rs                 # Syncs shared shell state into generated Slint components
      controller.rs             # Future bridge between app state and Slint callbacks
      mod.rs                    # Slint presentation entrypoint
    webview_shell/
      templates.rs               # Placeholder and blocked page templates for the embedded content
      runner.rs                  # Slint-hosted window, WebView2 integration, tabs, state and events
  main.rs                 # Composition root
build.rs                  # Compiles Slint UI definitions into Rust types
SLINT_MIGRATION.md        # Incremental plan for moving browser chrome to Slint
```

## Shell component architecture

- `browser_shell_slint/app_window.slint` now owns the visible browser chrome: top bar, address/navigation controls, window actions, status bar, and options panel.
- `browser_shell_slint/controller.rs` is the shared UI view model boundary between runtime state and Slint properties/callbacks.
- `browser_shell_slint/bridge.rs` applies the runtime snapshot to the generated Slint component and converts Slint callbacks into browser actions.
- `webview_shell/runner.rs` now focuses on runtime state, browser events, per-tab state, blocklist switching, and hosting the `WebView2` child inside the Slint-managed native window.
- `webview_shell/templates.rs` remains only for auxiliary embedded content pages such as placeholder and blocked states.

## Responsibilities

- `domain` contains business rules and has no knowledge of files, databases, or CLI output.
- `application` coordinates the use cases and depends only on abstractions and domain objects.
- `infrastructure` adapts external concerns like file loading.
- `presentation` owns only UI concerns and delegates navigation logic to the application layer.
- `main.rs` wires the application together in one place.

## Blocklist flow

- The startup runtime reads `RUST_BROWSER_BLOCKLIST_PATH` for the local list path.
- Optionally, `RUST_BROWSER_BLOCKLIST_URL` points to a remote domains-format blocklist and `RUST_BROWSER_BLOCKLIST_CACHE` defines where its cached copy lives.
- If no remote URL override is provided, the selected profile maps to a HaGeZi domains list:
  `Light -> light.txt`, `Normal -> multi.txt`, `Pro -> pro.txt`.
- The source loader merges local and remote content, caching successful downloads and falling back to the cache if the remote fetch fails.
- The domain blocklist matcher blocks exact hosts and subdomains, avoiding accidental substring matches.

## Navigation flow

- The shell UI asks the application layer to normalize and validate the target URL.
- If the URL is allowed, the embedded `WebView2` child view navigates to the page inside the native window.
- In parallel, a background worker fetches the page through the HTTP adapter and extracts semantic sections from the HTML.
- When the page load and preview results return, the application layer commits navigation and the shell updates the active tab state.

## Refactor decisions

- Merged the useful blocklist behavior into `src/` and removed duplicate root modules.
- Replaced placeholder renderer/network/core files with cohesive modules that have clear ownership.
- Replaced the prototype UI with a native shell backed by `winit + wry`, using Microsoft Edge WebView2 on Windows.
- Introduced an HTTP infrastructure adapter to fetch pages and render a structured readable preview beside the embedded browser.
- Moved preview loading to a background workflow so the shell stays responsive during navigation.
- Kept URL validation and blocklist checks in the application/domain flow before any page load happens.
- Added remote blocklist ingestion with cache fallback so curated DNS lists such as HaGeZi can be consumed without hardcoding them into the repository.
- Replaced substring-based blocking with host/suffix matching suited to DNS domains-format blocklists.
