use crate::core::context::{enter_zone, exit_zone};
use crate::core::types::{SocketDef, python_string_literal};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeType {
    Geometry,
    Shader,
    GeometryGroup,
    ShaderGroup,
    Compositor,
    CompositorGroup,
}

pub struct TreeInput {
    pub name: String,
    pub blender_type: String,
    pub default_expr: Option<String>,
}

pub struct TreeOutput {
    pub name: String,
    pub blender_type: String,
}

pub struct NodeTree {
    name: String,
    tree_type: TreeType,
    inputs: Vec<TreeInput>,
    outputs: Vec<TreeOutput>,
}

impl NodeTree {
    pub fn new_geometry(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::Geometry,
            inputs: vec![],
            outputs: vec![],
        }
    }

    pub fn new_shader(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::Shader,
            inputs: vec![],
            outputs: vec![],
        }
    }

    pub fn new_geometry_group(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::GeometryGroup,
            inputs: vec![],
            outputs: vec![],
        }
    }

    pub fn new_shader_group(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::ShaderGroup,
            inputs: vec![],
            outputs: vec![],
        }
    }

    pub fn new_compositor(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::Compositor,
            inputs: vec![],
            outputs: vec![],
        }
    }

    pub fn new_compositor_group(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::CompositorGroup,
            inputs: vec![],
            outputs: vec![],
        }
    }

    pub fn with_input<S: SocketDef>(mut self, name: &str) -> Self {
        assert!(
            self.tree_type == TreeType::GeometryGroup
                || self.tree_type == TreeType::ShaderGroup
                || self.tree_type == TreeType::CompositorGroup,
            "with_input can only be used on Group Node Trees!"
        );
        self.inputs.push(TreeInput {
            name: name.to_string(),
            blender_type: S::blender_socket_type().to_string(),
            default_expr: None,
        });
        self
    }

    pub fn with_input_default<S: SocketDef>(
        mut self,
        name: &str,
        default_val: impl Into<crate::core::types::NodeSocket<S>>,
    ) -> Self {
        assert!(
            self.tree_type == TreeType::GeometryGroup
                || self.tree_type == TreeType::ShaderGroup
                || self.tree_type == TreeType::CompositorGroup,
            "with_input_default can only be used on Group Node Trees!"
        );
        let socket = default_val.into();
        assert!(
            socket.is_literal,
            "with_input_default expects a literal value, not a linked socket expression"
        );
        self.inputs.push(TreeInput {
            name: name.to_string(),
            blender_type: S::blender_socket_type().to_string(),
            default_expr: Some(socket.python_expr()),
        });
        self
    }

    pub fn with_output<S: SocketDef>(mut self, name: &str) -> Self {
        assert!(
            self.tree_type == TreeType::GeometryGroup
                || self.tree_type == TreeType::ShaderGroup
                || self.tree_type == TreeType::CompositorGroup,
            "with_output can only be used on Group Node Trees!"
        );
        self.outputs.push(TreeOutput {
            name: name.to_string(),
            blender_type: S::blender_socket_type().to_string(),
        });
        self
    }

    fn setup_shader(&self) -> String {
        let safe_name = python_string_literal(&self.name);
        format!(
            r#"
# --- Setup Shader: {name} ---
mat = bpy.data.materials.get({safe_name})
if not mat:
    mat = bpy.data.materials.new(name={safe_name})
tree = mat.node_tree
tree.nodes.clear()
"#,
            name = self.name,
            safe_name = safe_name
        )
    }

    fn setup_geometry(&self) -> String {
        let safe_name = python_string_literal(&self.name);
        format!(
            r#"
# --- Setup GeoNodes: {name} ---
tree_name = {safe_name}
if tree_name in bpy.data.node_groups:
    bpy.data.node_groups.remove(bpy.data.node_groups[tree_name])
group = bpy.data.node_groups.new(name=tree_name, type='GeometryNodeTree')

obj = bpy.context.object
if not obj:
    raise RuntimeError("No active object in scene; please select an object to attach the GeoNodes modifier.")

mod_name = 'RamenNodes'
existing_mod = obj.modifiers.get(mod_name)
if existing_mod:
    obj.modifiers.remove(existing_mod)

mod = obj.modifiers.new(name=mod_name, type='NODES')
mod.node_group = group
tree = group

tree.interface.new_socket('Geometry', in_out='OUTPUT', socket_type='NodeSocketGeometry')
"#,
            name = self.name,
            safe_name = safe_name
        )
    }

    fn setup_group(&self, label: &str, tree_type_id: &str) -> String {
        let safe_name = python_string_literal(&self.name);
        format!(
            r#"
# --- Setup {label}: {name} ---
tree_name = {safe_name}
if tree_name in bpy.data.node_groups:
    bpy.data.node_groups.remove(bpy.data.node_groups[tree_name])
tree = bpy.data.node_groups.new(name=tree_name, type='{tree_type_id}')
"#,
            label = label,
            name = self.name,
            safe_name = safe_name,
            tree_type_id = tree_type_id
        )
    }

    fn setup_compositor(&self) -> String {
        let safe_name = python_string_literal(&self.name);
        format!(
            r#"
# --- Setup Compositor: {name} ---
scene = bpy.context.scene
tree = getattr(scene, 'compositing_node_group', None)
if tree is None or tree.name != {safe_name}:
    scene.compositing_node_group = bpy.data.node_groups.new(name={safe_name}, type='CompositorNodeTree')
    tree = scene.compositing_node_group
tree.nodes.clear()

tree.interface.clear()
tree.interface.new_socket('Image', in_out='OUTPUT', socket_type='NodeSocketColor')
tree.interface.new_socket('Alpha', in_out='OUTPUT', socket_type='NodeSocketFloat')
"#,
            name = self.name,
            safe_name = safe_name
        )
    }

    fn append_sockets(&self, code: &mut String) {
        for input in &self.inputs {
            let safe_name = python_string_literal(&input.name);
            let _ = writeln!(
                code,
                "sock = tree.interface.new_socket({}, in_out='INPUT', socket_type='{}')",
                safe_name, input.blender_type
            );

            if let Some(expr) = &input.default_expr {
                let _ = writeln!(code, "sock.default_value = {}", expr);
            }
        }
        for output in &self.outputs {
            let safe_name = python_string_literal(&output.name);
            let _ = writeln!(
                code,
                "tree.interface.new_socket({}, in_out='OUTPUT', socket_type='{}')",
                safe_name, output.blender_type
            );
        }
    }

    fn generate_setup_script(&self) -> String {
        let mut code = match self.tree_type {
            TreeType::Shader => self.setup_shader(),
            TreeType::Geometry => self.setup_geometry(),
            TreeType::GeometryGroup => self.setup_group("GeoNodes Group", "GeometryNodeTree"),
            TreeType::ShaderGroup => self.setup_group("Shader Group", "ShaderNodeTree"),
            TreeType::Compositor => self.setup_compositor(),
            TreeType::CompositorGroup => self.setup_group("Compositor Group", "CompositorNodeTree"),
        };

        self.append_sockets(&mut code);
        code
    }

    pub fn build<F>(&self, body: F) -> String
    where
        F: FnOnce(),
    {
        struct PanicGuard {
            is_panicking: bool,
        }

        impl Drop for PanicGuard {
            fn drop(&mut self) {
                if self.is_panicking {
                    let _ = exit_zone();
                }
            }
        }

        enter_zone();
        let mut guard = PanicGuard { is_panicking: true };
        body();
        guard.is_panicking = false;
        let my_nodes = exit_zone();

        let mut code = self.generate_setup_script();

        code.push_str("\n# --- Node Creation Phase ---\n");
        for node in &my_nodes {
            code.push_str(&node.creation_script());
        }

        // For calling custom groups, etc
        code.push_str("\n# --- Node Post Creation Phase ---\n");
        for node in &my_nodes {
            if !node.post_creation_script.is_empty() {
                code.push_str(&node.post_creation_script);
                code.push('\n');
            }
        }

        code.push_str("\n# --- Node Linking Phase ---\n");
        for node in &my_nodes {
            code.push_str(&node.links_script());
        }

        code
    }
}

