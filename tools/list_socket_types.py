"""
List all unique socket types from blender_nodes_dump.json,
separated by input and output.
"""

import json
from pathlib import Path

dump_path = Path(__file__).parent.parent / "blender_nodes_dump.json"
data = json.loads(dump_path.read_text())

input_types: set[str] = set()
output_types: set[str] = set()

for category in data.values():
    for node in category.values():
        for socket in node.get("inputs", []):
            input_types.add(socket["type"])
        for socket in node.get("outputs", []):
            output_types.add(socket["type"])

out_path = Path(__file__).parent / "socket_types.txt"
with open(out_path, "w") as f:
    # input_types.update(output_types)  # for enum generation
    # for t in sorted(input_types):
    #     f.write(f"    {t},\n")
    f.write("=== Input Socket Types ===\n")
    for t in sorted(input_types):
        f.write(f"  {t}\n")
    f.write(f"\n=== Output Socket Types ===\n")
    for t in sorted(output_types):
        f.write(f"  {t}\n")

print(f"Written to {out_path}")
