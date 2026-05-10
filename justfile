# COSMIC ORDER - Build and Development Commands
# Follows cosmic-app-template conventions.
# Run 'just --list' to see all available commands.

# Application binary name
name := 'cosmic-order'
# Application ID (reverse-domain)
appid := 'com.github.jfreed-dev.CosmicOrder'

# Path to root file system (for packaging)
rootdir := ''
# Prefix for /usr directory
prefix := '/usr'
# Cargo target directory
cargo-target-dir := env('CARGO_TARGET_DIR', 'target')

# Resource file names
desktop := appid + '.desktop'
icon-svg := appid + '.svg'
metainfo := appid + '.metainfo.xml'

# Install destinations
base-dir := absolute_path(clean(rootdir / prefix))
bin-dst := base-dir / 'bin' / name
desktop-dst := base-dir / 'share' / 'applications' / desktop
icons-dst := base-dir / 'share' / 'icons' / 'hicolor'
icon-svg-dst := icons-dst / 'scalable' / 'apps' / icon-svg
metainfo-dst := base-dir / 'share' / 'metainfo' / metainfo
screensaver-dst := base-dir / 'share' / name / 'screensaver'

# Default recipe
default: build-release

# Compile debug profile
build-debug *args:
    cargo build {{args}}

# Compile release profile
build-release *args: (build-debug '--release' args)

# Compile release profile with vendored dependencies
build-vendored *args: vendor-extract (build-release '--frozen --offline' args)

# Run clippy check
check *args:
    cargo clippy --all-features {{args}} -- -W clippy::pedantic

# Run clippy check with JSON output
check-json: (check '--message-format=json')

# Run the application
run *args:
    env RUST_BACKTRACE=full RUST_LOG=cosmic_order=info cargo run --release {{args}} 2>&1

# Run with debug logging
run-debug *args:
    env RUST_LOG=cosmic_order=debug,libcosmic=info cargo run {{args}} 2>&1

# Run with trace logging
run-trace *args:
    env RUST_LOG=cosmic_order=trace cargo run {{args}} 2>&1

# Run tests
test *args:
    cargo test {{args}}

# Run tests with output
test-verbose: (test '-- --nocapture')

# Format code
fmt:
    cargo fmt

# Check code formatting
fmt-check:
    cargo fmt -- --check

# Run all lints (clippy + format + docs)
lint: check fmt-check lint-docs

# Check documentation warnings
lint-docs:
    cargo doc --no-deps --document-private-items 2>&1 | grep -E "^warning:" || true

# Pre-commit checks
pre-commit: fmt-check check test
    @echo "All pre-commit checks passed!"

# Clean build artifacts
clean:
    cargo clean

# Remove vendored dependencies
clean-vendor:
    rm -rf .cargo vendor vendor.tar

# Full clean (build artifacts + vendored deps)
clean-dist: clean clean-vendor

# Install binary, desktop file, icon, metainfo, and screensaver scripts
install:
    install -Dm0755 {{cargo-target-dir / 'release' / name}} {{bin-dst}}
    install -Dm0644 {{'resources' / desktop}} {{desktop-dst}}
    install -Dm0644 {{'resources' / 'icons' / icon-svg}} {{icon-svg-dst}}
    install -Dm0644 {{'resources' / metainfo}} {{metainfo-dst}}
    install -Dm0755 resources/screensaver/screensaver-ctl.sh {{screensaver-dst}}/screensaver-ctl.sh
    install -Dm0755 resources/screensaver/launch-fullscreen.sh {{screensaver-dst}}/launch-fullscreen.sh
    install -Dm0755 resources/screensaver/cosmic-screensaver.sh {{screensaver-dst}}/cosmic-screensaver.sh
    install -d {{screensaver-dst}}/logos
    install -m0644 resources/screensaver/logos/*.txt {{screensaver-dst}}/logos/

# Uninstall installed files
uninstall:
    rm -f {{bin-dst}}
    rm -f {{desktop-dst}}
    rm -f {{icon-svg-dst}}
    rm -f {{metainfo-dst}}
    rm -rf {{screensaver-dst}}

# Vendor dependencies locally
vendor:
    mkdir -p .cargo
    cargo vendor | head -n -1 > .cargo/config.toml
    echo 'directory = "vendor"' >> .cargo/config.toml
    tar pcf vendor.tar vendor
    rm -rf vendor

# Extract vendored dependencies
vendor-extract:
    rm -rf vendor
    tar pxf vendor.tar

# Update dependencies
update:
    cargo update

# Generate and open documentation
doc:
    cargo doc --no-deps --open

# Audit dependencies for security issues
audit:
    cargo audit

# Show dependency tree
deps:
    cargo tree

# Show outdated dependencies
outdated:
    cargo outdated

# Run health check (full system validation)
health-check:
    ./scripts/health-check.sh

# Run quick health check (build checks only, no runtime)
health-check-quick:
    ./scripts/health-check.sh --quick

# Run with memory profiler (requires heaptrack)
heaptrack:
    heaptrack cargo run --release
