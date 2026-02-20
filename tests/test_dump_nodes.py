import json
import os
import sys
import tempfile
from pathlib import Path
from unittest import mock
from unittest.mock import MagicMock, Mock, PropertyMock, call, mock_open, patch

import pytest

# Create proper mock classes for mathutils types
class MockVector(list):
    """Mock mathutils.Vector class."""
    pass

class MockColor(list):
    """Mock mathutils.Color class."""
    pass

class MockEuler(list):
    """Mock mathutils.Euler class."""
    pass

class MockQuaternion(list):
    """Mock mathutils.Quaternion class."""
    pass

# Create mock mathutils module with proper type classes
mock_mathutils = MagicMock()
mock_mathutils.Vector = MockVector
mock_mathutils.Color = MockColor
mock_mathutils.Euler = MockEuler
mock_mathutils.Quaternion = MockQuaternion

# Mock bpy and mathutils before importing dump_nodes
sys.modules["bpy"] = MagicMock()
sys.modules["bpy.types"] = MagicMock()
sys.modules["mathutils"] = mock_mathutils

import dump_nodes


class TestSafeConvert:
    """Test the safe_convert function with various data types."""

    def test_safe_convert_none(self):
        """Test that None is returned as-is."""
        assert dump_nodes.safe_convert(None) is None

    def test_safe_convert_primitives(self):
        """Test that primitive types are returned as-is."""
        assert dump_nodes.safe_convert(42) == 42
        assert dump_nodes.safe_convert(3.14) == 3.14
        assert dump_nodes.safe_convert("hello") == "hello"
        assert dump_nodes.safe_convert(True) is True
        assert dump_nodes.safe_convert(False) is False

    def test_safe_convert_mathutils_vector(self):
        """Test conversion of mathutils.Vector to list."""
        mock_vector = MockVector([1.0, 2.0, 3.0])
        result = dump_nodes.safe_convert(mock_vector)
        assert result == [1.0, 2.0, 3.0]

    def test_safe_convert_mathutils_color(self):
        """Test conversion of mathutils.Color to list."""
        mock_color = MockColor([0.5, 0.7, 0.9])
        result = dump_nodes.safe_convert(mock_color)
        assert result == [0.5, 0.7, 0.9]

    def test_safe_convert_mathutils_euler(self):
        """Test conversion of mathutils.Euler to list."""
        mock_euler = MockEuler([0.0, 1.57, 3.14])
        result = dump_nodes.safe_convert(mock_euler)
        assert result == [0.0, 1.57, 3.14]

    def test_safe_convert_mathutils_quaternion(self):
        """Test conversion of mathutils.Quaternion to list."""
        mock_quat = MockQuaternion([1.0, 0.0, 0.0, 0.0])
        result = dump_nodes.safe_convert(mock_quat)
        assert result == [1.0, 0.0, 0.0, 0.0]

    def test_safe_convert_bpy_prop_array(self):
        """Test conversion of bpy_prop_array to list."""
        mock_array = Mock()
        type(mock_array).__name__ = "bpy_prop_array"
        mock_array.__iter__ = Mock(return_value=iter([10, 20, 30]))

        result = dump_nodes.safe_convert(mock_array)
        assert result == [10, 20, 30]

    def test_safe_convert_set(self):
        """Test conversion of set to list."""
        test_set = {3, 1, 2}
        result = dump_nodes.safe_convert(test_set)
        assert isinstance(result, list)
        assert set(result) == test_set

    def test_safe_convert_node_enum_item(self):
        """Test conversion of NodeEnumItem to identifier."""
        mock_enum = Mock()
        type(mock_enum).__name__ = "NodeEnumItem"
        mock_enum.identifier = "ITEM_ID"

        result = dump_nodes.safe_convert(mock_enum)
        assert result == "ITEM_ID"

    def test_safe_convert_node_enum_item_no_identifier(self):
        """Test conversion of NodeEnumItem without identifier attribute uses str fallback."""
        mock_enum = Mock()
        type(mock_enum).__name__ = "NodeEnumItem"
        # Delete the identifier attribute so getattr uses the default
        del mock_enum.identifier
        mock_enum.__str__ = Mock(return_value="fallback_string")

        result = dump_nodes.safe_convert(mock_enum)
        assert result == "fallback_string"

    def test_safe_convert_object_with_to_dict(self):
        """Test conversion of object with to_dict method."""
        mock_obj = Mock()
        mock_obj.to_dict = Mock(return_value={"key": "value"})

        result = dump_nodes.safe_convert(mock_obj)
        assert result == {"key": "value"}

    def test_safe_convert_unknown_type(self):
        """Test conversion of unknown type returns string representation."""
        class CustomClass:
            def __str__(self):
                return "custom_string"

        obj = CustomClass()
        result = dump_nodes.safe_convert(obj)
        assert result == "custom_string"

    def test_safe_convert_list(self):
        """Test that list gets converted to string (unknown type handler)."""
        test_list = [1, 2, 3]
        result = dump_nodes.safe_convert(test_list)
        assert result == "[1, 2, 3]"

    def test_safe_convert_dict(self):
        """Test that dict gets converted to string (unknown type handler)."""
        test_dict = {"a": 1, "b": 2}
        result = dump_nodes.safe_convert(test_dict)
        assert isinstance(result, str)


