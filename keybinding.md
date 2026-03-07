# Keybindings Reference

## Playhead Movement

| Key | Action | Description |
|-----|--------|-------------|
| `h` | Move Left | Move playhead 1 step left |
| `j` | Move Down | Move playhead 1 step down |
| `k` | Move Up | Move playhead 1 step up |
| `l` | Move Right | Move playhead 1 step right |
| `Option+h` / `Alt+h` | Leap Left | Jump 5 steps left |
| `Option+j` / `Alt+j` | Leap Down | Jump 5 steps down |
| `Option+k` / `Alt+k` | Leap Up | Jump 5 steps up |
| `Option+l` / `Alt+l` | Leap Right | Jump 5 steps right |

> **Note (macOS):** Option+hjkl produces Unicode characters (`˙`, `∆`, `˚`, `¬`) which are automatically mapped.

## Playhead Area Adjustment

| Key | Action | Description |
|-----|--------|-------------|
| `Shift+h` / `H` | Scale Left | Decrease area width by 1 |
| `Shift+j` / `J` | Scale Down | Decrease area height by 1 |
| `Shift+k` / `K` | Scale Up | Increase area height by 1 |
| `Shift+l` / `L` | Scale Right | Increase area width by 1 |
| `Shift+Option+h`* | Scale Left (Large) | Decrease area width by 8 |
| `Shift+Option+j`* | Scale Down (Large) | Decrease area height by 8 |
| `Shift+Option+k`* | Scale Up (Large) | Increase area height by 8 |
| `Shift+Option+l`* | Scale Right (Large) | Increase area width by 8 |

> **Note (macOS only):** Shift+Option combinations produce Unicode characters (`Ó`, `Ô`, ``, `Ò`) for large scale adjustments.

## Grid Configuration

| Key | Action | Description |
|-----|--------|-------------|
| `0` or `1` | 1×1 Grid | Single cell (no splits) |
| `2` | 2×1 Grid | 2 vertical splits |
| `3` | 3×1 Grid | 3 vertical splits |
| `4` | 4×1 Grid | 4 vertical splits |
| `5` | 4×2 Grid | 4×2 grid (8 cells) |
| `6` | 4×3 Grid | 4×3 grid (12 cells) |
| `7` | 4×4 Grid | 4×4 grid (16 cells) |

## Playback Control

| Key | Action | Description |
|-----|--------|-------------|
| `Space` | Toggle Play/Pause | Start or stop playback |
| `Ctrl+f` | Toggle Forward | Enable forward playback mode |
| `Ctrl+r` | Toggle Reverse | Enable reverse playback mode |
| `Ctrl+d` | Toggle Random | Enable random playback mode |
| `Ctrl+p` | Toggle Pendulum | Enable pendulum playback mode |

## Modes

| Key | Action | Description |
|-----|--------|-------------|
| `Ctrl+a` | Toggle Arpeggiator | Enable/disable arpeggiator mode |
| `Ctrl+u` | Toggle Accumulation | Enable/disable queue accumulation |
| `Ctrl+e` | Toggle Event Operator | Enable/disable event operators |
| `Ctrl+n` | Toggle Drain Queue | Enable/disable queue draining |
| `Ctrl+s` | Toggle Sweep | Enable/disable sweep mode |
| `Ctrl+l` | Toggle Dynamic Length | Enable/disable dynamic length mode |

## Tempo & Timing

| Key | Action | Description |
|-----|--------|-------------|
| `>` | Increase BPM | Speed up tempo |
| `<` | Decrease BPM | Slow down tempo |
| `}` | Increase Ratio | Increase timing ratio |
| `{` | Decrease Ratio | Decrease timing ratio |

## Scale & Musical Settings

| Key | Action | Description |
|-----|--------|-------------|
| `Shift++` / `Shift+Plus` | Cycle Scale Mode Up | Next musical scale |
| `Shift+_` / `Shift+Underscore` | Cycle Scale Mode Down | Previous musical scale |
| `=` / `Equal` | Increase Root Note | Next root note |
| `-` / `Minus` | Decrease Root Note | Previous root note |

## UI & Navigation

| Key | Action | Description |
|-----|--------|-------------|
| `Ctrl+b` | Show Menubar | Display/access menu |
| `Esc` | Toggle Input Mode | Switch between regex input and canvas |
| `q` | Quit | Exit application |

## Mouse Controls

| Action | Description |
|--------|-------------|
| **Click** | Set playhead position to clicked location |
| **Click+Drag** | Adjust playhead area size by dragging |

---

## Platform Notes

- **macOS**: Option key (⌥) is used for leap and large scale adjustments
- **Linux/Windows**: Alt key behaves similarly to Option on macOS for leap functions
- **Cross-platform**: Vim-style hjkl keys work consistently across all platforms

## Implementation Details

- Arrow keys have been removed in favor of vim-style navigation
- Platform-specific key combinations use `#[cfg(target_os = "macos")]` compilation flags
- Minimum playhead area size is enforced (1×1) to prevent overflow errors
