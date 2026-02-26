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

pub struct NodeTree {
    name: String,
    tree_type: TreeType,
    inputs: Vec<(String, String)>,
    outputs: Vec<(String, String)>,
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
        self.inputs
            .push((name.to_string(), S::blender_socket_type().to_string()));
        self
    }

    pub fn with_output<S: SocketDef>(mut self, name: &str) -> Self {
        assert!(
            self.tree_type == TreeType::GeometryGroup
                || self.tree_type == TreeType::ShaderGroup
                || self.tree_type == TreeType::CompositorGroup,
            "with_output can only be used on Group Node Trees!"
        );
        self.outputs
            .push((name.to_string(), S::blender_socket_type().to_string()));
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

mod_name = 'RustNodes'
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
"#,
            name = self.name,
            safe_name = safe_name
        )
    }

    fn append_sockets(&self, code: &mut String) {
        for (name, s_type) in &self.inputs {
            let safe_name = python_string_literal(name);
            let _ = writeln!(
                code,
                "tree.interface.new_socket({}, in_out='INPUT', socket_type='{}')",
                safe_name, s_type
            );
        }
        for (name, s_type) in &self.outputs {
            let safe_name = python_string_literal(name);
            let _ = writeln!(
                code,
                "tree.interface.new_socket({}, in_out='OUTPUT', socket_type='{}')",
                safe_name, s_type
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