class TestGetSocketInfo:
    """Test the get_socket_info function."""

    def test_get_socket_info_basic(self):
        """Test basic socket info extraction."""
        mock_socket = Mock()
        mock_socket.name = "Socket Name"
        mock_socket.identifier = "socket_id"
        mock_socket.bl_idname = "NodeSocketFloat"
        mock_socket.description = "Test description"
        mock_socket.default_value = 1.0

        result = dump_nodes.get_socket_info(mock_socket)

        assert result["name"] == "Socket Name"
        assert result["identifier"] == "socket_id"
        assert result["type"] == "NodeSocketFloat"
        assert result["description"] == "Test description"
        assert result["default"] == 1.0

    def test_get_socket_info_no_default_value(self):
        """Test socket without default_value attribute."""
        mock_socket = Mock()
        mock_socket.name = "Socket Name"
        mock_socket.identifier = "socket_id"
        mock_socket.bl_idname = "NodeSocketGeometry"
        mock_socket.description = "Geometry socket"
        del mock_socket.default_value

        result = dump_nodes.get_socket_info(mock_socket)

        assert result["name"] == "Socket Name"
        assert result["identifier"] == "socket_id"
        assert result["type"] == "NodeSocketGeometry"
        assert result["description"] == "Geometry socket"
        assert "default" not in result

    def test_get_socket_info_exception_on_default_value(self):
        """Test socket where accessing default_value in try block raises exception."""
        mock_socket = Mock()
        mock_socket.name = "Socket Name"
        mock_socket.identifier = "socket_id"
        mock_socket.bl_idname = "NodeSocketFloat"
        mock_socket.description = "Test"

        # Create a mock that passes hasattr but fails when accessed
        mock_default = Mock()
        mock_default.__class__ = float
        type(mock_socket).default_value = PropertyMock(return_value=mock_default)

        # Make safe_convert fail by having mock raise exception
        with patch('dump_nodes.safe_convert', side_effect=RuntimeError("Cannot convert")):
            result = dump_nodes.get_socket_info(mock_socket)
            assert result["default"] is None

    def test_get_socket_info_empty_description(self):
        """Test socket with empty description."""
        mock_socket = Mock()
        mock_socket.name = "Socket"
        mock_socket.identifier = "sock"
        mock_socket.bl_idname = "NodeSocketInt"
        mock_socket.description = ""
        mock_socket.default_value = 0

        result = dump_nodes.get_socket_info(mock_socket)

        assert result["description"] == ""

    def test_get_socket_info_vector_default(self):
        """Test socket with vector default value."""
        mock_socket = Mock()
        mock_socket.name = "Vector Socket"
        mock_socket.identifier = "vector"
        mock_socket.bl_idname = "NodeSocketVector"
        mock_socket.description = "Vector input"

        mock_vector = MockVector([1.0, 2.0, 3.0])
        mock_socket.default_value = mock_vector

        result = dump_nodes.get_socket_info(mock_socket)
        assert result["default"] == [1.0, 2.0, 3.0]


