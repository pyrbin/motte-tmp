set shell := ["nu.exe", "-c"]

bin := "motte"

watch:
    cargo watch --features 'dev_tools, dynamic_linking' -q -c -x 'run -- --bin $bin'

dev:
    cargo run --bin $bin --features 'dev_tools, dynamic_linking'

run:
    cargo run --bin $bin --release

run-wasm:
    cargo run --bin $bin --target wasm32-unknown-unknown --features 'dev_tools'

build:
    cargo build --bin $bin --release

build-wasm:
    cargo build --bin $bin --profile wasm --target wasm32-unknown-unknown

build-debug:
    cargo build --bin $bin --features 'dev_tools'

fix:
	just clippy
	just fmt

fmt: 
    cargo fmt -v --all

clippy:
    cargo clippy --fix --workspace --all-features --allow-dirty --allow-no-vcs -- --no-deps

ci: 
    cargo run -p ci

clean: 
    cargo clean
