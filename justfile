set shell := ["nu.exe", "-c"]

bin := "motte"

watch:
    cargo watch --features dev_tools -q -c -x 'run -- --bin $bin'

run:
    cargo run --bin $bin --features dev_tools

wasm:
    cargo run --bin $bin --target wasm32-unknown-unknown --features dev_tools

build:
    cargo build --release --bin $bin

build-debug:
    cargo build --bin $bin --features dev_tools

fix:
	just clippy
	just fmt

fmt: 
    cargo fmt -v --all

clippy:
    # TODO: remove "allow dead_code" when I care about that
    cargo clippy --fix --workspace --all-features --allow-dirty --allow-no-vcs -- -A dead_code --no-deps

ci: 
    cargo run -p ci

clean: 
    cargo clean
