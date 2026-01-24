# Contributing to COSMIC ORDER

Thank you for your interest in contributing to COSMIC ORDER!

## Code of Conduct

Be respectful and constructive. We follow the same standards as the
COSMIC desktop project.

## Getting Started

1. Read [SETUP.md](SETUP.md) to set up your development environment
2. Review [../architecture/OVERVIEW.md](../architecture/OVERVIEW.md)
3. Check [../ROADMAP.md](../ROADMAP.md) for current priorities

## Development Workflow

### 1. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-description
```

### 2. Make Changes

- Follow the code standards below
- Write tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
# Run lints
cargo clippy --all-features

# Run formatter
cargo fmt

# Run tests
cargo test

# Test the application
cargo run --release
```

### 4. Commit

Write clear commit messages:

```text
feat(themes): add theme export functionality

- Add export button to theme page
- Implement RON serialization for themes
- Add file picker dialog
```

Prefixes:

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `refactor`: Code refactoring
- `test`: Tests
- `chore`: Maintenance

### 5. Submit Pull Request

- Describe what your PR does
- Reference any related issues
- Include screenshots for UI changes

## Code Standards

### Rust Style

- Follow `rustfmt` defaults
- Use `cargo clippy` warnings as errors
- Prefer explicit error handling over panics

### File Headers

All Rust files must start with:

```rust
// SPDX-License-Identifier: GPL-3.0-only
```

### Error Handling

```rust
// Don't do this
let value = operation().unwrap();

// Do this instead
let value = match operation() {
    Ok(v) => v,
    Err(e) => {
        tracing::error!("Operation failed: {e}");
        return Task::none();
    }
};
```

### Logging

Use structured logging:

```rust
tracing::info!(page = %page_id, "Page loaded");
tracing::error!(error = %e, "Failed to save config");
```

### Localization

All user-facing strings must use `fl!`:

```rust
// Don't do this
widget::text("Settings")

// Do this
widget::text(fl!("settings"))
```

### Documentation

- Document public items with `///` comments
- Include examples for complex APIs
- Keep comments up-to-date with code

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature() {
        // Arrange
        let input = ...;

        // Act
        let result = function(input);

        // Assert
        assert_eq!(result, expected);
    }
}
```

### Integration Tests

Place in `tests/` directory for cross-module testing.

## Documentation Updates

When making changes:

1. Update relevant docs in `docs/`
2. Update ROADMAP.md if completing tasks
3. Add/update code comments

## Questions?

Open an issue for:

- Feature proposals
- Bug reports
- Questions about the codebase

## License

By contributing, you agree that your contributions will be licensed
under GPL-3.0-only.
