use crate::core::context::{enter_zone, exit_zone, take_root_nodes};
use std::fmt::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeType {
    Geometry,
    Shader,
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
for node in tree.nodes:
    tree.nodes.remove(node)
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

if not bpy.context.object:
    bpy.ops.mesh.primitive_cube_add()
obj = bpy.context.object

mod_name = 'RustNodes'
if obj.modifiers.get(mod_name):
    obj.modifiers.remove(obj.modifiers.get(mod_name))
mod = obj.modifiers.new(name=mod_name, type='NODES')
mod.node_group = group
tree = group

if not tree.interface.items_tree:
    tree.interface.new_socket('Geometry', in_out='OUTPUT', socket_type='NodeSocketGeometry')
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
        enter_zone();
        body();
        let my_nodes = exit_zone();

        let mut code = self.generate_setup_script();

        code.push_str("\n# --- Node Creation Phase ---\n");
        for node in &my_nodes {
            code.push_str(&node.creation_script());
        }

        code.push_str("\n# --- Node Linking Phase ---\n");
        for node in &my_nodes {
            code.push_str(&node.links_script());
        }

        code
    }
}

pub fn generate_script_header() -> String {
    "import bpy\nimport math\n".to_string()
}