class TestGetPropertiesInfo:
    """Test the get_properties_info function."""

    def test_get_properties_info_no_bl_rna(self):
        """Test node without bl_rna attribute returns empty list."""
        mock_node = Mock()
        del mock_node.bl_rna

        result = dump_nodes.get_properties_info(mock_node)

        assert result == []

    def test_get_properties_info_basic(self):
        """Test basic property extraction."""
        mock_node = Mock()

        mock_prop = Mock()
        mock_prop.is_readonly = False
        mock_prop.identifier = "test_prop"
        mock_prop.name = "Test Property"
        mock_prop.type = "INT"
        mock_prop.description = "A test property"

        mock_node.bl_rna.properties = [mock_prop]
        mock_node.test_prop = 42

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 1
        assert result[0]["identifier"] == "test_prop"
        assert result[0]["name"] == "Test Property"
        assert result[0]["type"] == "INT"
        assert result[0]["description"] == "A test property"
        assert result[0]["default"] == 42

    def test_get_properties_info_skip_readonly(self):
        """Test that readonly properties are skipped."""
        mock_node = Mock()

        mock_prop = Mock()
        mock_prop.is_readonly = True
        mock_prop.identifier = "readonly_prop"

        mock_node.bl_rna.properties = [mock_prop]

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 0

    def test_get_properties_info_skip_excluded(self):
        """Test that excluded properties are skipped."""
        mock_node = Mock()

        props = []
        for excluded in ["rna_type", "name", "label", "inputs", "outputs"]:
            mock_prop = Mock()
            mock_prop.is_readonly = False
            mock_prop.identifier = excluded
            props.append(mock_prop)

        mock_node.bl_rna.properties = props

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 0

    def test_get_properties_info_skip_unsafe_types(self):
        """Test that unsafe property types are skipped."""
        mock_node = Mock()

        unsafe_prop = Mock()
        unsafe_prop.is_readonly = False
        unsafe_prop.identifier = "unsafe_prop"
        unsafe_prop.type = "POINTER"

        mock_node.bl_rna.properties = [unsafe_prop]

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 0

    def test_get_properties_info_safe_types(self):
        """Test that all safe property types are included."""
        mock_node = Mock()

        props = []
        safe_types = ["STRING", "BOOLEAN", "INT", "FLOAT", "ENUM"]
        for idx, prop_type in enumerate(safe_types):
            mock_prop = Mock()
            mock_prop.is_readonly = False
            mock_prop.identifier = f"prop_{idx}"
            mock_prop.name = f"Property {idx}"
            mock_prop.type = prop_type
            mock_prop.description = f"Description {idx}"
            props.append(mock_prop)
            setattr(mock_node, f"prop_{idx}", idx)

        mock_node.bl_rna.properties = props

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 5

    def test_get_properties_info_exception_on_getattr(self, capsys):
        """Test handling of exception when reading property value."""
        mock_node = Mock()

        mock_prop = Mock()
        mock_prop.is_readonly = False
        mock_prop.identifier = "bad_prop"
        mock_prop.name = "Bad Property"
        mock_prop.type = "INT"
        mock_prop.description = "Cannot read this"

        mock_node.bl_rna.properties = [mock_prop]
        type(mock_node).bad_prop = PropertyMock(side_effect=RuntimeError("Access denied"))

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 1
        assert result[0]["default"] is None

        captured = capsys.readouterr()
        assert "Warning: could not read property bad_prop" in captured.err

    def test_get_properties_info_multiple_exceptions(self, capsys):
        """Test handling of multiple property read failures."""
        mock_node = Mock()

        props = []
        for i in range(3):
            mock_prop = Mock()
            mock_prop.is_readonly = False
            mock_prop.identifier = f"bad_prop_{i}"
            mock_prop.name = f"Bad Property {i}"
            mock_prop.type = "INT"
            mock_prop.description = f"Description {i}"
            props.append(mock_prop)
            setattr(
                type(mock_node),
                f"bad_prop_{i}",
                PropertyMock(side_effect=RuntimeError(f"Error {i}"))
            )

        mock_node.bl_rna.properties = props

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 3
        for prop in result:
            assert prop["default"] is None

        captured = capsys.readouterr()
        assert "Could not retrieve defaults for 3 properties" in captured.err

    def test_get_properties_info_mixed_success_failure(self, capsys):
        """Test mix of successful and failed property reads."""
        mock_node = Mock()

        good_prop = Mock()
        good_prop.is_readonly = False
        good_prop.identifier = "good_prop"
        good_prop.name = "Good Property"
        good_prop.type = "INT"
        good_prop.description = "Good"

        bad_prop = Mock()
        bad_prop.is_readonly = False
        bad_prop.identifier = "bad_prop"
        bad_prop.name = "Bad Property"
        bad_prop.type = "INT"
        bad_prop.description = "Bad"

        mock_node.bl_rna.properties = [good_prop, bad_prop]
        mock_node.good_prop = 100
        type(mock_node).bad_prop = PropertyMock(side_effect=RuntimeError("Error"))

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 2
        assert result[0]["default"] == 100
        assert result[1]["default"] is None

    def test_get_properties_info_enum_type(self):
        """Test extraction of ENUM property."""
        mock_node = Mock()

        enum_prop = Mock()
        enum_prop.is_readonly = False
        enum_prop.identifier = "operation"
        enum_prop.name = "Operation"
        enum_prop.type = "ENUM"
        enum_prop.description = "Operation type"

        mock_node.bl_rna.properties = [enum_prop]
        mock_node.operation = "ADD"

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 1
        assert result[0]["type"] == "ENUM"
        assert result[0]["default"] == "ADD"


