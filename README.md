# anu-rs

A Rust-based reimplementation of `anu`, designed to operate on resource-constrained devices.

> [!WARNING]
> This project is a **work in progress**. Features and APIs are subject to change anytime.

<img src="ss.png" />

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