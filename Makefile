WASM_PACK := wasm-pack
CARGO     := cargo
OUT_DIR   := web/pkg
SERVE_DIR := web

.PHONY: all wasm wasm-dev serve clean

all: wasm

## Build WASM in release mode (optimised, copied to web/pkg/)
wasm:
	$(WASM_PACK) build \
		--target web \
		--out-dir $(OUT_DIR) \
		--release

## Build WASM in debug mode (faster compile, larger binary)
wasm-dev:
	$(WASM_PACK) build \
		--target web \
		--out-dir $(OUT_DIR) \
		--dev

## Serve the web directory for local testing
serve:
	PORT=3001 npx serve $(SERVE_DIR)

## Remove wasm-pack build artefacts (keeps target/)
clean:
	rm -rf $(OUT_DIR)
	$(CARGO) clean --target wasm32-unknown-unknown
