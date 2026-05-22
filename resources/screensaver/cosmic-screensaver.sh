#!/bin/bash
# cosmic-screensaver.sh - Terminal-based screensaver for COSMIC Desktop
# Inspired by Omarchy's screensaver, adapted for Pop!_OS
#
# Dependencies: terminaltexteffects (tte)
# Install: pipx install terminaltexteffects

set -uo pipefail

# Configuration
SCREENSAVER_DIR="${SCREENSAVER_DIR:-$HOME/.config/cosmic-screensaver}"
CONFIG_FILE="${SCREENSAVER_DIR}/config"
LOGO_FILE="${SCREENSAVER_DIR}/logo.txt"
DEFAULT_LOGO_FILE="$(dirname "$(readlink -f "$0")")/logo.txt"
FRAME_RATE="${FRAME_RATE:-60}"
INCLUDE_EFFECTS="${INCLUDE_EFFECTS:-}"
EXCLUDE_EFFECTS="${EXCLUDE_EFFECTS:-dev_worm}"
FADE_IN_EFFECT="${FADE_IN_EFFECT:-}"      # e.g., expand, slide, middleout
FADE_OUT_EFFECT="${FADE_OUT_EFFECT:-}"    # e.g., burn, crumble, scattered
SHOW_CLOCK="${SHOW_CLOCK:-false}"         # Show time between effects
CLOCK_DURATION="${CLOCK_DURATION:-3}"     # Seconds to display clock
CLOCK_FORMAT="${CLOCK_FORMAT:-%H:%M}"     # Time format (strftime)
CLOCK_FONT="${CLOCK_FONT:-}"              # figlet font (empty = default)
CURSOR_HIDE="${CURSOR_HIDE:-true}"        # Hide text cursor during screensaver
HIDE_MOUSE="${HIDE_MOUSE:-true}"          # Hide mouse pointer (disables mouse tracking)
DISMISS_ON_KEY="${DISMISS_ON_KEY:-true}"  # Keyboard input dismisses screensaver

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info() { echo -e "${CYAN}[INFO]${NC} $*"; }
log_success() { echo -e "${GREEN}[OK]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*" >&2; }

# Load configuration if exists
load_config() {
    if [[ -f "$CONFIG_FILE" ]]; then
        # shellcheck source=/dev/null
        source "$CONFIG_FILE"
    fi
}

# Apply the power-aware effect profile.
# The cosmic-order app writes power-state.env with EFFECT_PROFILE
# (full|standard|simple|minimal|skip), derived from battery level and the
# system power profile. Pick the matching per-profile effect list (set in the
# main config) as INCLUDE_EFFECTS; an empty list falls back to the normal
# include/exclude behaviour. If the app hasn't written power-state.env, leave
# effects unchanged.
apply_power_profile() {
    local power_env="${SCREENSAVER_DIR}/power-state.env"
    [[ -f "$power_env" ]] || return 0
    # shellcheck source=/dev/null
    source "$power_env"
    case "${EFFECT_PROFILE:-}" in
        skip)
            log_info "Critical battery (EFFECT_PROFILE=skip) — not starting screensaver"
            exit 0
            ;;
        full) [[ -n "${EFFECTS_FULL:-}" ]] && INCLUDE_EFFECTS="$EFFECTS_FULL" ;;
        standard) [[ -n "${EFFECTS_STANDARD:-}" ]] && INCLUDE_EFFECTS="$EFFECTS_STANDARD" ;;
        simple) [[ -n "${EFFECTS_SIMPLE:-}" ]] && INCLUDE_EFFECTS="$EFFECTS_SIMPLE" ;;
        minimal) [[ -n "${EFFECTS_MINIMAL:-}" ]] && INCLUDE_EFFECTS="$EFFECTS_MINIMAL" ;;
    esac
}

