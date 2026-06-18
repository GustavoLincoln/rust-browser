# Clean Architecture Overview

## Folder structure

```text
src/
  application/
    browser_service.rs    # Orchestrates navigation policy and browser state
    ports.rs              # Application boundary traits
  domain/
    blocklist.rs          # Pure blocking rules
    browser.rs            # Browser state and behavior
    filter.rs             # Policy abstraction
    page.rs               # Domain model for loaded pages
  infrastructure/
    blocklist/
      file_blocklist_policy.rs   # Reads rules from blocklist.txt
    http/
      page_fetcher.rs            # Fetches pages and extracts structured readable content
  presentation/
    webview_shell/
      runner.rs                  # Native window + embedded WebView2 shell
  main.rs                 # Composition root
```

## Responsibilities

- `domain` contains business rules and has no knowledge of files, databases, or CLI output.
- `application` coordinates the use cases and depends only on abstractions and domain objects.
- `infrastructure` adapts external concerns like file loading.
- `presentation` owns only UI concerns and delegates navigation logic to the application layer.
- `main.rs` wires the application together in one place.

## Navigation flow

- The shell UI asks the application layer to normalize and validate the target URL.
- If the URL is allowed, the embedded `WebView2` child view navigates to the page inside the native window.
- In parallel, a background worker fetches the page through the HTTP adapter and extracts semantic sections from the HTML.
- When the page load and preview results return, the application layer commits browser history and the shell updates the sidebar state.

## Refactor decisions

- Merged the useful blocklist behavior into `src/` and removed duplicate root modules.
- Replaced placeholder renderer/network/core files with cohesive modules that have clear ownership.
- Replaced the prototype UI with a native shell backed by `winit + wry`, using Microsoft Edge WebView2 on Windows.
- Introduced an HTTP infrastructure adapter to fetch pages and render a structured readable preview beside the embedded browser.
- Moved preview loading to a background workflow so the shell stays responsive during navigation.
- Kept URL validation and blocklist checks in the application/domain flow before any page load happens.