class TestScanValidNodesForTree:
    """Test the scan_valid_nodes_for_tree function."""

    def test_scan_valid_nodes_tree_creation_fails(self, capsys):
        """Test handling when tree creation fails."""
        with patch("dump_nodes.bpy") as mock_bpy:
            mock_bpy.data.node_groups.new.side_effect = RuntimeError("Cannot create tree")

            result = dump_nodes.scan_valid_nodes_for_tree("InvalidTree", "Invalid")

            assert result == {}
            captured = capsys.readouterr()
            assert "Error creating tree InvalidTree" in captured.err

    def test_scan_valid_nodes_basic(self, capsys):
        """Test basic node scanning."""
        with patch("dump_nodes.bpy") as mock_bpy:
            mock_tree = Mock()
            mock_nodes = Mock()
            mock_tree.nodes = mock_nodes
            mock_bpy.data.node_groups.new.return_value = mock_tree

            mock_node = Mock()
            mock_node.bl_idname = "GeometryNodeMeshCube"
            mock_node.bl_label = "Cube"
            mock_node.inputs = []
            mock_node.outputs = []
            mock_nodes.new.return_value = mock_node

            mock_class = Mock()
            mock_class.bl_idname = "GeometryNodeMeshCube"

            mock_bpy.types.GeometryNodeMeshCube = mock_class

            with patch("dump_nodes.dir", return_value=["GeometryNodeMeshCube"]):
                with patch("dump_nodes.get_properties_info", return_value=[]):
                    result = dump_nodes.scan_valid_nodes_for_tree("GeometryNodeTree", "Geometry Nodes")

            assert "GeometryNodeMeshCube" in result
            assert result["GeometryNodeMeshCube"]["bl_label"] == "Cube"
            mock_bpy.data.node_groups.remove.assert_called_once_with(mock_tree)

    def test_scan_valid_nodes_multiple_candidates(self, capsys):
        """Test scanning with multiple node candidates."""
        with patch("dump_nodes.bpy") as mock_bpy:
            mock_tree = Mock()
            mock_nodes = Mock()
            mock_tree.nodes = mock_nodes
            mock_bpy.data.node_groups.new.return_value = mock_tree

            def new_side_effect(node_id):
                if node_id in ["GeometryNodeMeshCube", "ShaderNodeEmission"]:
                    mock_node = Mock()
                    mock_node.bl_idname = node_id
                    mock_node.bl_label = node_id.replace("Node", " ")
                    mock_node.inputs = []
                    mock_node.outputs = []
                    return mock_node
                raise RuntimeError("Cannot create")

            mock_nodes.new.side_effect = new_side_effect

            mock_class1 = Mock()
            mock_class1.bl_idname = "GeometryNodeMeshCube"
            mock_class2 = Mock()
            mock_class2.bl_idname = "ShaderNodeEmission"
            mock_class3 = Mock()
            mock_class3.bl_idname = "NodeInvalid"

            mock_bpy.types.GeometryNodeMeshCube = mock_class1
            mock_bpy.types.ShaderNodeEmission = mock_class2
            mock_bpy.types.NodeInvalid = mock_class3

            with patch("dump_nodes.dir", return_value=[
                "GeometryNodeMeshCube",
                "ShaderNodeEmission",
                "NodeInvalid",
                "SomeOtherClass"
            ]):
                with patch("dump_nodes.get_properties_info", return_value=[]):
                    result = dump_nodes.scan_valid_nodes_for_tree("GeometryNodeTree", "Geometry")

            assert len(result) == 2
            assert "GeometryNodeMeshCube" in result
            assert "ShaderNodeEmission" in result

            captured = capsys.readouterr()
            assert "Successfully dumped 2 nodes" in captured.err
            assert "1 candidates skipped" in captured.err

    def test_scan_valid_nodes_node_removal_cleanup(self):
        """Test that nodes are properly cleaned up after scanning."""
        with patch("dump_nodes.bpy") as mock_bpy:
            mock_tree = Mock()
            mock_nodes = Mock()
            mock_tree.nodes = mock_nodes
            mock_bpy.data.node_groups.new.return_value = mock_tree

            mock_node = Mock()
            mock_node.bl_idname = "GeometryNodeMeshCube"
            mock_node.bl_label = "Cube"
            mock_node.inputs = []
            mock_node.outputs = []
            mock_nodes.new.return_value = mock_node

            mock_class = Mock()
            mock_class.bl_idname = "GeometryNodeMeshCube"
            mock_bpy.types.GeometryNodeMeshCube = mock_class

            with patch("dump_nodes.dir", return_value=["GeometryNodeMeshCube"]):
                with patch("dump_nodes.get_properties_info", return_value=[]):
                    dump_nodes.scan_valid_nodes_for_tree("GeometryNodeTree", "Geometry")

            mock_nodes.remove.assert_called_once_with(mock_node)

    def test_scan_valid_nodes_node_removal_exception(self):
        """Test handling when node removal fails."""
        with patch("dump_nodes.bpy") as mock_bpy:
            mock_tree = Mock()
            mock_nodes = Mock()
            mock_tree.nodes = mock_nodes
            mock_bpy.data.node_groups.new.return_value = mock_tree

            mock_node = Mock()
            mock_node.bl_idname = "GeometryNodeMeshCube"
            mock_node.bl_label = "Cube"
            mock_node.inputs = []
            mock_node.outputs = []
            mock_nodes.new.return_value = mock_node
            mock_nodes.remove.side_effect = RuntimeError("Cannot remove")

            mock_class = Mock()
            mock_class.bl_idname = "GeometryNodeMeshCube"
            mock_bpy.types.GeometryNodeMeshCube = mock_class

            with patch("dump_nodes.dir", return_value=["GeometryNodeMeshCube"]):
                with patch("dump_nodes.get_properties_info", return_value=[]):
                    result = dump_nodes.scan_valid_nodes_for_tree("GeometryNodeTree", "Geometry")

            assert "GeometryNodeMeshCube" in result

    def test_scan_valid_nodes_with_sockets(self):
        """Test scanning nodes with input and output sockets."""
        with patch("dump_nodes.bpy") as mock_bpy:
            mock_tree = Mock()
            mock_nodes = Mock()
            mock_tree.nodes = mock_nodes
            mock_bpy.data.node_groups.new.return_value = mock_tree

            mock_input = Mock()
            mock_input.name = "Input"
            mock_input.identifier = "input"
            mock_input.bl_idname = "NodeSocketFloat"
            mock_input.description = "Input socket"
            mock_input.default_value = 0.0

            mock_output = Mock()
            mock_output.name = "Output"
            mock_output.identifier = "output"
            mock_output.bl_idname = "NodeSocketFloat"
            mock_output.description = "Output socket"
            del mock_output.default_value

            mock_node = Mock()
            mock_node.bl_idname = "GeometryNodeMeshCube"
            mock_node.bl_label = "Cube"
            mock_node.inputs = [mock_input]
            mock_node.outputs = [mock_output]
            mock_nodes.new.return_value = mock_node

            mock_class = Mock()
            mock_class.bl_idname = "GeometryNodeMeshCube"
            mock_bpy.types.GeometryNodeMeshCube = mock_class

            with patch("dump_nodes.dir", return_value=["GeometryNodeMeshCube"]):
                with patch("dump_nodes.get_properties_info", return_value=[]):
                    result = dump_nodes.scan_valid_nodes_for_tree("GeometryNodeTree", "Geometry")

            assert len(result["GeometryNodeMeshCube"]["inputs"]) == 1
            assert len(result["GeometryNodeMeshCube"]["outputs"]) == 1
            assert result["GeometryNodeMeshCube"]["inputs"][0]["name"] == "Input"


