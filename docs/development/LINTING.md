# Linting and Code Quality

This document describes the linting and code quality practices for COSMIC ORDER.

## Overview

We use multiple tools to ensure code quality:

| Tool | Purpose | Configuration |
|------|---------|---------------|
| clippy | Rust lints | `Cargo.toml` [lints.clippy], `clippy.toml` |
| rustfmt | Code formatting | `rustfmt.toml` |
| cargo doc | Documentation | Built-in |
| cargo audit | Security audit | Built-in |

## Quick Commands

```bash
# Run all lints
just lint

# Run specific lints
just check          # Clippy (pedantic)
just fmt-check      # Format check only
just lint-docs      # Documentation warnings

# Auto-fix formatting
just fmt

# Pre-commit checks
just pre-commit
```

## Clippy Configuration

### Denied Patterns (Errors)

These patterns will cause build failure:

```toml
[lints.clippy]
unwrap_used = "deny"    # Use proper error handling
expect_used = "deny"    # Use proper error handling
panic = "deny"          # Never panic in production
unsafe_code = "deny"    # No unsafe code
```

### Warned Patterns

These generate warnings but don't fail the build:

- `todo!()` - Mark incomplete code
- `unimplemented!()` - Mark unimplemented features
- `dbg!()` - Debug macro left in code
- Various code quality issues (see `Cargo.toml`)

### Allowed Patterns

Some common patterns are explicitly allowed:

```toml
too_many_arguments = "allow"  # Complex constructors OK
type_complexity = "allow"      # libcosmic types can be complex
module_inception = "allow"     # mod.rs pattern
```

## Error Handling

Instead of `unwrap()` or `expect()`, use:

```rust
// Pattern 1: Match with logging
match operation() {
    Ok(value) => { /* use value */ }
    Err(e) => {
        tracing::error!("Operation failed: {e}");
        return default_value();
    }
}

// Pattern 2: If-let with early return
let Ok(value) = operation() else {
    tracing::error!("Operation failed");
    return Task::none();
};

// Pattern 3: Result propagation (when appropriate)
let value = operation()?;

// Pattern 4: unwrap_or_default for optional values
let value = config.get("key").unwrap_or_default();
```

## Formatting

### Configuration (`rustfmt.toml`)

```toml
edition = "2024"
max_width = 100
imports_granularity = "Module"
group_imports = "StdExternalCrate"
```

### Import Organization

```rust
// Standard library first
use std::collections::HashMap;
use std::path::PathBuf;

// External crates
use cosmic::widget;
use serde::{Deserialize, Serialize};

// Internal modules
use crate::config::Config;
use crate::pages::PageId;
```

## Documentation

### Required Documentation

- All public items (`pub fn`, `pub struct`, etc.)
- Module-level documentation (`//!`)
- Complex private functions

### Format

```rust
/// Short description of the function.
///
/// Longer description if needed, explaining behavior,
/// edge cases, or important details.
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// Conditions that cause errors
///
/// # Examples
///
/// ```
/// let result = function(value);
/// ```
pub fn function(param: Type) -> Result<Value, Error> {
    // ...
}
```

## Pre-commit Hook (optional)

Example hook at `.git/hooks/pre-commit`:

```bash
#!/bin/bash
set -e
just pre-commit
```

Make it executable with `chmod +x .git/hooks/pre-commit`, or simply run
`just pre-commit` manually before committing.

## CI Integration

The following checks run in CI:

1. `just fmt-check` - Formatting
2. `just check` - Clippy (pedantic)
3. `just test` - Tests
4. `just lint-docs` - Documentation builds

All checks must pass for PRs to be merged.

## Security Auditing

Run periodically or before releases:

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
just audit
```

## IDE Integration

### VS Code / Cursor

Settings for `.vscode/settings.json`:

```json
{
    "rust-analyzer.check.command": "clippy",
    "rust-analyzer.check.extraArgs": ["--all-features"],
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    }
}
```

### Neovim

With rust-analyzer:

```lua
require('lspconfig').rust_analyzer.setup({
    settings = {
        ["rust-analyzer"] = {
            check = { command = "clippy" },
        }
    }
})
```
