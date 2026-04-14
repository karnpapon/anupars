import init, {
  wasm_init,
  wasm_step,
  wasm_send_key,
  wasm_send_mouse,
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
    fontFamily: '"Cascadia Code", "Fira Code", "Source Code Pro", monospace',
    fontSize: 14,
    theme: {
      background: "#000000",
      foreground: "#ffffff",
    },
    // Treat macOS Option as Meta for simple Option+letter combos.
    // Option+Shift+letter is handled by attachCustomKeyEventHandler below.
    macOptionIsMeta: true,
    disableStdin: false,
  });

  const fitAddon = new FitAddon.FitAddon();
  term.loadAddon(fitAddon);
  term.open(document.getElementById("terminal"));
  fitAddon.fit();

  wasm_init(term.cols, term.rows);

  // ── Keyboard ──────────────────────────────────────────────────────────────

  // Intercept Option/Alt + char (with or without Shift) before xterm.js does.
  // macOptionIsMeta handles bare Option+letter, but on macOS some
  // Option+Shift+letter combos still go through the OS IME and produce
  // Unicode characters instead of \x1b+char. Returning false blocks onData.
  term.attachCustomKeyEventHandler((e) => {
    if (e.type !== "keydown") return true;
    if (!e.altKey || e.ctrlKey) return true;
    if (e.key.length !== 1) return true; // skip arrows, F-keys, etc.
    wasm_send_key("\x1b" + e.key);
    return false; // prevent xterm.js default processing
  });

  // Forward every other keystroke / paste into the Rust event queue
  term.onData((data) => {
    wasm_send_key(data);
  });

  // ── Mouse ─────────────────────────────────────────────────────────────────

  // Convert a DOM MouseEvent pixel position to terminal cell coordinates.
  function cellPos(e) {
    const el = term.element;
    const rect = el.getBoundingClientRect();
    const col = Math.floor((e.clientX - rect.left) / (rect.width / term.cols));
    const row = Math.floor((e.clientY - rect.top) / (rect.height / term.rows));
    return {
      col: Math.max(0, Math.min(term.cols - 1, col)),
      row: Math.max(0, Math.min(term.rows - 1, row)),
    };
  }

  let dragging = false;
  let dragButton = 0;

  // Use capture mode so our handler fires before xterm.js's inner listeners.
  // stopPropagation() prevents the event from reaching xterm.js's selection
  // manager; preventDefault() blocks native browser text-selection.
  term.element.addEventListener("mousedown", (e) => {
    dragging = true;
    dragButton = e.button;
    const { col, row } = cellPos(e);
    wasm_send_mouse(0, e.button, col, row); // Press
    term.focus();          // keep keyboard input alive
    e.preventDefault();
    e.stopPropagation();   // stop xterm.js selection from starting
  }, true /* capture */);

  term.element.addEventListener("mousemove", (e) => {
    if (!dragging) return;
    const { col, row } = cellPos(e);
    wasm_send_mouse(1, dragButton, col, row); // Hold / drag
  });

  window.addEventListener("mouseup", (e) => {
    if (!dragging) return;
    dragging = false;
    const { col, row } = cellPos(e);
    wasm_send_mouse(2, e.button, col, row); // Release
  });

  // Block the browser context menu so right-click reaches the app.
  term.element.addEventListener("contextmenu", (e) => e.preventDefault());

  // Prevent browser text-selection during mouse drags inside the terminal.
  term.element.addEventListener("selectstart", (e) => e.preventDefault());

  // ── Resize ────────────────────────────────────────────────────────────────

  term.onResize(({ cols, rows }) => {
    wasm_resize(cols, rows);
  });

  window.addEventListener("resize", () => {
    fitAddon.fit();
    wasm_resize(term.cols, term.rows);
  });

  // ── Render loop (~60 fps) ─────────────────────────────────────────────────

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
