# python ==========================================================
fmt-py:
    uv run ruff format dump_nodes.py tests/ server.py

test-py:
    uv run pytest

# rust ==========================================================
fix-rs:
    cargo clippy --fix --allow-dirty --allow-staged --all-targets -- -D warnings

fmt-rs:
    just fix-rs
    cargo fmt --all

test-rs:
    cargo test

# setup =========================================================
dump-nodes:
    blup run -- --background --factory-startup --python dump_nodes.py

build:
    RAMEN_DEBUG_NODES=1 cargo build

# boot ===========================================================
blender:
    echo "🍜 Starting Blender with Live Link Server..."
    blup run -- --python server.py

ex target:
    #!/usr/bin/env bash
    set -euo pipefail
    MATCH=$(find examples -maxdepth 1 -name "*{{target}}*.rs" | sort | head -n 1)
    if [ -z "$MATCH" ]; then
        echo "❌ Error: No example matching '{{target}}' found."
        exit 1
    fi
    BASENAME=$(basename "$MATCH" .rs)
    echo "🚀 Running: cargo run --example $BASENAME"
    cargo run --example "$BASENAME"
