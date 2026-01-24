# Development Environment Setup

Guide to setting up a development environment for COSMIC Tweaks.

## Prerequisites

### Operating System

- Pop!_OS 24.04 LTS or later with COSMIC Desktop
- Ubuntu 24.04+ with COSMIC (when available)
- Other Linux distributions with COSMIC

### Required Tools

| Tool | Version | Purpose |
|------|---------|---------|
| Rust | 1.85+ | Programming language |
| Cargo | (with Rust) | Package manager |
| just | latest | Task runner |
| git | latest | Version control |

## Installation Steps

### 1. Install System Dependencies

```bash
# Pop!_OS / Ubuntu / Debian
sudo apt update
sudo apt install -y \
    build-essential \
    cargo \
    cmake \
    git \
    just \
    libexpat1-dev \
    libfontconfig-dev \
    libfreetype-dev \
    libxkbcommon-dev \
    pkg-config
```

### 2. Install Rust (if not using system package)

```bash
# Install rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Restart shell or source env
source ~/.cargo/env

# Verify installation
rustc --version  # Should be 1.85+
cargo --version
```

### 3. Clone the Repository

```bash
cd ~/Repos
git clone https://github.com/YOUR_USERNAME/cosmic-tweaks.git
cd cosmic-tweaks
```

### 4. Build the Project

```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized)
cargo build --release

# Run the application
cargo run --release
```

## IDE Setup

### VS Code / Cursor

Install extensions:

- **rust-analyzer** - Rust language support
- **Even Better TOML** - TOML file support
- **CodeLLDB** - Debugger

Recommended settings (`.vscode/settings.json`):

```json
{
    "rust-analyzer.cargo.features": "all",
    "rust-analyzer.checkOnSave.command": "clippy",
    "editor.formatOnSave": true,
    "[rust]": {
        "editor.defaultFormatter": "rust-lang.rust-analyzer"
    }
}
```

### Neovim

With `nvim-lspconfig`:

```lua
require('lspconfig').rust_analyzer.setup({
    settings = {
        ["rust-analyzer"] = {
            cargo = { features = "all" },
            checkOnSave = { command = "clippy" },
        }
    }
})
```

## Development Workflow

### Running During Development

```bash
# Run with debug logging
RUST_LOG=debug cargo run

# Run with specific log level
RUST_LOG=cosmic_tweaks=trace cargo run

# Run release build
cargo run --release
```

### Checking Code Quality

```bash
# Run clippy lints
cargo clippy --all-features

# Format code
cargo fmt

# Check without building
cargo check --all-features
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

## Troubleshooting

### Build Errors

**Missing system libraries**:

```bash
# Check for missing libraries
pkg-config --list-all | grep -i font
pkg-config --list-all | grep -i xkb
```

**Rust version too old**:

```bash
# Update Rust
rustup update stable
```

### Runtime Errors

**Application doesn't start**:

```bash
# Check for Wayland
echo $XDG_SESSION_TYPE  # Should be "wayland"

# Check COSMIC is running
pgrep cosmic-comp
```

**Theme not loading**:

```bash
# Verify cosmic-config daemon
systemctl --user status cosmic-config-daemon
```

## Useful Commands

```bash
# Clean build artifacts
cargo clean

# Update dependencies
cargo update

# Generate documentation
cargo doc --open

# Check dependency tree
cargo tree

# Security audit
cargo audit
```

## Next Steps

After setup:

1. Read [CONTRIBUTING.md](CONTRIBUTING.md) for code standards
2. Review [../architecture/OVERVIEW.md](../architecture/OVERVIEW.md)
3. Check [../ROADMAP.md](../ROADMAP.md) for current tasks
