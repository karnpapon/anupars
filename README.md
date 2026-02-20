# anupars

anupars (อนุภา(ส)), a Rust-based reimagining of [`anu`](https://github.com/karnpapon/anu) a musical step-sequencer driven by regular expressions, designed to operate on resource-constrained devices, and performance-oriented.

> [!WARNING]
> This project is a **work in progress**. Features and APIs are subject to change anytime.

<img src="ss.png" />

# Features

- **MIDI Out Selector**
  - Choose from available MIDI output devices for flexible routing to synths, DAWs, or hardware.

- **Keyboard MIDI Layout**
  - The on-screen keyboard uses a spatial layout similar to Laurie Spiegel's [Music Mouse](https://en.wikipedia.org/wiki/Music_Mouse), enabling expressive, algorithmic play.

- **Separated Scale Change for Vertical/Horizontal Steps**
  - Independently assign musical scales for vertical (Y-axis) and horizontal (X-axis) movement, allowing complex modal and harmonic explorations.

- **Reverse Step Mode**
  - Instantly reverse the running direction of the sequencer, creating evolving or retrograde patterns at the touch of a button.

- **Arpeggiator Mode**
  - When enabled, the sequencer steps only through positions matching the current regex, producing arpeggiator-like melodic patterns from your rules.

- **Generated Text Content (Dissociative Press Algorithm)**
  - Generate new musical or textual material using the Dissociative Press algorithm, for creative pattern mutation and generative composition.
  - Manaul file loader TBD

- **Accumulation Mode (Semi Self-Configuration)**
  - Activate accumulation mode to let the system semi-autonomously reconfigure itself, stacking and evolving patterns for emergent musical results.

- **OSC**
  - Soon

- **Multi-step**
  - TBD

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