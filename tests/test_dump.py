import os
import json
import subprocess
import pytest

EXPECTED_JSON_PATH = os.path.join(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__))),
    "blender_nodes_dump.json",
)


@pytest.fixture(scope="session")
def dumped_json_file():
    if os.path.exists(EXPECTED_JSON_PATH):
        os.remove(EXPECTED_JSON_PATH)

    result = subprocess.run(
        ["just", "dump-nodes"],
        capture_output=True,
        text=True,
        timeout=300,
    )

    assert result.returncode == 0, (
        f"Blender script execution failed!\nSTDERR:\n{result.stderr}"
    )
    assert os.path.exists(EXPECTED_JSON_PATH), (
        "JSON file was not created by the script!"
    )

    return EXPECTED_JSON_PATH


def test_blender_dump_execution(dumped_json_file):
    assert os.path.exists(dumped_json_file)


def test_dumped_json_schema(dumped_json_file):
    with open(dumped_json_file, "r", encoding="utf-8") as f:
        data = json.load(f)

    for category in ["GeometryNodes", "ShaderNodes", "CompositorNodes"]:
        assert category in data, f"Missing category: {category}"
        assert len(data[category]) > 0, f"{category} is empty!"

    geo_nodes = data["GeometryNodes"]
    assert "GeometryNodeMeshCube" in geo_nodes, (
        "Missing standard node GeometryNodeMeshCube"
    )

    cube_node = geo_nodes["GeometryNodeMeshCube"]
    assert "bl_idname" in cube_node
    assert "bl_label" in cube_node
    assert "inputs" in cube_node
    assert "outputs" in cube_node
    assert "properties" in cube_node

    shader_nodes = data["ShaderNodes"]
    assert "ShaderNodeMath" in shader_nodes

    math_node = shader_nodes["ShaderNodeMath"]
    operation_prop = next(
        (p for p in math_node["properties"] if p["identifier"] == "operation"), None
    )

    assert operation_prop is not None, "Math node should have an 'operation' property"
    assert operation_prop["type"] == "ENUM"
    assert "enum_items" in operation_prop
    assert len(operation_prop["enum_items"]) > 0
    assert "identifier" in operation_prop["enum_items"][0]
    assert "name" in operation_prop["enum_items"][0]
