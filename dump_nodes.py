import bpy
import json
import os
import mathutils
import sys

OUTPUT_FILE = "blender_nodes_dump.json"

CANDIDATE_PREFIXES = ["GeometryNode", "ShaderNode", "Node", "FunctionNode"]

# TODO Only allow property types that can be safely retrieved.
# Exclude POINTER (data block references) and COLLECTION (lists) as they can cause crashes.
SAFE_PROP_TYPES = {"STRING", "BOOLEAN", "INT", "FLOAT", "ENUM"}

EXCLUDE_PROPS = {
    "rna_type",
    "name",
    "label",
    "inputs",
    "outputs",
    "location",
    "dimensions",
    "width",
    "height",
    "parent",
    "use_custom_color",
    "color",
    "select",
    "show_options",
    "show_preview",
    "show_texture",
    "bl_idname",
    "bl_label",
    "bl_description",
    "bl_icon",
    "bl_static_type",
    "bl_width_default",
    "bl_width_min",
    "bl_width_max",
}


def safe_convert(val):
    if val is None:
        return None
    if isinstance(
        val, (mathutils.Vector, mathutils.Color, mathutils.Euler, mathutils.Quaternion)
    ):
        return list(val)
    if type(val).__name__ == "bpy_prop_array":
        return list(val)
    if isinstance(val, set):
        return list(val)
    if type(val).__name__ == "NodeEnumItem":
        return getattr(val, "identifier", str(val))
    if hasattr(val, "to_dict"):
        return val.to_dict()

    if isinstance(val, (int, float, str, bool)):
        return val
    return str(val)


def get_socket_info(socket):
    info = {
        "name": str(socket.name),
        "identifier": str(socket.identifier),
        "type": str(socket.bl_idname),
        "description": str(getattr(socket, "description", "")),
    }
    if hasattr(socket, "default_value"):
        try:
            info["default"] = safe_convert(socket.default_value)
        except Exception:
            info["default"] = None
    return info


def get_properties_info(node):
    props = []
    if not hasattr(node, "bl_rna"):
        return props

    for prop in node.bl_rna.properties:
        if prop.is_readonly:
            continue
        if prop.identifier in EXCLUDE_PROPS:
            continue

        if prop.type not in SAFE_PROP_TYPES:
            continue

        prop_def = {
            "identifier": str(prop.identifier),
            "name": str(prop.name),
            "type": str(prop.type),
            "description": str(prop.description),
        }

        try:
            raw_val = getattr(node, prop.identifier)
            prop_def["default"] = safe_convert(raw_val)
        except Exception:
            continue

        props.append(prop_def)
    return props


def scan_valid_nodes_for_tree(tree_type, system_label):
    print(f"--- Scanning valid nodes for {system_label} ---", file=sys.stderr)

    try:
        temp_tree = bpy.data.node_groups.new(f"Temp_{tree_type}", tree_type)
    except Exception as e:
        print(f"Error creating tree {tree_type}: {e}", file=sys.stderr)
        return {}

    try:
        nodes = temp_tree.nodes
        definitions = {}

        all_types = dir(bpy.types)
        candidates = []
        for name in all_types:
            for prefix in CANDIDATE_PREFIXES:
                if name.startswith(prefix):
                    candidates.append(name)
                    break

        print(
            f"Found {len(candidates)} candidate classes. Testing instantiation...",
            file=sys.stderr,
        )

        success_count = 0

        for cls_name in candidates:
            try:
                cls = getattr(bpy.types, cls_name)
                node_id = getattr(cls, "bl_idname", cls_name)

                node = nodes.new(node_id)

                node_def = {
                    "bl_idname": str(node.bl_idname),
                    "bl_label": str(node.bl_label),
                    "inputs": [get_socket_info(s) for s in node.inputs],
                    "outputs": [get_socket_info(s) for s in node.outputs],
                    "properties": get_properties_info(node),
                }

                definitions[node.bl_idname] = node_def
                nodes.remove(node)
                success_count += 1

            except Exception:
                continue

        print(
            f"-> Successfully dumped {success_count} nodes for {system_label}.",
            file=sys.stderr,
        )
        return definitions

    finally:
        bpy.data.node_groups.remove(temp_tree)


def main():
    full_dump = {
        "GeometryNodes": scan_valid_nodes_for_tree(
            "GeometryNodeTree", "Geometry Nodes"
        ),
        "ShaderNodes": scan_valid_nodes_for_tree("ShaderNodeTree", "Shader Nodes"),
        "CompositorNodes": scan_valid_nodes_for_tree(
            "CompositorNodeTree", "Compositor Nodes"
        ),
    }

    output_path = os.path.join(".", OUTPUT_FILE)
    print(f"Writing JSON to {output_path}...", file=sys.stderr)

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(full_dump, f, indent=2)
    print("Done!", file=sys.stderr)


if __name__ == "__main__":
    main()
