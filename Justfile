dump-nodes:
    blup run -- --background --factory-startup --python dump_nodes.py

fmt-py:
    uv run ruff format dump_nodes.py tests/

test-py:
    uv run pytest

fix-rs:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings

fmt-rs:
    just fix-rs
    cargo fmt --all

build:
    RAMEN_DEBUG_NODES=1 cargo build

test-rs:
    cargo test
