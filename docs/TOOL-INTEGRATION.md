# Tool Integration Research

Analysis of OMARCHY-style tools and their integration potential with COSMIC ORDER
using the libcosmic framework.

## Executive Summary

| Tool | Integration Potential | Effort | Priority |
|------|----------------------|--------|----------|
| **Ghostty** | Theme sync, D-Bus launch | Low | High |
| **Neovim/LazyVim** | RPC theme sync, real-time control | Medium | High |
| **btop** | Theme generation, config management | Low | Medium |
| **Zellij** | KDL config generation | Low | Medium |
| **fzf/ripgrep/fd** | Color config generation | Low | Low |
| **lazygit** | Theme config generation | Low | Medium |

---

## 1. Ghostty Integration

### Current Capabilities

Ghostty is a supported terminal and tool-sync target. (The screensaver defaults
to Alacritty, which self-fullscreens — see
[SCREENSAVER-INTEGRATION.md](SCREENSAVER-INTEGRATION.md).)

**What's Available:**

- Simple text config file (`~/.config/ghostty/config`)
- Theme files in `~/.config/ghostty/themes/`
- D-Bus service: `com.mitchellh.ghostty`
- CLI flags mirror all config options
- Runtime config reload (`Ctrl+Shift+,`)
- Light/dark mode auto-switching: `theme = dark:Catppuccin,light:Catppuccin Latte`

**What's Missing:**

- No programmatic theme switching API
- No window embedding (libghostty-vt is parse-only currently)
- No plugin/extension system

### Integration Approach

```text
COSMIC Theme Change
    ↓
Read COSMIC accent/palette from cosmic-config
    ↓
Generate Ghostty theme file
    ↓
Write to ~/.config/ghostty/themes/cosmic-current.conf
    ↓
Update config: theme = cosmic-current
    ↓
Send OSC escape sequences to reload colors
```

**D-Bus Window Control:**

```bash
# Instant new window via D-Bus (faster than spawning)
ghostty +new-window
```

### libcosmic Benefits

- **Theme generation**: Extract COSMIC colors via `cosmic-theme` crate
- **Config management**: Store Ghostty preferences in `cosmic-config`
- **Launch integration**: Use D-Bus for instant window creation

### Future: libghostty

Mitchell Hashimoto is building full libghostty with:

- GTK widget for embedding
- Native UI framework support
- GPU rendering surfaces

Once available, could embed Ghostty windows directly in COSMIC ORDER.

---

## 2. Neovim/LazyVim Integration

### Current Capabilities

Neovim has the **best integration potential** due to its RPC API.

**What's Available:**

- Full MessagePack-RPC protocol for external control
- Bidirectional communication (call functions AND receive events)
- Real-time highlight group modification via `nvim_set_hl()`
- All options readable/writable via RPC
- LazyVim's colorscheme system is well-structured

**RPC Functions for Theming:**

```text
nvim_set_hl(namespace, group, attributes)  # Set highlight colors
nvim_get_hl(namespace, name)               # Query current colors
nvim_command(":colorscheme name")          # Switch colorscheme
nvim_exec_lua("vim.g.colors_name = ...")   # Direct Lua execution
```

### Integration Approach

#### Option A: File-based (Current)

```bash
# Current switch-theme.sh approach
cp themes/$THEME/colorscheme.lua ~/.config/nvim/lua/plugins/colorscheme.lua
# User must restart Neovim
```

#### Option B: RPC-based (Enhanced)

```rust
// Rust pseudo-code for COSMIC ORDER
let nvim = NvimClient::connect_socket("~/.cache/nvim/server.sock")?;

// Apply COSMIC colors directly
nvim.set_hl(0, "Normal", {"bg": cosmic_bg, "fg": cosmic_fg})?;
nvim.set_hl(0, "CursorLine", {"bg": cosmic_surface})?;
nvim.command(&format!(":colorscheme {}", theme_name))?;
// Changes apply instantly - no restart needed
```

### libcosmic Benefits

**Immediate:**

- Generate colorscheme.lua from COSMIC theme
- Store Neovim preferences in cosmic-config
- RPC client for real-time theme sync

**Future (Advanced):**

- Native COSMIC Neovim GUI using RPC + libcosmic rendering
- Would require significant development (6-12 months)
- Neovide/Neovim-Qt exist as reference implementations

### Rust Libraries

```toml
# For RPC communication
nvim-rs = "0.6"  # Async Neovim RPC client
# OR
neovim-lib = "0.6"  # Sync Neovim RPC client
```

---

## 3. btop Integration

### Current Capabilities

btop is a system monitor with theme support but no external APIs.

**What's Available:**

- Theme files in `~/.config/btop/themes/`
- CLI flags: `--config`, `--preset`, `--themes-dir`
- Key-value configuration format

**What's Missing:**

- No IPC/D-Bus interface
- No runtime theme reloading (must restart)
- No data export API
- No plugin system

### Integration Approach

