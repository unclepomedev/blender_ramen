dump-nodes:
    blup run -- --background --factory-startup --python dump_nodes.py

fmt-py:
    uv run ruff format --exclude .blender_stubs