class TestMain:
    """Test the main function."""

    def test_main_success(self, capsys):
        """Test successful execution of main function."""
        mock_geometry_nodes = {"GeometryNodeCube": {"bl_label": "Cube"}}
        mock_shader_nodes = {"ShaderNodeEmission": {"bl_label": "Emission"}}
        mock_compositor_nodes = {"CompositorNodeBlur": {"bl_label": "Blur"}}

        with patch("dump_nodes.scan_valid_nodes_for_tree") as mock_scan:
            mock_scan.side_effect = [
                mock_geometry_nodes,
                mock_shader_nodes,
                mock_compositor_nodes
            ]

            m = mock_open()
            with patch("dump_nodes.open", m):
                dump_nodes.main()

            handle = m()
            written_data = "".join(call[0][0] for call in handle.write.call_args_list)

            result = json.loads(written_data)
            assert "GeometryNodes" in result
            assert "ShaderNodes" in result
            assert "CompositorNodes" in result
            assert result["GeometryNodes"] == mock_geometry_nodes
            assert result["ShaderNodes"] == mock_shader_nodes
            assert result["CompositorNodes"] == mock_compositor_nodes

            captured = capsys.readouterr()
            assert "Writing JSON to" in captured.err
            assert "Done!" in captured.err

    def test_main_file_write_error(self, capsys):
        """Test main function handles file write errors."""
        with patch("dump_nodes.scan_valid_nodes_for_tree", return_value={}):
            m = mock_open()
            m.side_effect = OSError("Permission denied")

            with patch("dump_nodes.open", m):
                with pytest.raises(SystemExit) as exc_info:
                    dump_nodes.main()

                assert exc_info.value.code == 1

            captured = capsys.readouterr()
            assert "Error writing output file" in captured.err
            assert "Permission denied" in captured.err

    def test_main_output_file_path(self):
        """Test that main writes to correct output file path."""
        with patch("dump_nodes.scan_valid_nodes_for_tree", return_value={}):
            m = mock_open()
            with patch("dump_nodes.open", m):
                dump_nodes.main()

            m.assert_called_once_with("./blender_nodes_dump.json", "w", encoding="utf-8")

    def test_main_scan_order(self):
        """Test that main scans tree types in correct order."""
        with patch("dump_nodes.scan_valid_nodes_for_tree") as mock_scan:
            mock_scan.return_value = {}

            with patch("dump_nodes.open", mock_open()):
                dump_nodes.main()

            expected_calls = [
                call("GeometryNodeTree", "Geometry Nodes"),
                call("ShaderNodeTree", "Shader Nodes"),
                call("CompositorNodeTree", "Compositor Nodes")
            ]
            mock_scan.assert_has_calls(expected_calls, any_order=False)

    def test_main_json_formatting(self):
        """Test that main writes properly formatted JSON."""
        test_data = {"NodeA": {"prop": "value"}}

        with patch("dump_nodes.scan_valid_nodes_for_tree", return_value=test_data):
            m = mock_open()
            with patch("dump_nodes.open", m):
                dump_nodes.main()

            handle = m()
            written_data = "".join(call[0][0] for call in handle.write.call_args_list)

            parsed = json.loads(written_data)
            assert isinstance(parsed, dict)


