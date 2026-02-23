use crate::core::context::{enter_zone, exit_zone};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeType {
    Geometry,
    Shader,
    GeometryGroup,
    ShaderGroup,
}

pub struct NodeTree {
    name: String,
    tree_type: TreeType,
}

impl NodeTree {
    pub fn new_geometry(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::Geometry,
        }
    }

    pub fn new_shader(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::Shader,
        }
    }

    pub fn new_geometry_group(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::GeometryGroup,
        }
    }

    pub fn new_shader_group(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tree_type: TreeType::ShaderGroup,
        }
    }

    fn generate_setup_script(&self) -> String {
        let mut code = String::new();
        match self.tree_type {
            TreeType::Shader => {
                let _ = write!(
                    &mut code,
                    r#"
# --- Setup Shader: {name} ---
mat = bpy.data.materials.get('{name}')
if not mat:
    mat = bpy.data.materials.new(name='{name}')
tree = mat.node_tree
tree.nodes.clear()
"#,
                    name = self.name
                );
            }
            TreeType::Geometry => {
                let _ = write!(
                    &mut code,
                    r#"
# --- Setup GeoNodes: {name} ---
tree_name = '{name}'
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
                    name = self.name
                );
            }
            TreeType::GeometryGroup => {
                let _ = write!(
                    &mut code,
                    r#"
# --- Setup GeoNodes Group: {name} ---
tree_name = '{name}'
if tree_name in bpy.data.node_groups:
    bpy.data.node_groups.remove(bpy.data.node_groups[tree_name])
tree = bpy.data.node_groups.new(name=tree_name, type='GeometryNodeTree')
"#,
                    name = self.name
                );
            }
            TreeType::ShaderGroup => {
                let _ = write!(
                    &mut code,
                    r#"
# --- Setup Shader Group: {name} ---
tree_name = '{name}'
if tree_name in bpy.data.node_groups:
    bpy.data.node_groups.remove(bpy.data.node_groups[tree_name])
tree = bpy.data.node_groups.new(name=tree_name, type='ShaderNodeTree')
"#,
                    name = self.name
                );
            }
        }
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
    crate::core::context::update_post_creation(
        &node.name,
        format!(
            "{}.node_tree = bpy.data.node_groups['{}']\n",
            node.name, group_name
        ),
    );
    node
}

/// call and instantiate shader node groups
pub fn call_shader_group(group_name: &str) -> crate::core::nodes::ShaderNodeGroup {
    let node = crate::core::nodes::ShaderNodeGroup::new();
    crate::core::context::update_post_creation(
        &node.name,
        format!(
            "{}.node_tree = bpy.data.node_groups['{}']\n",
            node.name, group_name
        ),
    );
    node
}
