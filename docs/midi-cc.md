## Sweep as a CC modulator

When sweep mode is active, the crosshair moving across the grid can output CC instead of (or alongside) notes. The output mode is controlled by `SweepOutputMode`:

| Mode | Label | Behaviour |
|------|-------|-----------|
| `Note` | _(default, no label)_ | notes only - existing behaviour unchanged |
| `CC` | `cc74` | CC only - no notes triggered |
| `Both` | `cc74+` | notes and CC simultaneously |

Cycle through modes with `@`. Adjust the CC number with `[` (decrease) and `]` (increase). The active mode and CC number are shown in the status bar next to the sweep indicator, for example `S<cc74>` or `S<cc74+>`.

## How it works, visually

```
GRID (regex matches shown as █, empty as ·)
─────────────────────────────────────────────────────
col:  0    1    2    3    4    5    6    7    8    9
      █    █    █    ·    ·    █    █    ·    █    █


LFO PHASE (always advancing, sawtooth example)
─────────────────────────────────────────────────────
col:  0    1    2    3    4    5    6    7    8    9
val: 14   28   42   56   70   84   98  112  126   12


CC OUTPUT SENT (what the synth actually receives)
─────────────────────────────────────────────────────
col:  0    1    2    3    4    5    6    7    8    9
      14   28   42   —    —   84   98   —   126   12
                     ↑    ↑             ↑
                   MUTED MUTED        MUTED
                   (CC value held at latest value)


SYNTH PARAMETER OVER TIME
─────────────────────────────────────────────────────

sweep pos:  [0]──[1]──[2]──[3]──[4]──[5]──[6]──[7]──[8]──[9]
regex:       █    █    █    ·    ·    █    █    ·    █    █

synth hears:
┌──────────────────────────────────────────────────────────┐
│  /‾\                /‾‾‾‾‾\              /‾‾‾‾‾‾‾‾‾\     │
│ /   \              /       \            /           \    │
│      \            /         \          /             \   │
│       \__________/           \________/               \  │
│       ↑↑↑↑↑↑↑↑↑↑             ↑↑↑↑↑↑↑↑                    │
│       synth HOLDS             synth HOLDS                │
│       last received           last received              │
│       value (42)              value (98)                 │
└──────────────────────────────────────────────────────────┘
            ↑                        ↑
          STUCK at 42              STUCK at 98
          until next match         until next match


WAVEFORM SHAPES (LFO phase → CC value mapping)
─────────────────────────────────────────────────────

Forward  (sawtooth):         0──────────────────127 → 0
Reverse  (reverse sawtooth): 127──────────────────0 → 127
Pendulum (triangle):         0────64────127────64────0
Random   (S&H):              ?    ?    ?    ?    ?
                             ↑ jumps to new random val each step
```

## LFO - how the CC value is computed

The CC value is derived from the sweep crosshair's position within the playhead area, computed each tick as:

```
phase = (sweep_x - area_left) / area_width   -- range [0.0, 1.0)
cc_value = phase * 127
```

The waveform shape comes for free from the existing sweep movement setting:

| Sweep movement | CC waveform shape |
|----------------|-------------------|
| Forward | sawtooth up - 0 to 127 |
| Reverse | sawtooth down - 127 to 0 |
| Pendulum | triangle - 0 to 127 to 0 |
| Random | sample-and-hold - jumps each step |

The LFO rate is determined by BPM and the DIV setting (`{` / `}`). A higher DIV means faster CC modulation.

## Interaction with other modes

- **Sweep Row Mode** (`|`) - filters which rows fire CC, creating rhythmic CC patterns from sparse row selections
- **Tilt mode** (`Ctrl+t`) - shifts the column each row fires from diagonally, sending CC on different channels per row simultaneously
- **Sweep Movement** (`!`) - gives the crosshair an independent direction, decoupling the CC LFO rate and shape from normal playhead movement

## Quick reference

| Key | Action |
|-----|--------|
| `Ctrl+s` | toggle sweep mode |
| `@` | cycle sweep output mode - Note, CC, Both |
| `[` | decrease CC number by 1 |
| `]` | increase CC number by 1 |
| `!` | toggle independent sweep movement (LFO direction) |
| `Ctrl+f` / `Ctrl+r` / `Ctrl+p` / `Ctrl+d` | set sweep movement waveform shape |
