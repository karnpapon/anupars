# anupars

anupars (อนุภา(ส), meaning "tiny following light" in Thai), a Rust-based reimagining of [`anu`](https://github.com/karnpapon/anu) a musical sequencer driven by regular expressions, designed to operate on resource-constrained devices, and performance-oriented.

> [!WARNING]
> This project is a **work in progress**. Features and APIs are subject to change anytime.

<img src="ss.png" />

# Features

- **MIDI Out Selector**
  - Choose from available MIDI output devices for flexible routing to synths, DAWs, or hardware.

- **MIDI Out Clock Features**
  - Implements standard MIDI clock transport messages for external device:
    - `Start` Sends MIDI Start (`0xFA`) to begin playback from the start.
    - `Stop` Sends MIDI Stop (`0xFC`) to halt playback.
    - `Tick` Sends MIDI Clock (`0xF8`) for timing synchronization (24 per quarter note ([PPQN](https://en.wikipedia.org/wiki/MIDI_beat_clock))).
    - `Continue` Sends MIDI Continue (`0xFB`) to resume playback from the current position.
    - `Song Position Pointer` (SPP) Sends MIDI Song Position Pointer (`0xF2`) to set the playback position in beats.

- **Keyboard MIDI Layout**
  - The on-screen keyboard uses a spatial layout similar to Laurie Spiegel's [Music Mouse](https://en.wikipedia.org/wiki/Music_Mouse), enabling expressive, algorithmic play.

- **Separated Scale Change for Vertical/Horizontal Steps**
  - Independently assign musical scales for vertical (Y-axis) and horizontal (X-axis) movement, allowing complex modal and harmonic explorations.

- **Movement**
  - Instantly change running direction of the sequencer, creating evolving or retrograde patterns at the touch.
    - Forward
    - Reverse
    - Pendulum
    - Random

- **Arpeggiator Mode**
  - When enabled, the sequencer steps only through positions matching the current regex, producing arpeggiator-like melodic patterns from your rules.

- **Generated Text Content (Dissociative Press Algorithm)**
  - Generate new musical or textual material using the Dissociative Press algorithm, for creative pattern mutation and generative composition.
  - Manaul file loader TBD

- **Accumulation Mode (Semi Self-Configuration)**
  - Activate accumulation mode to let the system semi-autonomously reconfigure itself via [Queue System](#queue-system), stacking and evolving patterns for emergent musical results.


- **Modes** (shown in status bar; uppercase = active)
  - `a`: Arpeggiator (see above)
  - `n`: Drain Queue
  - `u`: Accumulation (see above)
  - `l`: Dynamic Length, playhead length adjusts dynamically.
  - `e`: Event Operator, enables event operator triggering from keyboard.
  - `s`: Sweep, sweeps through positions across the playhead range.

- **Scale Selection**
  - 16 scales available: `Chromatic`, `Major`, `Minor`, `Harmonic Minor`, `Melodic Minor`, `Dorian`, `Phrygian`, `Lydian`, `Mixolydian`, `Locrian`, `Major Pentatonic`, `Minor Pentatonic`, `Blues`, `Whole Tone`, `Diminished`, and **Thai 7-TET** (microtonal).
  - Scale root selectable across all 12 chromatic pitches (C–B).


# Queue System

Inpspired by [Event Loop](https://medium.com/@ignatovich.dm/the-javascript-event-loop-explained-with-examples-d8f7ddf0861d), the queue is a first-in-first-out (`FIFO`) dispatch mechanism, events accumulate, wait, and are consumed one at a time, each driving a state transition in the sequencer. every jump or timbral gesture is the result of something that was previously enqueued.

it comprised of 
- **Event Queue (EVQ)** holds pending event operators to be fused into the next push, current available ops are
  - `c` / `>CHORD` chord event triggers a triad on the next matched position.
  - `h` / `>HOLDN` hold event sustains the note on the next matched position.
- **Queue Operators** spatially mapped on keyboard, current available ops are
  - `P` Push, push current playhead position (or front event) onto OPQ.
  - `S` Swap, swap the top two items in OPQ.
  - `O` Pop, execute and remove the front item from OPQ.
  - `D` Duplicate, duplicate the top item in OPQ.

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


# Building
- Docker must be installed before proceeding
- Execute: `sh ./build`
- finger-crossed

**Supported Platforms:**
- Desktop: Linux, macOS, Windows (x86_64, ARM64)
- Embedded: Raspberry Pi 4B (aarch64-unknown-linux-gnu)

# Running

- Desktop mode (default): `cargo run`
- Microcontroller mode: `cargo run --no-default-features --features microcontroller`

# Compilation
- Desktop mode (default): `cargo build --release`
- Microcontroller mode: `cargo build --release --no-default-features --features microcontroller`


# Credits

- Typography: [Departure Mono](https://departuremono.com/)
