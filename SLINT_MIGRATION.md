# Slint Migration Plan

## Goal

Migrate the browser chrome incrementally to Slint without replacing the embedded web rendering engine.

## Recommendation

- Keep `WebView2` for page rendering.
- Move the browser shell UI to Slint in phases.
- Preserve the current application and infrastructure layers during the migration.

## Why this split

- Slint is a strong fit for native application chrome:
  tabs, toolbar, dialogs, settings, status, and side panels.
- `WebView2` remains the right tool for rendering arbitrary web pages.
- This avoids rewriting the browser engine concerns into a UI toolkit that is not a web renderer.

## Suggested target structure

```text
src/
  presentation/
    browser_shell_slint/
      app_window.slint
      controller.rs
      mod.rs
    webview_host.rs
```

## Incremental phases

### Phase 1

- Introduce a Slint host window for the browser chrome.
- Keep the current Rust event/controller logic.
- Replicate the toolbar, tab strip, status bar, and options dialog in Slint.
- Status: `build.rs` now compiles a real `AppWindow` Slint view, and the production `runner` uses that Slint window as the host for the top chrome while `WebView2` is embedded below it as native content. The old HTML shell is no longer the primary chrome layer.

### Phase 2

- Move the profile selector, history, and preview panels to Slint components.
- Replace the current HTML shell state bridge with Slint property bindings and callbacks.

### Phase 3

- Extract reusable Slint components:
  `Toolbar`, `TabStrip`, `OptionsDialog`, `ProfileSelector`, `PreviewPanel`.
- Add responsive layout variants and keyboard focus behavior directly in `.slint` files.

### Phase 4

- Host `WebView2` inside the Slint-managed native window.
- Keep the current `runner` logic only as a browser controller and `WebView2` bridge.

## Current code to preserve

- `application/`
- `domain/`
- `infrastructure/`
- blocklist loading and profile selection
- navigation and preview worker logic

## Migration risks

- Native embedding of `WebView2` inside a Slint-controlled window is the main integration risk.
- Event routing between Slint callbacks and the webview host should stay thin and well scoped.
- The migration should not move business logic into Slint components.

## Success criteria

- Same navigation behavior as today
- Same blocklist policy enforcement
- Better maintainability for browser chrome
- Cleaner separation between shell UI and browser engine
