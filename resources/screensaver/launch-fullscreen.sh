#!/bin/bash
# launch-fullscreen.sh - Launch screensaver in fullscreen Ghostty windows
# Launches a fullscreen Ghostty terminal on each connected monitor
# For COSMIC Desktop on Pop!_OS

set -uo pipefail

# Ensure PATH includes user local bin (for tte/pipx installed tools)
export PATH="$HOME/.local/bin:$PATH"

SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
CONFIG_DIR="${HOME}/.config/cosmic-screensaver"
CONFIG_FILE="${CONFIG_DIR}/config"
SCREENSAVER_SCRIPT="${SCRIPT_DIR}/cosmic-screensaver.sh"
GHOSTTY_SCREENSAVER_CONFIG="${CONFIG_DIR}/ghostty-screensaver.conf"
ALACRITTY_SCREENSAVER_CONFIG="${CONFIG_DIR}/alacritty-screensaver.toml"
PID_FILE="${CONFIG_DIR}/screensaver.pid"
TOGGLE_FILE="${CONFIG_DIR}/screensaver-disabled"

# Default terminal
DEFAULT_TERMINAL="alacritty"

# Default cursor/mouse hiding
CURSOR_HIDE="${CURSOR_HIDE:-true}"
HIDE_MOUSE="${HIDE_MOUSE:-true}"

# COSMIC compositor config paths
COSMIC_COMP_DIR="${HOME}/.config/cosmic/com.system76.CosmicComp/v1"
SAVED_FOCUS_FOLLOWS=""
SAVED_AUTOTILE=""