pub fn generate_script_header() -> String {
    "import bpy\n".to_string()
}

/// call and instantiate geometry node groups
pub fn call_geometry_group(group_name: &str) -> crate::core::nodes::GeometryNodeGroup {
    let node = crate::core::nodes::GeometryNodeGroup::new();
    crate::core::context::update_property(
        &node.name,
        "node_tree",
        format!(
            "bpy.data.node_groups[{}]",
            python_string_literal(group_name)
        ),
    );
    node
}

/// call and instantiate shader node groups
pub fn call_shader_group(group_name: &str) -> crate::core::nodes::ShaderNodeGroup {
    let node = crate::core::nodes::ShaderNodeGroup::new();
    crate::core::context::update_property(
        &node.name,
        "node_tree",
        format!(
            "bpy.data.node_groups[{}]",
            python_string_literal(group_name)
        ),
    );
    node
}

// ---------------------------------------------------------
// unittest
// ---------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::{Float, Geo, Object};

    #[test]
    fn test_tree_io_definitions() {
        let tree = NodeTree::new_geometry_group("TestGroup")
            .with_input::<Float>("Scale")
            .with_input_default::<Object>("Target", "Cube")
            .with_output::<Geo>("OutGeo");

        assert_eq!(tree.inputs.len(), 2);
        assert_eq!(tree.outputs.len(), 1);

        assert_eq!(tree.inputs[0].name, "Scale");
        assert_eq!(tree.inputs[0].blender_type, "NodeSocketFloat");
        assert_eq!(tree.inputs[0].default_expr, None);

        assert_eq!(tree.inputs[1].name, "Target");
        assert_eq!(tree.inputs[1].blender_type, "NodeSocketObject");
        assert_eq!(
            tree.inputs[1].default_expr.as_deref(),
            Some("bpy.data.objects.get(\"Cube\")")
        );

        assert_eq!(tree.outputs[0].name, "OutGeo");
        assert_eq!(tree.outputs[0].blender_type, "NodeSocketGeometry");
    }

    #[test]
    fn test_append_sockets_script() {
        let tree = NodeTree::new_geometry_group("ScriptGroup")
            .with_input_default::<Float>("Threshold", 0.75)
            .with_output::<Geo>("Geometry");

        let mut code = String::new();
        tree.append_sockets(&mut code);

        assert!(
            code.contains("sock = tree.interface.new_socket(\"Threshold\", in_out='INPUT', socket_type='NodeSocketFloat')"),
            "Input socket creation script is missing or incorrect."
        );
        assert!(
            code.contains("sock.default_value = 0.7500"),
            "Default value assignment script is missing or incorrect."
        );

        assert!(
            code.contains("tree.interface.new_socket(\"Geometry\", in_out='OUTPUT', socket_type='NodeSocketGeometry')"),
            "Output socket creation script is missing or incorrect."
        );
    }
}