# Ensure config directory exists
ensure_config_dir() {
    if [[ ! -d "$SCREENSAVER_DIR" ]]; then
        mkdir -p "$SCREENSAVER_DIR"
        log_info "Created config directory: $SCREENSAVER_DIR"
    fi

    # Copy default logo if user doesn't have one
    if [[ ! -f "$LOGO_FILE" ]] && [[ -f "$DEFAULT_LOGO_FILE" ]]; then
        cp "$DEFAULT_LOGO_FILE" "$LOGO_FILE"
        log_info "Copied default logo to: $LOGO_FILE"
    fi
}

# Check for tte (TerminalTextEffects)
check_dependencies() {
    if ! command -v tte &>/dev/null; then
        log_error "TerminalTextEffects (tte) is not installed."
        log_info "Install with: pipx install terminaltexteffects"
        log_info "Or: pip install terminaltexteffects"
        exit 1
    fi
}

# Get available effects
# Extracts the effects list from TTE help output (the last {list} containing effect names)
get_effects() {
    tte -h 2>&1 | grep -oE '\{beams,[^}]+\}' | tr -d '{}' | tr ',' '\n'
}

# Prepare terminal for screensaver display
# Sets up black background, hides cursor, disables echo
# NOTE: Mouse tracking is NOT enabled here — it's enabled in the animation loop
# right before the read loop, to avoid buffering ESC sequences during clock display
prepare_terminal() {
    clear
    stty -echo  # Always disable echo (prevents mouse tracking sequences from showing)
    if [[ "$CURSOR_HIDE" != "false" ]]; then
        tput civis  # Hide text cursor
    fi
    # Set background to black
    printf '\033]11;rgb:00/00/00\007'
}

# Restore terminal to normal state
restore_terminal() {
    printf '\e[?1003l' 2>/dev/null  # Disable mouse tracking
    tput cnorm 2>/dev/null  # Show cursor
    tput reset 2>/dev/null
    stty echo 2>/dev/null   # Re-enable echo
}

# Clean exit handler
# shellcheck disable=SC2317
exit_screensaver() {
    # Kill any running tte processes first
    pkill -P $$ -x tte 2>/dev/null
    sleep 0.1

    # Run fade-out effect if configured
    if [[ -n "${FADE_OUT_EFFECT:-}" ]] && [[ -n "${CURRENT_LOGO_FILE:-}" ]]; then
        run_fade_out "$CURRENT_LOGO_FILE"
    fi

    # Restore terminal state
    restore_terminal

    exit 0
}

# Build effect filter arguments for TTE
# INCLUDE_EFFECTS takes precedence over EXCLUDE_EFFECTS (they're mutually exclusive in TTE)
# Converts comma-separated list to space-separated arguments
build_effect_args() {
    if [[ -n "${INCLUDE_EFFECTS:-}" ]]; then
        echo "--include-effects ${INCLUDE_EFFECTS//,/ }"
    elif [[ -n "${EXCLUDE_EFFECTS:-}" ]]; then
        echo "--exclude-effects ${EXCLUDE_EFFECTS//,/ }"
    fi
}

# Run fade-in effect (logo appears)
# Good effects: expand, slide, middleout, pour, waves, decrypt
run_fade_in() {
    local logo_file="$1"
    local effect="${FADE_IN_EFFECT:-}"

    [[ -z "$effect" ]] && return 0

    clear
    tte -i "$logo_file" \
        --frame-rate "$FRAME_RATE" \
        --canvas-width 0 \
        --canvas-height 0 \
        --anchor-canvas c \
        --anchor-text c \
        "$effect" 2>/dev/null

    # Brief pause to appreciate the revealed logo
    sleep 0.5
}

# Run fade-out effect (logo disappears)
# Good effects: burn, crumble, scattered, blackhole, fireworks
run_fade_out() {
    local logo_file="$1"
    local effect="${FADE_OUT_EFFECT:-}"

    [[ -z "$effect" ]] && return 0

    clear
    tte -i "$logo_file" \
        --frame-rate "$FRAME_RATE" \
        --canvas-width 0 \
        --canvas-height 0 \
        --anchor-canvas c \
        --anchor-text c \
        "$effect" 2>/dev/null
}