```text
COSMIC Theme Change
    ↓
Read COSMIC colors
    ↓
Generate btop theme file using template:
    theme[main_bg] = "#1e1e2e"
    theme[main_fg] = "#cdd6f4"
    theme[cpu_start] = "#89b4fa"
    ...
    ↓
Write to ~/.config/btop/themes/cosmic.theme
    ↓
Update btop.conf: color_theme = "cosmic"
    ↓
(User must restart btop to see changes)
```

### Theme Template

```python
# btop theme generation (24 colors from COSMIC)
BTOP_TEMPLATE = """
theme[main_bg]="{background}"
theme[main_fg]="{foreground}"
theme[title]="{accent}"
theme[hi_fg]="{accent}"
theme[selected_bg]="{surface}"
theme[selected_fg]="{on_surface}"
theme[inactive_fg]="{subtle}"
theme[proc_misc]="{accent}"
theme[cpu_box]="{accent}"
theme[mem_box]="{secondary}"
theme[net_box]="{tertiary}"
theme[proc_box]="{accent}"
theme[div_line]="{surface}"
theme[cpu_start]="{accent}"
theme[cpu_mid]="{secondary}"
theme[cpu_end]="{destructive}"
theme[free_start]="{success}"
theme[free_mid]="{warning}"
theme[free_end]="{destructive}"
"""
```

### libcosmic Benefits

- Theme file generation from cosmic-theme
- Configuration management via cosmic-config
- Launch with custom config via subprocess

---

## 4. Zellij Integration

### Current Capabilities

Zellij is a terminal multiplexer with theme support.

**What's Available:**

- KDL configuration format
- Theme definitions in config
- Layout system

**What's Missing:**

- No runtime theme switching
- No IPC for external control

### Integration Approach

Generate full KDL theme from COSMIC colors:

```kdl
// Generated zellij theme
themes {
    cosmic {
        fg "#cdd6f4"
        bg "#1e1e2e"
        black "#45475a"
        red "#f38ba8"
        green "#a6e3a1"
        yellow "#f9e2af"
        blue "#89b4fa"
        magenta "#f5c2e7"
        cyan "#94e2d5"
        white "#bac2de"
        orange "#fab387"
    }
}
```

### libcosmic Benefits

- KDL generation from cosmic-theme
- Layout presets stored in cosmic-config

---

## 5. CLI Tools (fzf, ripgrep, fd, lazygit)

### fzf

**Color Configuration:**

```bash
export FZF_DEFAULT_OPTS="
  --color=bg+:#313244,bg:#1e1e2e,spinner:#f5e0dc
  --color=hl:#f38ba8,fg:#cdd6f4,header:#f38ba8
  --color=info:#cba6f7,pointer:#f5e0dc,marker:#f5e0dc
  --color=fg+:#cdd6f4,prompt:#cba6f7,hl+:#f38ba8
"
```

**Integration**: Generate shell export from COSMIC colors.

### ripgrep

**Color Configuration:**

```bash
export RIPGREP_CONFIG_PATH="$HOME/.config/ripgrep/config"
# In config file:
--colors=line:fg:yellow
--colors=match:fg:magenta
--colors=path:fg:green
```

**Integration**: Generate config from COSMIC colors.

### lazygit

**Theme Configuration** (`~/.config/lazygit/config.yml`):

```yaml
gui:
  theme:
    activeBorderColor:
      - "#89b4fa"  # COSMIC accent
      - bold
    inactiveBorderColor:
      - "#6c7086"  # COSMIC subtle
    selectedLineBgColor:
      - "#313244"  # COSMIC surface
```

**Integration**: Generate YAML from COSMIC colors.

---

## 6. OMARCHY Patterns to Adopt

### colors.toml Standard

OMARCHY uses a 24-color palette standard:

```toml
# Base colors
base00 = "#1e1e2e"  # Background
base01 = "#181825"  # Lighter background
base02 = "#313244"  # Selection background
base03 = "#45475a"  # Comments
base04 = "#585b70"  # Dark foreground
base05 = "#cdd6f4"  # Foreground
base06 = "#f5e0dc"  # Light foreground
base07 = "#b4befe"  # Light background

# Accent colors
base08 = "#f38ba8"  # Red
base09 = "#fab387"  # Orange
base0A = "#f9e2af"  # Yellow
base0B = "#a6e3a1"  # Green
base0C = "#94e2d5"  # Cyan
base0D = "#89b4fa"  # Blue
base0E = "#cba6f7"  # Purple
base0F = "#f2cdcd"  # Brown
```

**Benefit**: Single source of truth for all tool theme generation.

### Theme Hook System

OMARCHY uses hook scripts for extensibility:

```bash
~/.config/omarchy/hooks/theme-set.d/
├── 10-terminal.sh
├── 20-editor.sh
├── 30-apps.sh
└── 99-custom.sh
```

**COSMIC Equivalent:**

