# Contributing to COSMIC ORDER

## Getting Started

1. Read [SETUP.md](SETUP.md) to set up your development environment
2. Review [../architecture/OVERVIEW.md](../architecture/OVERVIEW.md)
3. Check [../ROADMAP.md](../ROADMAP.md) for current priorities

## Build and Test

```bash
just              # build release
just run           # run with info logging
just check         # clippy pedantic
just fmt           # format code
just test          # run tests
just pre-commit    # fmt + clippy + tests
```

## Code Standards

### File Headers

All Rust files must start with:

```rust
// SPDX-License-Identifier: GPL-3.0-only
```

### Error Handling

Avoid `unwrap()` and `expect()` in production code. Use pattern matching
or `tracing::error!()` for failures.

```rust
let value = match operation() {
    Ok(v) => v,
    Err(e) => {
        tracing::error!("Operation failed: {e}");
        return Task::none();
    }
};
```

### Logging

Use `tracing` macros:

```rust
tracing::info!(page = %page_id, "Page loaded");
tracing::error!(error = %e, "Failed to save config");
```

### Localization

All user-facing strings must use `fl!`:

```rust
widget::text(fl!("settings"))
```

## Commit Messages

Use conventional commits:

```text
feat: add new feature
fix: fix a bug
docs: update documentation
refactor: code refactoring
test: add tests
chore: maintenance tasks
```

## License

By submitting a pull request, you agree to license your contribution under
GPL-3.0-only. See [../../LICENSE](../../LICENSE).

All contributions are subject to the
[Developer Certificate of Origin](https://developercertificate.org/).
