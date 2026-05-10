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

# Activate tracked git hooks (.githooks/) for this clone
setup-hooks:
    git config core.hooksPath .githooks
    @echo "Hooks active: pre-commit (just pre-commit), pre-push (just health-check-quick)"

# --- Release pipeline (local CI/CD) ---

# Output dir for release artifacts
dist-dir := 'dist'
deb-builder-image := 'cosmic-order-deb-builder:noble'

# Build the .deb-builder docker image (cached after first run)
release-image:
    docker build -f scripts/Dockerfile.deb-builder -t {{deb-builder-image}} scripts/

# Pre-flight checks for release: clean tree, on main, version sanity, tests
release-check VERSION:
    @if [ -n "$(git status --porcelain)" ]; then echo "release-check: working tree not clean"; exit 1; fi
    @if [ "$(git rev-parse --abbrev-ref HEAD)" != "main" ]; then echo "release-check: not on main branch"; exit 1; fi
    @if ! grep -q '^version = "{{VERSION}}"' Cargo.toml; then echo "release-check: Cargo.toml version != {{VERSION}}"; exit 1; fi
    @if ! head -1 debian/changelog | grep -q "^cosmic-order ({{VERSION}})"; then echo "release-check: top of debian/changelog != {{VERSION}}"; exit 1; fi
    @if git rev-parse "v{{VERSION}}" >/dev/null 2>&1; then echo "release-check: tag v{{VERSION}} already exists"; exit 1; fi
    just health-check-quick

# Build .deb in debian:noble container; result lands in dist/
#
# dpkg-buildpackage writes its outputs (.deb, .buildinfo, .changes) to
# the parent of the source tree. Mount the project's parent dir into
# /build so the container user can write there, and cd into the project
# subdir before running dpkg-buildpackage.
release-deb VERSION: release-image vendor
    mkdir -p {{dist-dir}}
    docker run --rm \
        --user "$(id -u):$(id -g)" \
        -v "$(pwd)/..":/build \
        -w "/build/$(basename $(pwd))" \
        -e HOME=/tmp \
        -e VENDOR=0 \
        {{deb-builder-image}} \
        bash -c "git config --global --add safe.directory '*' && dpkg-buildpackage -us -uc -b -d"
    mv ../cosmic-order_{{VERSION}}_*.deb ../cosmic-order_{{VERSION}}_*.buildinfo ../cosmic-order_{{VERSION}}_*.changes {{dist-dir}}/ 2>/dev/null || true
    just clean-vendor
    @echo "Built .deb in {{dist-dir}}/"

# Full release: pre-flight + .deb + annotated tag. Push tag manually after.
release VERSION: (release-check VERSION) (release-deb VERSION)
    git tag -a "v{{VERSION}}" -m "Release v{{VERSION}}"
    @echo ""
    @echo "Release v{{VERSION}} ready."
    @echo "  Tag:     v{{VERSION}} (push: git push origin v{{VERSION}})"
    @echo "  Package: $(ls {{dist-dir}}/cosmic-order_{{VERSION}}_*.deb 2>/dev/null | head -1)"
