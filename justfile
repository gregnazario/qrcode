default:
    @just --list

build:
    cargo build --workspace --release

test:
    cargo test --workspace

fmt:
    cargo fmt --all

lint:
    cargo clippy --workspace --all-targets -- -D warnings

check: fmt lint test

build-wasm:
    wasm-pack build crates/qrkit-wasm --target web --out-dir ../../web/pkg

serve:
    python3 -m http.server 8080 --directory web
