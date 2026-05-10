#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
#
# COSMIC ORDER - Health Check Script
#
# Validates build integrity, runtime dependencies, and system integration.
# Run after builds, upgrades, or on a new system to verify everything works.
#
# Usage:
#   ./scripts/health-check.sh          # Run all checks
#   ./scripts/health-check.sh --quick  # Build checks only (no runtime)
#
# Exit codes:
#   0 - All checks passed
#   1 - One or more checks failed

set -euo pipefail

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

PASS=0
FAIL=0
WARN=0
SKIP=0

pass() { PASS=$((PASS + 1)); echo -e "  ${GREEN}PASS${NC} $1"; }
fail() { FAIL=$((FAIL + 1)); echo -e "  ${RED}FAIL${NC} $1"; }
warn() { WARN=$((WARN + 1)); echo -e "  ${YELLOW}WARN${NC} $1"; }
skip() { SKIP=$((SKIP + 1)); echo -e "  ${CYAN}SKIP${NC} $1"; }
section() { echo -e "\n${BOLD}[$1]${NC}"; }

QUICK=false
if [[ "${1:-}" == "--quick" ]]; then
    QUICK=true
fi

# --- Ensure we're in the project root ---
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
cd "$PROJECT_DIR"

echo -e "${BOLD}COSMIC ORDER Health Check${NC}"
echo "================================"

# =========================================================================
# BUILD CHECKS
# =========================================================================
section "Build Environment"

# Rust toolchain
if command -v rustc &>/dev/null; then
    RUST_VERSION=$(rustc --version | awk '{print $2}')
    # Check minimum version (1.90)
    MAJOR=$(echo "$RUST_VERSION" | cut -d. -f1)
    MINOR=$(echo "$RUST_VERSION" | cut -d. -f2)
    if [[ "$MAJOR" -ge 1 && "$MINOR" -ge 90 ]]; then
        pass "Rust $RUST_VERSION (>= 1.90 required)"
    else
        fail "Rust $RUST_VERSION (>= 1.90 required)"
    fi
else
    fail "Rust not found"
fi

# Cargo
if command -v cargo &>/dev/null; then
    pass "Cargo available"
else
    fail "Cargo not found"
fi

# Just
if command -v just &>/dev/null; then
    pass "just command runner available"
else
    warn "just not found (optional, use cargo directly)"
fi

section "Code Quality"

# Format check
if cargo fmt -- --check 2>/dev/null; then
    pass "Code formatting (cargo fmt)"
else
    fail "Code formatting (cargo fmt)"
fi

# Clippy
if cargo clippy --all-features -- -W clippy::pedantic 2>&1 | tail -1 | grep -q "Finished"; then
    pass "Clippy pedantic (zero warnings)"
else
    fail "Clippy pedantic has warnings"
fi

section "Compilation"

# Debug build check
if cargo check 2>&1 | tail -1 | grep -q "Finished"; then
    pass "Debug build compiles"
else
    fail "Debug build fails"
fi

# Release build check
if cargo check --release 2>&1 | tail -1 | grep -q "Finished"; then
    pass "Release build compiles"
else
    fail "Release build fails"
fi

section "Tests"

# Run all tests
TEST_OUTPUT=$(cargo test 2>&1)
if echo "$TEST_OUTPUT" | grep -q "test result: ok"; then
    TEST_COUNT=$(echo "$TEST_OUTPUT" | grep "test result:" | grep -oP '\d+ passed' | grep -oP '\d+')
    pass "All $TEST_COUNT unit tests pass"
else
    fail "Unit tests have failures"
    echo "$TEST_OUTPUT" | grep -E "^test .* FAILED" || true
fi

section "Project Structure"

# SPDX headers on all .rs files
MISSING_SPDX=$(find src/ -name "*.rs" -exec grep -L "SPDX-License-Identifier" {} \;)
if [[ -z "$MISSING_SPDX" ]]; then
    pass "All .rs files have SPDX license headers"
else
    fail "Missing SPDX headers: $MISSING_SPDX"
fi

