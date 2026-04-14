import init, {
  wasm_init,
  wasm_step,
  wasm_send_key,
  wasm_render,
  wasm_resize,
} from "./pkg/anupars.js";

async function main() {
  // Load and initialise the WASM module
  await init();

  // Set up the xterm.js terminal
  const term = new Terminal({
    cursorBlink: false,
    allowProposedApi: true,
    // Use the same font the TUI was designed for
    fontFamily: '"Cascadia Code", "Fira Code", "Source Code Pro", monospace',
    fontSize: 14,
    theme: {
      background: "#000000",
      foreground: "#ffffff",
    },
    // Disable xterm's own cursor — the ANSI stream manages it
    disableStdin: false,
  });

  const fitAddon = new FitAddon.FitAddon();
  term.loadAddon(fitAddon);
  term.open(document.getElementById("terminal"));
  fitAddon.fit();

  // Tell the Rust backend about the initial dimensions
  wasm_init(term.cols, term.rows);

  // Forward every keystroke / paste into the Rust event queue
  term.onData((data) => {
    wasm_send_key(data);
  });

  // Propagate terminal resize events to the Rust backend
  term.onResize(({ cols, rows }) => {
    wasm_resize(cols, rows);
  });

  // Re-fit and re-notify on window resize
  window.addEventListener("resize", () => {
    fitAddon.fit();
    wasm_resize(term.cols, term.rows);
  });

  // Main render loop at ~60 fps
  let lastTs = null;
  function frame(ts) {
    const elapsed = lastTs === null ? 16.0 : ts - lastTs;
    lastTs = ts;

    wasm_step(elapsed);
    const ansi = wasm_render();
    if (ansi.length > 0) {
      term.write(ansi);
    }

    requestAnimationFrame(frame);
  }
  requestAnimationFrame(frame);
}

main().catch((err) => {
  console.error("anupars WASM failed to initialise:", err);
});