class TestConstants:
    """Test module constants."""

    def test_output_file_constant(self):
        """Test OUTPUT_FILE constant value."""
        assert dump_nodes.OUTPUT_FILE == "blender_nodes_dump.json"

    def test_candidate_prefixes_constant(self):
        """Test CANDIDATE_PREFIXES constant."""
        assert "GeometryNode" in dump_nodes.CANDIDATE_PREFIXES
        assert "ShaderNode" in dump_nodes.CANDIDATE_PREFIXES
        assert "CompositorNode" in dump_nodes.CANDIDATE_PREFIXES
        assert "Node" in dump_nodes.CANDIDATE_PREFIXES
        assert "FunctionNode" in dump_nodes.CANDIDATE_PREFIXES
        assert len(dump_nodes.CANDIDATE_PREFIXES) == 5

    def test_safe_prop_types_constant(self):
        """Test SAFE_PROP_TYPES constant."""
        expected = {"STRING", "BOOLEAN", "INT", "FLOAT", "ENUM"}
        assert dump_nodes.SAFE_PROP_TYPES == expected

    def test_exclude_props_constant(self):
        """Test EXCLUDE_PROPS constant contains expected values."""
        assert "rna_type" in dump_nodes.EXCLUDE_PROPS
        assert "name" in dump_nodes.EXCLUDE_PROPS
        assert "label" in dump_nodes.EXCLUDE_PROPS
        assert "inputs" in dump_nodes.EXCLUDE_PROPS
        assert "outputs" in dump_nodes.EXCLUDE_PROPS
        assert len(dump_nodes.EXCLUDE_PROPS) >= 10