# Display clock between effects
display_clock() {
    [[ "$SHOW_CLOCK" != "true" ]] && return 0

    local duration="${CLOCK_DURATION:-3}"
    local format="${CLOCK_FORMAT:-%H:%M}"
    local font="${CLOCK_FONT:-}"
    local time_str
    time_str=$(date +"$format")

    clear

    # Get terminal dimensions for centering
    local term_lines term_cols
    term_lines=$(tput lines)
    term_cols=$(tput cols)

    # Try figlet first, then toilet, then fall back to simple text
    if command -v figlet &>/dev/null; then
        local figlet_output font_arg=""
        [[ -n "$font" ]] && font_arg="-f $font"
        # shellcheck disable=SC2086
        figlet_output=$(figlet $font_arg "$time_str" 2>/dev/null)
        if [[ -n "$figlet_output" ]]; then
            # Center the figlet output
            local fig_lines fig_width
            fig_lines=$(echo "$figlet_output" | wc -l)
            fig_width=$(echo "$figlet_output" | head -1 | wc -c)
            local start_row=$(( (term_lines - fig_lines) / 2 ))
            local start_col=$(( (term_cols - fig_width) / 2 ))
            [[ $start_row -lt 0 ]] && start_row=0
            [[ $start_col -lt 0 ]] && start_col=0

            tput cup "$start_row" 0
            echo "$figlet_output" | while IFS= read -r line; do
                printf "%*s%s\n" "$start_col" "" "$line"
            done
        fi
    elif command -v toilet &>/dev/null; then
        local toilet_output font_arg=""
        [[ -n "$font" ]] && font_arg="-f $font"
        # shellcheck disable=SC2086
        toilet_output=$(toilet $font_arg "$time_str" 2>/dev/null)
        if [[ -n "$toilet_output" ]]; then
            local toi_lines toi_width
            toi_lines=$(echo "$toilet_output" | wc -l)
            toi_width=$(echo "$toilet_output" | head -1 | wc -c)
            local start_row=$(( (term_lines - toi_lines) / 2 ))
            local start_col=$(( (term_cols - toi_width) / 2 ))
            [[ $start_row -lt 0 ]] && start_row=0
            [[ $start_col -lt 0 ]] && start_col=0

            tput cup "$start_row" 0
            echo "$toilet_output" | while IFS= read -r line; do
                printf "%*s%s\n" "$start_col" "" "$line"
            done
        fi
    else
        # Fallback: simple centered large text
        local center_row=$(( term_lines / 2 ))
        local center_col=$(( (term_cols - ${#time_str}) / 2 ))
        tput cup "$center_row" "$center_col"
        # Bold white text
        printf '\033[1;37m%s\033[0m' "$time_str"
    fi

    sleep "$duration"
}

# Inject input via ydotool (fullscreen toggle / mouse parking).
# ydotool needs /dev/uinput access (input group or root). Never prompt
# interactively — a bare `sg`/`sudo` password prompt would freeze the
# screensaver — so only take a path that won't ask for a password.
inject_input() {
    local user
    user="$(id -un)"
    if id -nG 2>/dev/null | grep -qw input; then
        ydotool "$@"
    elif getent group input 2>/dev/null | awk -F: '{print $4}' | grep -qw "$user"; then
        sg input -c "ydotool $*"
    elif sudo -n true 2>/dev/null; then
        sudo -n ydotool "$@"
    else
        return 1
    fi
}

# Run the screensaver
run_screensaver() {
    local logo_file="${1:-$LOGO_FILE}"

    # Verify logo file exists
    if [[ ! -f "$logo_file" ]]; then
        log_error "Logo file not found: $logo_file"
        exit 1
    fi

    # Track logo file for fade-out on exit
    CURRENT_LOGO_FILE="$logo_file"

    # Set up exit handlers
    trap exit_screensaver SIGINT SIGTERM SIGHUP SIGQUIT EXIT

    # Toggle fullscreen via COSMIC compositor keybind (Super+F)
    # ydotool injects at kernel level (/dev/uinput) so compositor sees real input
    # wtype sends to Wayland surface directly, bypassing compositor keybinds
    if command -v ydotool &>/dev/null; then
        sleep 2.0
        # Toggle fullscreen via Super+F — skipped when the terminal already
        # self-fullscreens (Alacritty sets NO_FULLSCREEN_TOGGLE=1), since the
        # toggle would otherwise un-fullscreen the window.
        if [[ "${NO_FULLSCREEN_TOGGLE:-}" != "1" ]]; then
            # Use modifier+key syntax (ydotool 0.1.x)
            inject_input key --delay 0 super+f 2>/dev/null || true
        fi
        sleep 0.5
        # Park mouse pointer in bottom-right corner (invisible on black background)
        # Must happen BEFORE mouse tracking is enabled, or the movement triggers dismiss
        # ydotool 0.1.x uses relative movement: mousemove <dx> <dy>
        if [[ "$HIDE_MOUSE" != "false" ]]; then
            inject_input mousemove 19999 19999 2>/dev/null || true
            sleep 0.2
            # Drain any mouse tracking events that leaked from the move
            while read -r -s -n1 -t 0.05 2>/dev/null; do :; done
        fi
        # Drain any leaked keystrokes from ydotool
        while read -r -s -n1 -t 0.1 2>/dev/null; do :; done
    fi

    # Prepare terminal
    prepare_terminal

    # Run fade-in effect if configured
    run_fade_in "$logo_file"

    # Build effect filter arguments once (include or exclude)
    local effect_args
    effect_args=$(build_effect_args)

    # Run screensaver loop using TTE's native random effect selection
    while true; do
        # Show clock between effects if enabled
        display_clock

        clear
        # Reapply cursor hide after clear (clear resets terminal modes)
        if [[ "$CURSOR_HIDE" != "false" ]]; then
            tput civis 2>/dev/null
        fi

        # shellcheck disable=SC2086
        tte -i "$logo_file" \
            --random-effect \
            $effect_args \
            --frame-rate "$FRAME_RATE" \
            --canvas-width 0 \
            --canvas-height 0 \
            --anchor-canvas c \
            --anchor-text c \
            </dev/null 2>/dev/null &

        local tte_pid=$!

        # Check if TTE is still alive after a brief moment
        sleep 0.1
        if ! kill -0 "$tte_pid" 2>/dev/null; then
            wait "$tte_pid" 2>/dev/null
            continue
        fi

        # Drain any buffered stdin before enabling mouse tracking
        while read -r -s -n1 -t 0.01 2>/dev/null; do :; done

        # Enable mouse tracking for dismiss detection
        # Must be AFTER stdin drain to avoid immediately triggering exit
        printf '\e[?1003h'

        # Wait for effect to complete or user input
        # Mouse tracking ESC sequences from mouse movement always dismiss
        # Keyboard dismiss is conditional on DISMISS_ON_KEY
        while kill -0 "$tte_pid" 2>/dev/null; do
            if read -r -s -n1 -t 0.5 key 2>/dev/null; then
                if [[ "$key" == $'\e' ]]; then
                    # ESC sequence (mouse event or Escape key) — always dismiss
                    read -r -s -n5 -t 0.01 2>/dev/null || true
                    exit_screensaver
                elif [[ "$DISMISS_ON_KEY" != "false" ]]; then
                    # Regular key — only dismiss if enabled
                    exit_screensaver
                fi
            fi
        done

        wait "$tte_pid" 2>/dev/null
    done
}

# Run a single effect (for testing)
run_single_effect() {
    local effect="${1:-}"
    local logo_file="${2:-$LOGO_FILE}"

    if [[ ! -f "$logo_file" ]]; then
        log_error "Logo file not found: $logo_file"
        exit 1
    fi

    trap exit_screensaver SIGINT SIGTERM SIGHUP SIGQUIT EXIT

    prepare_terminal

    # Build effect filter arguments (include or exclude)
    local effect_args
    effect_args=$(build_effect_args)

    if [[ -n "$effect" ]]; then
        # Run specific effect
        tte -i "$logo_file" \
            --frame-rate "$FRAME_RATE" \
            --canvas-width 0 \
            --canvas-height 0 \
            --anchor-canvas c \
            --anchor-text c \
            "$effect" </dev/null
    else
        # Use TTE's random effect selection
        # shellcheck disable=SC2086
        tte -i "$logo_file" \
            --random-effect \
            $effect_args \
            --frame-rate "$FRAME_RATE" \
            --canvas-width 0 \
            --canvas-height 0 \
            --anchor-canvas c \
            --anchor-text c </dev/null
    fi

    exit_screensaver
}

# List available effects
list_effects() {
    log_info "Available TTE effects:"
    echo
    get_effects | sort | column
}

# Show usage
usage() {
    cat << EOF
Usage: $(basename "$0") [COMMAND] [OPTIONS]

Terminal-based screensaver for COSMIC Desktop using TerminalTextEffects.

Commands:
    run             Run the screensaver (default)
    test [EFFECT]   Test a specific effect (random if not specified)
    effects         List available effects
    setup           Set up config directory and copy default logo
    help            Show this help message

Options:
    -l, --logo FILE     Use a specific logo file
    -f, --fps NUMBER    Set frame rate (default: 60)

Environment Variables:
    SCREENSAVER_DIR     Config directory (default: ~/.config/cosmic-screensaver)
    FRAME_RATE          Animation frame rate (default: 60)
    INCLUDE_EFFECTS     Comma-separated effects to include (takes precedence)
    EXCLUDE_EFFECTS     Comma-separated effects to exclude
    FADE_IN_EFFECT      Effect for startup transition (e.g., expand, slide)
    FADE_OUT_EFFECT     Effect for exit transition (e.g., burn, crumble)
    SHOW_CLOCK          Show time between effects (true/false)
    CLOCK_DURATION      Seconds to display clock (default: 3)
    CLOCK_FORMAT        Time format, strftime syntax (default: %H:%M)
    CLOCK_FONT          figlet/toilet font (empty = default)

Examples:
    $(basename "$0")                    # Run screensaver
    $(basename "$0") test blackhole     # Test blackhole effect
    $(basename "$0") effects            # List all effects
    $(basename "$0") -l ~/my-logo.txt   # Use custom logo

Press any key to exit the screensaver.

EOF
}

# Main
main() {
    load_config

    # Parse global options first
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -l|--logo)
                LOGO_FILE="$2"
                shift 2
                ;;
            -f|--fps)
                FRAME_RATE="$2"
                shift 2
                ;;
            -*)
                if [[ "$1" == "-h" ]] || [[ "$1" == "--help" ]]; then
                    usage
                    exit 0
                fi
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
            *)
                break
                ;;
        esac
    done

    local command="${1:-run}"
    shift 2>/dev/null || true

    case "$command" in
        run)
            check_dependencies
            ensure_config_dir
            apply_power_profile
            run_screensaver "$LOGO_FILE"
            ;;
        test)
            check_dependencies
            ensure_config_dir
            local effect="${1:-}"
            if [[ -z "$effect" ]]; then
                # Use TTE's random selection for consistency
                log_info "Testing random effect..."
            fi
            run_single_effect "$effect"
            ;;
        effects|list)
            check_dependencies
            list_effects
            ;;
        setup)
            ensure_config_dir
            check_dependencies
            log_success "Setup complete!"
            log_info "Config directory: $SCREENSAVER_DIR"
            log_info "Logo file: $LOGO_FILE"
            ;;
        help)
            usage
            ;;
        *)
            log_error "Unknown command: $command"
            usage
            exit 1
            ;;
    esac
}

main "$@"