# Load terminal preference from config
load_terminal_config() {
    TERMINAL="${DEFAULT_TERMINAL}"
    if [[ -f "$CONFIG_FILE" ]]; then
        local config_terminal
        config_terminal=$(grep -E "^TERMINAL=" "$CONFIG_FILE" 2>/dev/null | cut -d= -f2 | tr -d '"')
        if [[ -n "$config_terminal" ]]; then
            TERMINAL="$config_terminal"
        fi
        local config_cursor_hide
        config_cursor_hide=$(grep -E "^CURSOR_HIDE=" "$CONFIG_FILE" 2>/dev/null | cut -d= -f2 | tr -d '"')
        if [[ -n "$config_cursor_hide" ]]; then
            CURSOR_HIDE="$config_cursor_hide"
        fi
        local config_hide_mouse
        config_hide_mouse=$(grep -E "^HIDE_MOUSE=" "$CONFIG_FILE" 2>/dev/null | cut -d= -f2 | tr -d '"')
        if [[ -n "$config_hide_mouse" ]]; then
            HIDE_MOUSE="$config_hide_mouse"
        fi
    fi
}

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() { echo -e "${CYAN}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# Save and disable COSMIC compositor settings that interfere with screensaver
# Focus-follows-cursor steals focus from Ghostty; autotile prevents fullscreen
disable_compositor_interference() {
    if [[ -d "$COSMIC_COMP_DIR" ]]; then
        # Save current values
        SAVED_FOCUS_FOLLOWS=$(cat "$COSMIC_COMP_DIR/focus_follows_cursor" 2>/dev/null)
        SAVED_AUTOTILE=$(cat "$COSMIC_COMP_DIR/autotile" 2>/dev/null)

        # Disable if currently enabled (COSMIC watches for file changes via inotify)
        if [[ "$SAVED_FOCUS_FOLLOWS" == "true" ]]; then
            echo "false" > "$COSMIC_COMP_DIR/focus_follows_cursor"
        fi
        if [[ "$SAVED_AUTOTILE" == "true" ]]; then
            echo "false" > "$COSMIC_COMP_DIR/autotile"
        fi
    fi
}

# Restore COSMIC compositor settings to their original values
restore_compositor_settings() {
    if [[ -d "$COSMIC_COMP_DIR" ]]; then
        if [[ -n "$SAVED_FOCUS_FOLLOWS" ]]; then
            echo "$SAVED_FOCUS_FOLLOWS" > "$COSMIC_COMP_DIR/focus_follows_cursor"
        fi
        if [[ -n "$SAVED_AUTOTILE" ]]; then
            echo "$SAVED_AUTOTILE" > "$COSMIC_COMP_DIR/autotile"
        fi
    fi
}

# Check if screensaver is disabled via toggle
is_disabled() {
    [[ -f "$TOGGLE_FILE" ]]
}

# Check if screensaver is already running
is_running() {
    if [[ -f "$PID_FILE" ]]; then
        local pid
        pid=$(cat "$PID_FILE" 2>/dev/null)
        if [[ -n "$pid" ]] && kill -0 "$pid" 2>/dev/null; then
            return 0
        fi
        # Stale PID file
        rm -f "$PID_FILE"
    fi
    # Check for any running screensaver instances (any supported terminal)
    pgrep -f "alacritty.*cosmic-screensaver" &>/dev/null && return 0
    pgrep -f "ghostty.*cosmic-screensaver" &>/dev/null && return 0
    pgrep -f "cosmic-term.*cosmic-screensaver" &>/dev/null && return 0
    return 1
}

# Check if cosmic-randr is available
check_cosmic_randr() {
    if ! command -v cosmic-randr &>/dev/null; then
        log_warn "cosmic-randr not found, assuming single monitor"
        return 1
    fi
    return 0
}

# Get list of enabled monitors
get_monitors() {
    if ! check_cosmic_randr; then
        echo "default"
        return
    fi
    cosmic-randr list 2>/dev/null | grep -E '^\S+.*\(enabled\)' | awk '{print $1}'
}

# Get monitor count
get_monitor_count() {
    get_monitors | wc -l
}

# Create Ghostty config for screensaver
create_ghostty_config() {
    mkdir -p "$CONFIG_DIR"

    # Build config with conditional mouse hiding
    local mouse_hide_line=""
    if [[ "$HIDE_MOUSE" != "false" ]]; then
        mouse_hide_line="mouse-hide-while-typing = true"
    else
        mouse_hide_line="mouse-hide-while-typing = false"
    fi

    # Build cursor hide config (black cursor on black background = invisible)
    local cursor_hide_lines=""
    if [[ "$CURSOR_HIDE" != "false" ]]; then
        cursor_hide_lines="cursor-style = block
cursor-style-blink = false
cursor-color = 000000
cursor-text = 000000"
    fi

    cat > "$GHOSTTY_SCREENSAVER_CONFIG" << EOF
# Ghostty Screensaver Configuration
# Minimal config for fullscreen screensaver display

# Black background, no transparency
background = 000000
background-opacity = 1.0

# No window decorations
window-decoration = none
gtk-titlebar = false

# No padding for maximum display area
window-padding-x = 0
window-padding-y = 0

# Mouse pointer visibility
${mouse_hide_line}

# Invisible cursor (black on black background)
${cursor_hide_lines}

# Font size for ASCII art visibility
font-size = 16

# No scrollbar
scrollback-limit = 0

# Disable features that might interfere
confirm-close-surface = false
quit-after-last-window-closed = true

# Start as separate instance (not merged with main Ghostty)
gtk-single-instance = false
EOF
}

# Create Alacritty config for screensaver
# Minimal config (black bg, no padding, hidden cursor) that starts fullscreen.
# startup_mode = "Fullscreen" handles fullscreen at the terminal level, avoiding
# both the cosmic-term fullscreen bug and the fragile ydotool Super+F toggle.
create_alacritty_config() {
    mkdir -p "$CONFIG_DIR"

    # Hidden text cursor: black on black background
    local cursor_block=""
    if [[ "$CURSOR_HIDE" != "false" ]]; then
        cursor_block='
[colors.cursor]
cursor = "#000000"
text = "#000000"'
    fi

    cat > "$ALACRITTY_SCREENSAVER_CONFIG" << EOF
# Alacritty Screensaver Configuration
# Minimal config for fullscreen screensaver display.

[window]
startup_mode = "Fullscreen"
decorations = "None"
dynamic_padding = false

[window.padding]
x = 0
y = 0

[font]
size = 16.0

[colors.primary]
background = "#000000"
foreground = "#ffffff"
${cursor_block}

[mouse]
hide_when_typing = ${HIDE_MOUSE}

[scrolling]
history = 0
EOF
}

# Kill all screensaver instances and restore compositor settings
kill_screensaver() {
    restore_compositor_settings
    # Kill by PID file
    if [[ -f "$PID_FILE" ]]; then
        local pid
        pid=$(cat "$PID_FILE" 2>/dev/null)
        if [[ -n "$pid" ]]; then
            kill "$pid" 2>/dev/null
        fi
        rm -f "$PID_FILE"
    fi

    # Kill any terminal screensaver processes (any supported terminal)
    pkill -f "alacritty.*cosmic-screensaver" 2>/dev/null
    pkill -f "ghostty.*cosmic-screensaver" 2>/dev/null
    pkill -f "cosmic-term.*cosmic-screensaver" 2>/dev/null
    pkill -f "cosmic-screensaver.sh" 2>/dev/null
}

# Launch screensaver using Ghostty
launch_with_ghostty() {
    local monitor_count="$1"

    # Ensure config exists
    if [[ ! -f "$GHOSTTY_SCREENSAVER_CONFIG" ]]; then
        create_ghostty_config
    fi

    # Check for ghostty
    if ! command -v ghostty &>/dev/null; then
        log_error "Ghostty is required but not installed"
        log_info "Install Ghostty from: https://ghostty.org"
        log_info "Or switch to cosmic-term: screensaver-ctl set terminal cosmic-term"
        return 1
    fi

    log_info "Launching screensaver with ghostty on $monitor_count monitor(s)..."

    # Launch Ghostty for screensaver
    # --gtk-single-instance=false ensures we don't join existing Ghostty window
    ghostty \
        --gtk-single-instance=false \
        --config-file="$GHOSTTY_SCREENSAVER_CONFIG" \
        --class=cosmic-screensaver \
        -e "$SCREENSAVER_SCRIPT" run &

    local main_pid=$!

    # If multiple monitors, launch additional instances
    if [[ "$monitor_count" -gt 1 ]]; then
        sleep 0.5  # Brief delay to let first window establish

        local i=1
        while [[ $i -lt $monitor_count ]]; do
            ghostty \
                --gtk-single-instance=false \
                --config-file="$GHOSTTY_SCREENSAVER_CONFIG" \
                --class=cosmic-screensaver \
                -e "$SCREENSAVER_SCRIPT" run &
            ((i++))
            sleep 0.5
        done
    fi

    # Wait for the main process
    wait "$main_pid" 2>/dev/null
}

# Launch screensaver using cosmic-term
launch_with_cosmic_term() {
    local monitor_count="$1"

    # Check for cosmic-term
    if ! command -v cosmic-term &>/dev/null; then
        log_error "cosmic-term is required but not installed"
        log_info "cosmic-term is included with COSMIC Desktop"
        log_info "Or switch to ghostty: screensaver-ctl set terminal ghostty"
        return 1
    fi

    log_info "Launching screensaver with cosmic-term on $monitor_count monitor(s)..."

    # Launch cosmic-term for screensaver
    # cosmic-term doesn't support --config-file or --class, so options are limited
    cosmic-term -e "$SCREENSAVER_SCRIPT" run &

    local main_pid=$!

    # If multiple monitors, launch additional instances
    if [[ "$monitor_count" -gt 1 ]]; then
        sleep 0.5  # Brief delay to let first window establish

        local i=1
        while [[ $i -lt $monitor_count ]]; do
            cosmic-term -e "$SCREENSAVER_SCRIPT" run &
            ((i++))
            sleep 0.5
        done
    fi

    # Wait for the main process
    wait "$main_pid" 2>/dev/null
}

# Launch screensaver using Alacritty (default)
# Alacritty self-fullscreens via its config (startup_mode=Fullscreen), so the
# driver is told to SKIP the ydotool Super+F toggle (NO_FULLSCREEN_TOGGLE=1) —
# otherwise the toggle would un-fullscreen the already-fullscreen window.
launch_with_alacritty() {
    local monitor_count="$1"

    # Ensure config exists (regenerated each launch to pick up HIDE_MOUSE/CURSOR_HIDE)
    create_alacritty_config

    # Check for alacritty
    if ! command -v alacritty &>/dev/null; then
        log_error "Alacritty is required but not installed"
        log_info "Install with: sudo apt install alacritty"
        log_info "Or switch terminal: screensaver-ctl set terminal cosmic-term"
        return 1
    fi

    log_info "Launching screensaver with alacritty on $monitor_count monitor(s)..."

    # --class sets the app id so is_running/kill can find it.
    # NO_FULLSCREEN_TOGGLE propagates to the driver via Alacritty's child env.
    NO_FULLSCREEN_TOGGLE=1 alacritty \
        --config-file "$ALACRITTY_SCREENSAVER_CONFIG" \
        --class cosmic-screensaver,cosmic-screensaver \
        -e "$SCREENSAVER_SCRIPT" run &

    local main_pid=$!

    # If multiple monitors, launch additional instances
    if [[ "$monitor_count" -gt 1 ]]; then
        sleep 0.5  # Brief delay to let first window establish

        local i=1
        while [[ $i -lt $monitor_count ]]; do
            NO_FULLSCREEN_TOGGLE=1 alacritty \
                --config-file "$ALACRITTY_SCREENSAVER_CONFIG" \
                --class cosmic-screensaver,cosmic-screensaver \
                -e "$SCREENSAVER_SCRIPT" run &
            ((i++))
            sleep 0.5
        done
    fi

    # Wait for the main process
    wait "$main_pid" 2>/dev/null
}

# Launch screensaver on all monitors
launch_screensaver() {
    local force=""
    local skip_compositor=false

    # Parse arguments
    for arg in "$@"; do
        case "$arg" in
            force) force="force" ;;
            --skip-compositor) skip_compositor=true ;;
        esac
    done

    # Check toggle (unless forced)
    if [[ "$force" != "force" ]] && is_disabled; then
        log_info "Screensaver is disabled. Use 'screensaver-ctl toggle' to enable."
        exit 0
    fi

    # Kill any previous instance and wait for cleanup
    if is_running; then
        kill_screensaver
        sleep 2
    fi

    # Load terminal preference
    load_terminal_config

    # Get monitor count
    local monitor_count
    monitor_count=$(get_monitor_count)

    if [[ "$monitor_count" -eq 0 ]]; then
        log_error "No monitors detected"
        exit 1
    fi

    # Store our PID
    echo $$ > "$PID_FILE"

    # Regenerate terminal config (picks up HIDE_MOUSE changes)
    case "$TERMINAL" in
        cosmic-term) ;;  # cosmic-term has no config file
        ghostty) create_ghostty_config ;;
        alacritty|*) create_alacritty_config ;;
    esac

    # Temporarily disable compositor settings that interfere with fullscreen
    # (skipped when cosmic-order handles this via cosmic-config API)
    if [[ "$skip_compositor" != "true" ]]; then
        disable_compositor_interference
        sleep 0.3  # Allow compositor to process inotify config changes
    fi

    # Launch with configured terminal
    case "$TERMINAL" in
        cosmic-term)
            launch_with_cosmic_term "$monitor_count"
            ;;
        ghostty)
            launch_with_ghostty "$monitor_count"
            ;;
        alacritty|*)
            launch_with_alacritty "$monitor_count"
            ;;
    esac

    # Restore compositor settings before cleanup
    # (skipped when cosmic-order handles this via cosmic-config API)
    if [[ "$skip_compositor" != "true" ]]; then
        restore_compositor_settings
    fi

    # Cleanup — only remove PID file; don't pkill here as it races with new instances
    rm -f "$PID_FILE"
}