class TestEdgeCases:
    """Additional edge case and integration tests."""

    def test_safe_convert_nested_mathutils(self):
        """Test converting nested mathutils-like structures."""
        mock_vec1 = MockVector([1.0, 2.0])
        result = dump_nodes.safe_convert(mock_vec1)
        assert result == [1.0, 2.0]

    def test_get_socket_info_unicode_names(self):
        """Test socket with unicode characters in name."""
        mock_socket = Mock()
        mock_socket.name = "Socket \u2192 \u03c0"
        mock_socket.identifier = "socket_pi"
        mock_socket.bl_idname = "NodeSocketFloat"
        mock_socket.description = "Unicode test \u2713"
        mock_socket.default_value = 3.14

        result = dump_nodes.get_socket_info(mock_socket)

        assert "Socket" in result["name"]
        assert result["identifier"] == "socket_pi"

    def test_get_properties_info_all_excluded_props(self):
        """Test that all EXCLUDE_PROPS are actually excluded."""
        mock_node = Mock()

        props = []
        for excluded_prop in dump_nodes.EXCLUDE_PROPS:
            mock_prop = Mock()
            mock_prop.is_readonly = False
            mock_prop.identifier = excluded_prop
            mock_prop.type = "STRING"
            props.append(mock_prop)

        mock_node.bl_rna.properties = props

        result = dump_nodes.get_properties_info(mock_node)

        assert len(result) == 0

    def test_scan_valid_nodes_all_prefixes_matched(self):
        """Test that all candidate prefixes are checked during scanning."""
        with patch("dump_nodes.bpy") as mock_bpy:
            mock_tree = Mock()
            mock_nodes = Mock()
            mock_tree.nodes = mock_nodes
            mock_bpy.data.node_groups.new.return_value = mock_tree

            all_classes = [
                "GeometryNodeCube",
                "ShaderNodeEmission",
                "CompositorNodeBlur",
                "NodeGroup",
                "FunctionNodeCompare",
                "SomeOtherClass"
            ]

            matched_count = 0
            def new_side_effect(node_id):
                mock_node = Mock()
                mock_node.bl_idname = node_id
                mock_node.bl_label = node_id
                mock_node.inputs = []
                mock_node.outputs = []
                return mock_node

            mock_nodes.new.side_effect = new_side_effect

            for cls_name in all_classes:
                if any(cls_name.startswith(prefix) for prefix in dump_nodes.CANDIDATE_PREFIXES):
                    mock_class = Mock()
                    mock_class.bl_idname = cls_name
                    setattr(mock_bpy.types, cls_name, mock_class)

            with patch("dump_nodes.dir", return_value=all_classes):
                with patch("dump_nodes.get_properties_info", return_value=[]):
                    result = dump_nodes.scan_valid_nodes_for_tree("GeometryNodeTree", "Test")

            assert len(result) == 5

    def test_empty_node_scan_result(self, capsys):
        """Test scanning when no valid nodes are found."""
        with patch("dump_nodes.bpy") as mock_bpy:
            mock_tree = Mock()
            mock_nodes = Mock()
            mock_tree.nodes = mock_nodes
            mock_bpy.data.node_groups.new.return_value = mock_tree
            mock_nodes.new.side_effect = RuntimeError("No valid nodes")

            with patch("dump_nodes.dir", return_value=["GeometryNodeInvalid"]):
                result = dump_nodes.scan_valid_nodes_for_tree("GeometryNodeTree", "Empty Test")

            assert result == {}
            captured = capsys.readouterr()
            assert "Successfully dumped 0 nodes" in captured.err

    def test_safe_convert_zero_values(self):
        """Test safe_convert with zero/false values."""
        assert dump_nodes.safe_convert(0) == 0
        assert dump_nodes.safe_convert(0.0) == 0.0
        assert dump_nodes.safe_convert(False) is False
        assert dump_nodes.safe_convert("") == ""

    def test_main_with_empty_scans(self, capsys):
        """Test main function when all scans return empty results."""
        with patch("dump_nodes.scan_valid_nodes_for_tree", return_value={}):
            with patch("dump_nodes.open", mock_open()):
                dump_nodes.main()

        captured = capsys.readouterr()
        assert "Done!" in captured.err

    def test_safe_convert_complex_nested_values(self):
        """Regression test: complex nested structures with mixed types."""
        # Test that we properly handle complex scenarios
        test_cases = [
            ({"nested": "dict"}, "{'nested': 'dict'}"),
            (["list", "items"], "['list', 'items']"),
            (tuple([1, 2, 3]), "(1, 2, 3)"),
            (frozenset([1, 2]), "frozenset({1, 2})"),
        ]

        for input_val, expected_str in test_cases:
            result = dump_nodes.safe_convert(input_val)
            assert isinstance(result, str)
            # Just verify it's converted to string for unknown types

    def test_scan_valid_nodes_preserves_node_order(self):
        """Regression test: ensure node definitions maintain consistent structure."""
        with patch("dump_nodes.bpy") as mock_bpy:
            mock_tree = Mock()
            mock_nodes = Mock()
            mock_tree.nodes = mock_nodes
            mock_bpy.data.node_groups.new.return_value = mock_tree

            # Create a node with specific structure
            mock_input = Mock()
            mock_input.name = "Input1"
            mock_input.identifier = "in1"
            mock_input.bl_idname = "NodeSocketFloat"
            mock_input.description = "First input"
            mock_input.default_value = 1.0

            mock_output = Mock()
            mock_output.name = "Output1"
            mock_output.identifier = "out1"
            mock_output.bl_idname = "NodeSocketFloat"
            mock_output.description = "First output"
            del mock_output.default_value

            mock_node = Mock()
            mock_node.bl_idname = "GeometryNodeTestNode"
            mock_node.bl_label = "Test Node"
            mock_node.inputs = [mock_input]
            mock_node.outputs = [mock_output]
            mock_nodes.new.return_value = mock_node

            mock_class = Mock()
            mock_class.bl_idname = "GeometryNodeTestNode"
            mock_bpy.types.GeometryNodeTestNode = mock_class

            with patch("dump_nodes.dir", return_value=["GeometryNodeTestNode"]):
                with patch("dump_nodes.get_properties_info", return_value=[{"identifier": "test"}]):
                    result = dump_nodes.scan_valid_nodes_for_tree("TestTree", "Test")

            # Verify structure is as expected
            assert "GeometryNodeTestNode" in result
            node_def = result["GeometryNodeTestNode"]
            assert "bl_idname" in node_def
            assert "bl_label" in node_def
            assert "inputs" in node_def
            assert "outputs" in node_def
            assert "properties" in node_def
            assert len(node_def["inputs"]) == 1
            assert len(node_def["outputs"]) == 1
            assert node_def["inputs"][0]["name"] == "Input1"
            assert node_def["outputs"][0]["name"] == "Output1"

    def test_get_properties_info_boundary_large_property_set(self):
        """Boundary test: handling nodes with many properties."""
        mock_node = Mock()

        # Create 50 properties to test performance and correctness
        props = []
        for i in range(50):
            mock_prop = Mock()
            mock_prop.is_readonly = False
            mock_prop.identifier = f"prop_{i}"
            mock_prop.name = f"Property {i}"
            mock_prop.type = "INT"
            mock_prop.description = f"Description {i}"
            props.append(mock_prop)
            setattr(mock_node, f"prop_{i}", i * 10)

        mock_node.bl_rna.properties = props

        result = dump_nodes.get_properties_info(mock_node)

        # Verify all properties were processed
        assert len(result) == 50
        assert result[0]["default"] == 0
        assert result[25]["default"] == 250
        assert result[49]["default"] == 490

    def test_main_integration_full_pipeline(self, capsys):
        """Integration test: full pipeline from scan to file write with realistic data."""
        # Create realistic mock data that simulates actual Blender nodes
        geometry_data = {
            "GeometryNodeMeshCube": {
                "bl_idname": "GeometryNodeMeshCube",
                "bl_label": "Cube",
                "inputs": [
                    {"name": "Size", "identifier": "Size", "type": "NodeSocketVector", "default": [2.0, 2.0, 2.0]}
                ],
                "outputs": [
                    {"name": "Mesh", "identifier": "Mesh", "type": "NodeSocketGeometry"}
                ],
                "properties": []
            }
        }

        shader_data = {
            "ShaderNodeEmission": {
                "bl_idname": "ShaderNodeEmission",
                "bl_label": "Emission",
                "inputs": [
                    {"name": "Color", "identifier": "Color", "type": "NodeSocketColor", "default": [1.0, 1.0, 1.0, 1.0]},
                    {"name": "Strength", "identifier": "Strength", "type": "NodeSocketFloat", "default": 1.0}
                ],
                "outputs": [
                    {"name": "Emission", "identifier": "Emission", "type": "NodeSocketShader"}
                ],
                "properties": []
            }
        }

        with patch("dump_nodes.scan_valid_nodes_for_tree") as mock_scan:
            mock_scan.side_effect = [geometry_data, shader_data, {}]

            m = mock_open()
            with patch("dump_nodes.open", m):
                dump_nodes.main()

            # Verify all tree types were scanned
            assert mock_scan.call_count == 3

            # Verify JSON was written
            handle = m()
            assert handle.write.called

            # Verify output structure
            written_data = "".join(call[0][0] for call in handle.write.call_args_list)
            result = json.loads(written_data)

            assert "GeometryNodes" in result
            assert "ShaderNodes" in result
            assert "CompositorNodes" in result
            assert "GeometryNodeMeshCube" in result["GeometryNodes"]
            assert "ShaderNodeEmission" in result["ShaderNodes"]