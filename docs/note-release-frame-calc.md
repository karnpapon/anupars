# Note Release Formula Derivation

Compute the maximum number of stack frames a MIDI note may hold so that the
note-off message is guaranteed to arrive before the next note-on, even at
high DIV values (32, 64).

## Constants

| Symbol | Value | Meaning |
|--------|-------|---------|
| $T$ | $60{,}000$ ms/min | tempo base |
| $\tau$ | $16$ ticks/beat | `DEFAULT_TICKS_PER_BEAT` |
| $F$ | $8$ ms/frame | stack `refresh` interval |

## Derivation

### Step 1: Duration of one tick (ms)

$$\text{tick-ms} = \frac{T}{\text{bpm} \times \tau} = \frac{60{,}000}{\text{bpm} \times 16}$$

### Step 2: Duration of one playhead step (ms)

The clock handler uses `base_divider` $= 64 / \text{div}$, so the playhead advances every `base_divider` ticks:

$$\text{step-ms} = \frac{64}{\text{div}} \times \text{tick-ms} = \frac{64}{\text{div}} \times \frac{60{,}000}{\text{bpm} \times 16} = \frac{240{,}000}{\text{div} \times \text{bpm}}$$

### Step 3: Convert ms → stack frames

Each stack frame is $F = 8$ ms:

$$\text{step-frames} = \frac{\text{step-ms}}{F} = \frac{240{,}000}{\text{div} \times \text{bpm} \times 8} = \boxed{\frac{30{,}000}{\text{div} \times \text{bpm}}}$$

The constant $30{,}000$ collapses from $T \times (64/\tau) / F = 60{,}000 \times 4 / 8$.

### Step 4: Apply fill factor

A fill factor of $85\%$ ensures a gap between note-off and the next note-on, preventing stuck notes:

$$\text{step-max-frames} = \left\lfloor \frac{30{,}000}{\text{div} \times \text{bpm}} \times 0.85 \right\rfloor = \left\lfloor \frac{30{,}000 \times 85}{100 \times \text{div} \times \text{bpm}} \right\rfloor$$

## Sanity Check at 120 BPM

| DIV | step_ms | step_frames | ×85% cap (frames) |
|----:|--------:|------------:|------------------:|
| 8   | 250.0 ms | 31 | 26 |
| 16  | 125.0 ms | 15 | 13 |
| 32  | 62.5 ms  |  7 |  6 |
| 64  | 31.25 ms |  3 |  2 |