# Toggle screensaver enabled/disabled
toggle_screensaver() {
    if is_disabled; then
        rm -f "$TOGGLE_FILE"
        log_success "Screensaver enabled"
        notify-send "Screensaver" "Screensaver enabled" 2>/dev/null || true
    else
        mkdir -p "$(dirname "$TOGGLE_FILE")"
        touch "$TOGGLE_FILE"
        log_success "Screensaver disabled"
        notify-send "Screensaver" "Screensaver disabled" 2>/dev/null || true
    fi
}

# Show status
show_status() {
    load_terminal_config

    echo
    echo "Screensaver Fullscreen Status"
    echo "──────────────────────────────"

    if is_running; then
        echo -e "Running:   ${GREEN}yes${NC}"
    else
        echo -e "Running:   ${RED}no${NC}"
    fi

    if is_disabled; then
        echo -e "Enabled:   ${RED}no${NC} (toggle file exists)"
    else
        echo -e "Enabled:   ${GREEN}yes${NC}"
    fi

    echo -e "Monitors:  $(get_monitor_count)"
    echo -e "Terminal:  $TERMINAL"
    if [[ "$TERMINAL" == "ghostty" ]]; then
        echo -e "Config:    $GHOSTTY_SCREENSAVER_CONFIG"
    fi
    echo
}

