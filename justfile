# COSMIC ORDER - Build and Development Commands
# Run 'just --list' to see all available commands

# Default recipe: build release
default: build-release

# Build in debug mode
build:
    cargo build

# Build in release mode
build-release:
    cargo build --release

# Run the application
run:
    RUST_LOG=cosmic_order=info cargo run --release 2>&1

# Run with debug logging
run-debug:
    RUST_LOG=cosmic_order=debug,libcosmic=info cargo run 2>&1

# Run with trace logging
run-trace:
    RUST_LOG=cosmic_order=trace cargo run 2>&1

# Run all lints
lint: lint-clippy lint-fmt lint-docs

# Run clippy lints
lint-clippy:
    cargo clippy --all-features -- -D warnings

# Check code formatting
lint-fmt:
    cargo fmt -- --check

# Check documentation
lint-docs:
    cargo doc --no-deps --document-private-items 2>&1 | grep -E "^warning:" || true

# Format code
fmt:
    cargo fmt

# Run all checks (lint + test)
check: lint test

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Clean build artifacts
clean:
    cargo clean

# Update dependencies
update:
    cargo update

# Generate documentation
doc:
    cargo doc --no-deps --open

# Audit dependencies for security issues
audit:
    cargo audit

# Check compilation without building
check-compile:
    cargo check --all-features

# Install locally (binary only)
install:
    cargo install --path .

# Install with desktop integration
install-desktop: build-release
    @echo "Installing binary..."
    sudo install -Dm755 target/release/cosmic-order /usr/local/bin/cosmic-order
    @echo "Installing desktop file..."
    sudo install -Dm644 resources/com.github.jfreed-dev.CosmicOrder.desktop /usr/share/applications/com.github.jfreed-dev.CosmicOrder.desktop
    @echo "Installing icon..."
    sudo install -Dm644 resources/icons/com.github.jfreed-dev.CosmicOrder.svg /usr/share/icons/hicolor/scalable/apps/com.github.jfreed-dev.CosmicOrder.svg
    @echo "Updating icon cache..."
    sudo gtk-update-icon-cache -f /usr/share/icons/hicolor/ || true
    @echo "Installation complete!"

# Install to user directory (no sudo)
install-user: build-release
    @echo "Installing binary..."
    install -Dm755 target/release/cosmic-order ~/.local/bin/cosmic-order
    @echo "Installing desktop file..."
    install -Dm644 resources/com.github.jfreed-dev.CosmicOrder.desktop ~/.local/share/applications/com.github.jfreed-dev.CosmicOrder.desktop
    @echo "Installing icon..."
    install -Dm644 resources/icons/com.github.jfreed-dev.CosmicOrder.svg ~/.local/share/icons/hicolor/scalable/apps/com.github.jfreed-dev.CosmicOrder.svg
    @echo "Updating icon cache..."
    gtk-update-icon-cache -f ~/.local/share/icons/hicolor/ || true
    @echo "User installation complete!"

# Uninstall
uninstall:
    cargo uninstall cosmic-order || true
    sudo rm -f /usr/local/bin/cosmic-order
    sudo rm -f /usr/share/applications/com.github.jfreed-dev.CosmicOrder.desktop
    sudo rm -f /usr/share/icons/hicolor/scalable/apps/com.github.jfreed-dev.CosmicOrder.svg

# Uninstall from user directory
uninstall-user:
    rm -f ~/.local/bin/cosmic-order
    rm -f ~/.local/share/applications/com.github.jfreed-dev.CosmicOrder.desktop
    rm -f ~/.local/share/icons/hicolor/scalable/apps/com.github.jfreed-dev.CosmicOrder.svg

# Show dependency tree
deps:
    cargo tree

# Show outdated dependencies
outdated:
    cargo outdated

# Run with memory profiler (requires heaptrack)
heaptrack:
    heaptrack cargo run --release

# Pre-commit checks (run before committing)
pre-commit: lint-fmt lint-clippy test
    @echo "All pre-commit checks passed!"
