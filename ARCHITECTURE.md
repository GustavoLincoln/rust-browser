# Clean Architecture Overview

## Folder structure

```text
src/
  application/
    browser_service.rs    # Orchestrates browser use cases
  domain/
    blocklist.rs          # Pure blocking rules
    bookmark.rs           # Domain entity
    browser.rs            # Browser state and behavior
    filter.rs             # Policy abstraction
  infrastructure/
    blocklist/
      file_blocklist_policy.rs   # Reads rules from blocklist.txt
    storage/
      bookmark_store.rs          # Persists bookmarks with sled
  presentation/
    cli.rs                # Terminal simulation/output
  main.rs                 # Composition root
```

## Responsibilities

- `domain` contains business rules and has no knowledge of files, databases, or CLI output.
- `application` coordinates the use cases and depends only on abstractions and domain objects.
- `infrastructure` adapts external concerns like file loading and persistence.
- `presentation` formats the interaction layer without owning business logic.
- `main.rs` wires the application together in one place.

## Refactor decisions

- Merged the useful blocklist behavior into `src/` and removed duplicate root modules.
- Replaced placeholder renderer/network/core files with cohesive modules that have clear ownership.
- Reduced dependency noise in `Cargo.toml` to match the code that remains in the project.
- Kept the observable behavior aligned with the current program: initialize, evaluate URLs, and print allowed/blocked results.