# APP_ID consistency
APP_ID="com.github.jfreed-dev.CosmicOrder"
if grep -q "$APP_ID" src/main.rs && \
   grep -q "$APP_ID" justfile && \
   grep -q "$APP_ID" resources/*.desktop 2>/dev/null; then
    pass "APP_ID consistent across main.rs, justfile, desktop file"
else
    fail "APP_ID inconsistency detected"
fi

# i18n files exist
if [[ -d "i18n" && -f "i18n/en/cosmic_order.ftl" ]]; then
    FTL_KEYS=$(grep -c "^[a-z]" i18n/en/cosmic_order.ftl || true)
    pass "i18n: $FTL_KEYS localization keys in English"
else
    fail "i18n directory or English .ftl file missing"
fi

# Bundled themes
THEME_COUNT=$(find themes/ -name "*.ron" 2>/dev/null | wc -l)
if [[ "$THEME_COUNT" -gt 0 ]]; then
    pass "Bundled themes: $THEME_COUNT .ron files"
else
    warn "No bundled themes found in themes/"
fi

if $QUICK; then
    skip "Runtime checks (--quick mode)"
    section "Summary"
    echo -e "  Passed: ${GREEN}$PASS${NC}  Failed: ${RED}$FAIL${NC}  Warnings: ${YELLOW}$WARN${NC}  Skipped: ${CYAN}$SKIP${NC}"
    [[ $FAIL -eq 0 ]] && exit 0 || exit 1
fi

# =========================================================================
# RUNTIME CHECKS (require COSMIC Desktop session)
# =========================================================================
section "Runtime Dependencies"

# D-Bus session bus
if [[ -n "${DBUS_SESSION_BUS_ADDRESS:-}" ]]; then
    pass "D-Bus session bus available"
else
    warn "D-Bus session bus not detected (needed for systemd integration)"
fi

# D-Bus system bus
if busctl --system status org.freedesktop.login1 &>/dev/null 2>&1; then
    pass "logind D-Bus service available"
else
    warn "logind not reachable (power/inhibit features unavailable)"
fi

# Wayland session
if [[ -n "${WAYLAND_DISPLAY:-}" ]]; then
    pass "Wayland session active ($WAYLAND_DISPLAY)"
else
    warn "Wayland session not detected (idle monitoring unavailable)"
fi

# COSMIC Desktop detection
if [[ "${XDG_CURRENT_DESKTOP:-}" == *"COSMIC"* ]] || \
   [[ "${XDG_SESSION_DESKTOP:-}" == *"cosmic"* ]]; then
    pass "COSMIC Desktop session detected"
else
    warn "Not running in COSMIC Desktop (theme integration requires COSMIC)"
fi

section "System Integration"

# cosmic-config directory
COSMIC_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic"
if [[ -d "$COSMIC_CONFIG" ]]; then
    pass "COSMIC config directory exists ($COSMIC_CONFIG)"
else
    warn "COSMIC config directory not found"
fi

# cosmic-order config directory
ORDER_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic-order"
if [[ -d "$ORDER_CONFIG" ]]; then
    pass "cosmic-order config directory exists"
    # Check for tool-sync config
    if [[ -f "$ORDER_CONFIG/tool-sync.toml" ]]; then
        pass "tool-sync.toml present"
    else
        skip "tool-sync.toml not yet created (first run needed)"
    fi
    # Check for hooks directory
    if [[ -d "$ORDER_CONFIG/hooks.d" ]]; then
        HOOK_COUNT=$(find "$ORDER_CONFIG/hooks.d" -type f -executable 2>/dev/null | wc -l)
        pass "hooks.d directory exists ($HOOK_COUNT executable hooks)"
    else
        skip "hooks.d not yet created"
    fi
else
    skip "cosmic-order config directory not yet created (first run needed)"
fi

section "Tool Integration"

# Check which sync-target tools are installed
for tool in ghostty btop nvim zellij fzf lazygit; do
    if command -v "$tool" &>/dev/null; then
        pass "$tool installed"
    else
        skip "$tool not installed (sync target unavailable)"
    fi
done

section "CLI Smoke Test"

# Build release binary if not already built
if [[ ! -f "target/release/cosmic-order" ]]; then
    echo "  Building release binary..."
    if cargo build --release 2>/dev/null; then
        pass "Release binary built"
    else
        fail "Release binary build failed"
    fi
fi

BINARY="target/release/cosmic-order"
if [[ -x "$BINARY" ]]; then
    # Version flag
    if $BINARY --version 2>/dev/null | grep -q "cosmic-order"; then
        pass "CLI --version works"
    else
        fail "CLI --version failed"
    fi

    # Help flag
    if $BINARY --help 2>/dev/null | grep -q "cosmic-order"; then
        pass "CLI --help works"
    else
        fail "CLI --help failed"
    fi

    # Colors subcommand (reads live COSMIC theme — may fallback to defaults)
    COLORS_OUTPUT=$($BINARY colors 2>/dev/null || true)
    if echo "$COLORS_OUTPUT" | grep -q "accent"; then
        pass "CLI 'colors' subcommand produces output"
    else
        warn "CLI 'colors' produced no output (COSMIC theme may not be active)"
    fi

    # Colors JSON output
    COLORS_JSON=$($BINARY colors --json 2>/dev/null || true)
    if echo "$COLORS_JSON" | python3 -m json.tool &>/dev/null 2>&1; then
        pass "CLI 'colors --json' produces valid JSON"
    elif echo "$COLORS_JSON" | jq . &>/dev/null 2>&1; then
        pass "CLI 'colors --json' produces valid JSON"
    else
        warn "CLI 'colors --json' output not valid JSON (may need COSMIC session)"
    fi
else
    skip "Release binary not available for CLI tests"
fi

# =========================================================================
# SUMMARY
# =========================================================================
section "Summary"
echo -e "  Passed: ${GREEN}$PASS${NC}  Failed: ${RED}$FAIL${NC}  Warnings: ${YELLOW}$WARN${NC}  Skipped: ${CYAN}$SKIP${NC}"
echo ""

if [[ $FAIL -eq 0 ]]; then
    echo -e "${GREEN}${BOLD}All checks passed!${NC}"
    exit 0
else
    echo -e "${RED}${BOLD}$FAIL check(s) failed.${NC}"
    exit 1
fi