```bash
~/.config/cosmic-order/hooks/theme-set.d/
├── 10-ghostty.sh
├── 20-neovim.sh
├── 30-btop.sh
├── 40-zellij.sh
└── 50-cli-tools.sh
```

---

## 7. Architecture Recommendation

### Unified Theme Engine

```text
┌─────────────────────────────────────────────────────────┐
│                    COSMIC ORDER                          │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────┐   │
│  │              Theme Engine                        │   │
│  │  ┌─────────────┐  ┌─────────────────────────┐  │   │
│  │  │ cosmic-theme│  │    colors.toml          │  │   │
│  │  │   reader    │→ │  (24-color standard)    │  │   │
│  │  └─────────────┘  └─────────────────────────┘  │   │
│  │         ↓                    ↓                  │   │
│  │  ┌─────────────────────────────────────────┐   │   │
│  │  │           Theme Generators              │   │   │
│  │  │  ┌────────┐ ┌────────┐ ┌────────┐      │   │   │
│  │  │  │Ghostty │ │Neovim  │ │ btop   │ ...  │   │   │
│  │  │  └────────┘ └────────┘ └────────┘      │   │   │
│  │  └─────────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────┘   │
│                         ↓                               │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Application Layer                   │   │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐           │   │
│  │  │ Visuals │ │ToolSync │ │Screensav│           │   │
│  │  │  Page   │ │  Page   │ │  Page   │           │   │
│  │  └─────────┘ └─────────┘ └─────────┘           │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

### Data Flow

```text
User selects theme in COSMIC ORDER
    ↓
Read COSMIC theme via cosmic-theme crate
    ↓
Convert to colors.toml format (24 colors)
    ↓
Generate tool-specific configs:
    ├── ghostty.conf
    ├── colorscheme.lua (Neovim)
    ├── cosmic.theme (btop)
    ├── theme block (Zellij KDL)
    ├── FZF_DEFAULT_OPTS (shell export)
    └── lazygit config.yml
    ↓
Apply via:
    ├── File writes + reload signals
    ├── Neovim RPC (real-time)
    └── D-Bus where available
    ↓
Store state in cosmic-config
```

---

## 8. Implementation Phases

### Phase T1: Theme Foundation ✓ (Phase 6A)

- [x] Create colors.toml format specification
- [x] Build COSMIC → colors.toml converter (`src/colors.rs`)
- [x] Implement theme generator interface (`src/generators/`)

**Deliverable**: Unified color format for all tools

### Phase T2: Tool Generators ✅

- [x] Ghostty theme generator (`src/generators/ghostty.rs`)
- [x] Neovim colorscheme generator (`src/generators/nvim.rs`)
- [x] btop theme generator (`src/generators/btop.rs`)
- [x] Zellij theme generator (`src/generators/zellij.rs`)
- [x] fzf theme generator (`src/generators/fzf.rs`)
- [x] lazygit theme generator (`src/generators/lazygit.rs`)

**Deliverable**: Generated theme files on COSMIC theme change

### Phase T3: Real-time Sync ✅ (partial)

- [x] Ghostty SIGUSR2 reload signal
- [x] btop SIGUSR2 reload signal
- [x] Neovim `--remote-send` to unix sockets
- [ ] Zellij/fzf/lazygit have no live reload API

**Deliverable**: Instant theme propagation for Ghostty, btop, Neovim

### Phase T4: Hook System ✅

- [x] Implement hook directory (`~/.config/cosmic-order/hooks.d/`)
- [x] User-configurable tool enable/disable (per-tool togglers)
- [x] Custom script support with palette env vars

**Deliverable**: Extensible theme system

---

## 9. Dependencies

```toml
# Theme generators
cosmic-theme = { git = "https://github.com/pop-os/libcosmic.git" }
toml = "0.8"       # For colors.toml and tool-sync.toml
directories = "5"  # XDG path resolution (centralized in paths.rs)

# D-Bus (UPower, logind, systemd)
zbus = { version = "5", features = ["tokio"] }
```

Note: Zellij KDL and lazygit YAML are generated via string formatting,
not dedicated parser crates. Neovim uses `--remote-send` via subprocess,
not the RPC crate.

---

## 10. Benefits of libcosmic Integration

| Benefit | Description |
|---------|-------------|
| **Native theming** | Direct access to COSMIC color system |
| **Configuration persistence** | cosmic-config for all settings |
| **D-Bus integration** | System integration via zbus |
| **Wayland-native** | No X11 compatibility layer needed |
| **Consistent UX** | COSMIC design language throughout |

---

## Sources

- [Ghostty Documentation](https://ghostty.org/docs)
- [Neovim API Reference](https://neovim.io/doc/user/api.html)
- [btop GitHub](https://github.com/aristocratos/btop)
- [Zellij Documentation](https://zellij.dev/documentation/)
- [OMARCHY Theme Architecture](https://deepwiki.com/basecamp/omarchy/6-theming-and-customization)
- [libcosmic Book](https://pop-os.github.io/libcosmic-book/)