# Usage
usage() {
    cat << EOF
Usage: $(basename "$0") [COMMAND]

Launch screensaver in fullscreen Ghostty windows.

Commands:
    launch [force] [--skip-compositor]
                     Launch the screensaver (default)
    kill             Stop running screensaver
    toggle           Toggle screensaver enabled/disabled
    status           Show current status
    config           Regenerate Ghostty config
    help             Show this help

Options:
    --skip-compositor  Skip compositor settings management (used when
                       cosmic-order handles this via cosmic-config API)

Environment:
    CONFIG_DIR       Config directory (default: ~/.config/cosmic-screensaver)

EOF
}

# Main
main() {
    local cmd="${1:-launch}"
    shift 2>/dev/null || true

    case "$cmd" in
        launch|run|start)
            launch_screensaver "$@"
            ;;
        kill|stop)
            kill_screensaver
            log_success "Screensaver stopped"
            ;;
        toggle)
            toggle_screensaver
            ;;
        status)
            show_status
            ;;
        config)
            create_ghostty_config
            log_success "Config created: $GHOSTTY_SCREENSAVER_CONFIG"
            ;;
        help|-h|--help)
            usage
            ;;
        *)
            log_error "Unknown command: $cmd"
            usage
            exit 1
            ;;
    esac
}

main "$@"
