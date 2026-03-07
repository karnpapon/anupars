# Copilot Instructions

## Rust Module Structure

### `mod.rs`: parent-level exports only

A `mod.rs` file must only declare its direct child modules. It must **not** re-export items or submodules from deeper levels.

**Correct:**

```rust
// src/core/engine/mod.rs
pub mod disspress;
pub mod regex;
pub mod stack;
```

**Wrong: do not do this:**

```rust
pub use engine::{disspress, regex, stack}; // flattens submodule paths
pub use io::midi;                          // re-exports across module boundary
pub use tonal::scale;                      // hides where the type actually lives
```

### Imports: one `use` per line

Every `use` statement must be on its own line. Do not group imports with `{...}` braces.

**Correct:**

```rust
use crate::core::engine::regex;
use crate::core::engine::regex::Match;
use crate::core::io::midi;
use crate::core::tonal::scale::ScaleMode;
```

**Wrong: do not do this:**

```rust
use crate::core::{consts, io::midi, engine::regex::Match, utils};
use crate::core::{consts, midi, regex::Match};
```

### Imports: top of file only

All `use` statements must appear at the top of the file, never inside function bodies, closures, `impl` blocks, or any other inner scope.

**Correct:**

```rust
use crate::core::tonal::scale::ScaleMode;

fn set_scale(mode: ScaleMode) { ... }
```

**Wrong: do not do this:**

```rust
fn build_menu() {
  use crate::core::tonal::scale::ScaleMode; // inline import: not allowed
  let mode = ScaleMode::default();
}
```

### Import paths must reflect the actual file location

When a module moves to a submodule folder, every import must be updated to use its full path. Never rely on re-exports to keep old paths working.

**Example: after moving `midi.rs` into `src/core/io/`:**

```rust
// correct
use crate::core::io::midi;

// wrong: path no longer reflects file location
use crate::core::midi;
```
