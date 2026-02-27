import bpy
import json
import os
import mathutils
import sys

OUTPUT_FILE = "blender_nodes_dump.json"

CANDIDATE_PREFIXES = [
    "GeometryNode",
    "ShaderNode",
    "CompositorNode",
    "Node",
    "FunctionNode",
]

_ALL_CANDIDATE_CLASSES = [
    name
    for name in dir(bpy.types)
    if any(name.startswith(p) for p in CANDIDATE_PREFIXES)
]

# Only allow property types that can be safely retrieved.
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
    info: dict = {
        "name": str(socket.name),
        "identifier": str(socket.identifier),
        "type": str(socket.bl_idname),
        "description": str(getattr(socket, "description", "")),
        "is_multi_input": getattr(socket, "is_multi_input", False),
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

    skip_count = 0
    for prop in node.bl_rna.properties:
        if prop.is_readonly:
            continue
        if prop.identifier in EXCLUDE_PROPS:
            continue

        if prop.type not in SAFE_PROP_TYPES:
            continue

        prop_def: dict = {
            "identifier": str(prop.identifier),
            "name": str(prop.name),
            "type": str(prop.type),
            "description": str(prop.description),
        }

        if prop.type == "ENUM":
            prop_def["enum_items"] = [
                {
                    "identifier": item.identifier,
                    "name": item.name,
                    "description": item.description,
                }
                for item in prop.enum_items
            ]

        try:
            raw_val = getattr(node, prop.identifier)
            prop_def["default"] = safe_convert(raw_val)
        except Exception as e:
            print(
                f"  Warning: could not read property {prop.identifier}: {e}",
                file=sys.stderr,
            )
            prop_def["default"] = None
            skip_count += 1

        props.append(prop_def)
    if skip_count:
        print(
            f"  [warn] Could not retrieve defaults for {skip_count} properties.",
            file=sys.stderr,
        )
    return props


def scan_valid_nodes_for_tree(tree_type, system_label):
    """
    Scans and instances valid node classes for a given node tree type.

    Note on skipped candidates:
    It is completely normal and expected to see a high number of "skipped" candidates
    in the output logs. The scanner tests all available node classes in Blender against
    the target tree type (e.g., GeometryNodeTree) by brute-forcing instantiation.

    Candidates are gracefully skipped for the following reasons:
    1. Context Mismatch: Nodes belonging to other contexts (e.g., attempting to add
       a ShaderNodeEmission into a GeometryNodeTree) will be rejected by Blender.
    2. Abstract/Internal Classes: Abstract base classes (like `Node` or `GeometryNode`)
       and UI-related components (like `NodeTreeInterfaceSocket`) cannot be instantiated
       as physical nodes in the graph.

    Therefore, the successfully dumped nodes should represent the majority of
    standard, built-in functional nodes available in that specific editor.
    """
    print(f"--- Scanning valid nodes for {system_label} ---", file=sys.stderr)

    try:
        temp_tree = bpy.data.node_groups.new(f"Temp_{tree_type}", tree_type)
    except Exception as e:
        print(f"Error creating tree {tree_type}: {e}", file=sys.stderr)
        return {}

    try:
        nodes = temp_tree.nodes
        definitions = {}

        print(
            f"Found {len(_ALL_CANDIDATE_CLASSES)} candidate classes. Testing instantiation...",
            file=sys.stderr,
        )

        success_count = 0
        failed_count = 0

        for cls_name in _ALL_CANDIDATE_CLASSES:
            node = None
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
                success_count += 1

            except Exception:
                failed_count += 1
            finally:
                if node is not None:
                    try:
                        nodes.remove(node)
                    except Exception as e:
                        print(
                            f"  Warning: could not remove node {cls_name}: {e}",
                            file=sys.stderr,
                        )

        print(
            f"-> Successfully dumped {success_count} nodes for {system_label} "
            + f"({failed_count} candidates skipped).",
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

    output_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), OUTPUT_FILE)
    print(f"Writing JSON to {output_path}...", file=sys.stderr)

    try:
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(full_dump, f, indent=2)
        print("Done!", file=sys.stderr)
    except OSError as e:
        print(f"Error writing output file {output_path}: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
