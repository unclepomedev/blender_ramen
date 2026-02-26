use crate::core::live_link::send_to_blender;
use crate::core::tree::{NodeTree, generate_script_header};

pub struct BlenderProject {
    script: String,
}

impl Default for BlenderProject {
    fn default() -> Self {
        Self::new()
    }
}

impl BlenderProject {
    pub fn new() -> Self {
        Self {
            script: generate_script_header(),
        }
    }

    pub fn add_shader_tree<F>(mut self, tree_name: &str, builder: F) -> Self
    where
        F: FnOnce(),
    {
        let script = NodeTree::new_shader(tree_name).build(builder);
        self.script.push_str(&script);
        self
    }

    pub fn add_geometry_tree<F>(mut self, tree_name: &str, builder: F) -> Self
    where
        F: FnOnce(),
    {
        let script = NodeTree::new_geometry(tree_name).build(builder);
        self.script.push_str(&script);
        self
    }

    pub fn add_compositor(mut self, script: &str) -> Self {
        // TODO: setup compositor tree
        self.script.push_str(script);
        self
    }

    pub fn add_script(mut self, script: &str) -> Self {
        self.script.push_str(script);
        self
    }

    pub fn send(&self) {
        #[cfg(debug_assertions)]
        eprintln!("{}", self.script);
        send_to_blender(&self.script);
    }
}
