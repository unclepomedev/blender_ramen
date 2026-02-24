use std::collections::HashMap;
use std::fmt::Write;
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Debug)]
pub struct NodeData {
    pub name: String,
    pub bl_idname: String,
    pub properties: HashMap<String, String>,
    pub inputs: HashMap<usize, (String, bool)>,
    pub output_defaults: HashMap<usize, String>,
    pub post_creation_script: String,
    pub custom_links_script: String,
}

impl NodeData {
    pub fn new(name: String, bl_idname: String) -> Self {
        Self {
            name,
            bl_idname,
            properties: HashMap::new(),
            inputs: HashMap::new(),
            output_defaults: HashMap::new(),
            post_creation_script: String::new(),
            custom_links_script: String::new(),
        }
    }

    pub fn creation_script(&self) -> String {
        if self.bl_idname.is_empty() {
            return String::new();
        }

        let mut code = String::new();
        let _ = writeln!(
            &mut code,
            "{} = tree.nodes.new('{}')",
            self.name, self.bl_idname
        );

        for (k, v) in &self.properties {
            let _ = writeln!(&mut code, "{}.{} = {}", self.name, k, v);
        }

        for (idx, (expr, is_literal)) in &self.inputs {
            if *is_literal {
                let _ = writeln!(
                    &mut code,
                    "{}.inputs[{}].default_value = {}",
                    self.name, idx, expr
                );
            }
        }

        for (idx, expr) in &self.output_defaults {
            let _ = writeln!(
                &mut code,
                "{}.outputs[{}].default_value = {}",
                self.name, idx, expr
            );
        }

        code.push_str(&self.post_creation_script);
        code
    }

    pub fn links_script(&self) -> String {
        if self.bl_idname.is_empty() {
            return String::new();
        }

        let mut code = String::new();
        for (idx, (expr, is_literal)) in &self.inputs {
            if !*is_literal {
                let _ = writeln!(
                    &mut code,
                    "tree.links.new({}, {}.inputs[{}])",
                    expr, self.name, idx
                );
            }
        }

        code.push_str(&self.custom_links_script);
        code
    }
}

pub type Scope = Vec<NodeData>;

pub struct BuildContext {
    nodes: HashMap<String, NodeData>,
    stack: Vec<Vec<String>>,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            stack: vec![Vec::new()],
        }
    }

    pub fn add_node(&mut self, data: NodeData) {
        let name = data.name.clone();
        self.nodes.insert(name.clone(), data);

        if let Some(current_scope) = self.stack.last_mut() {
            current_scope.push(name);
        }
    }

    pub fn update_property(&mut self, name: &str, key: &str, val: String) {
        if let Some(node) = self.nodes.get_mut(name) {
            node.properties.insert(key.to_string(), val);
        }
    }

    pub fn update_input(&mut self, name: &str, index: usize, val: String, is_literal: bool) {
        if let Some(node) = self.nodes.get_mut(name) {
            node.inputs.insert(index, (val, is_literal));
        }
    }

    pub fn update_output_default(&mut self, name: &str, index: usize, val: String) {
        if let Some(node) = self.nodes.get_mut(name) {
            node.output_defaults.insert(index, val);
        }
    }

    pub fn update_post_creation(&mut self, name: &str, script: String) {
        if let Some(node) = self.nodes.get_mut(name) {
            node.post_creation_script = script;
        }
    }

    pub fn append_custom_link(&mut self, name: &str, script: String) {
        if let Some(node) = self.nodes.get_mut(name) {
            node.custom_links_script.push_str(&script);
        }
    }

    pub fn enter_scope(&mut self) {
        self.stack.push(Vec::new());
    }

    pub fn exit_scope(&mut self) -> Scope {
        if self.stack.len() > 1 {
            let scope_names = self.stack.pop().unwrap();
            scope_names
                .into_iter()
                .filter_map(|name| self.nodes.remove(&name))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn take_root(&mut self) -> Scope {
        let root_names = std::mem::take(&mut self.stack[0]);
        root_names
            .into_iter()
            .filter_map(|name| self.nodes.remove(&name))
            .collect()
    }
}

/// **[WARNING: Logical Thread Safety]**
///
/// `GLOBAL_CONTEXT` utilizes a `Mutex` to prevent memory corruption (data races),
/// making it strictly memory-safe. However, it is **logically thread-unsafe**.
///
/// Because node generation relies on a single shared state (like a global whiteboard),
/// if multiple threads attempt to generate node trees or enter/exit zones concurrently,
/// their operations will interleave. For example, Thread B might inject a node into
/// Thread A's active scope, or Thread A might steal Thread B's nodes upon `exit_zone()`.
///
/// **Constraints:**
/// - Node generation must be strictly **single-threaded** and sequential.
/// - Do not use `rayon` or concurrent `tokio` tasks to build multiple node trees at once.
///
/// **Future Architecture Note:**
/// To make this library fully thread-safe for highly concurrent environments (e.g., a Web API),
/// we should either migrate this to `thread_local!` or refactor the API to explicitly pass
/// a `&mut BuildContext` around instead of relying on hidden global state.
pub static GLOBAL_CONTEXT: LazyLock<Mutex<BuildContext>> =
    LazyLock::new(|| Mutex::new(BuildContext::new()));

pub fn add_node(data: NodeData) {
    GLOBAL_CONTEXT.lock().unwrap().add_node(data);
}
pub fn update_property(name: &str, key: &str, val: String) {
    GLOBAL_CONTEXT
        .lock()
        .unwrap()
        .update_property(name, key, val);
}
pub fn update_input(name: &str, index: usize, val: String, is_literal: bool) {
    GLOBAL_CONTEXT
        .lock()
        .unwrap()
        .update_input(name, index, val, is_literal);
}
pub fn update_output_default(name: &str, index: usize, val: String) {
    GLOBAL_CONTEXT
        .lock()
        .unwrap()
        .update_output_default(name, index, val);
}
pub fn update_post_creation(name: &str, script: String) {
    GLOBAL_CONTEXT
        .lock()
        .unwrap()
        .update_post_creation(name, script);
}

pub fn append_custom_link(name: &str, script: String) {
    GLOBAL_CONTEXT
        .lock()
        .unwrap()
        .append_custom_link(name, script);
}

pub fn enter_zone() {
    GLOBAL_CONTEXT.lock().unwrap().enter_scope();
}
pub fn exit_zone() -> Scope {
    GLOBAL_CONTEXT.lock().unwrap().exit_scope()
}
pub fn take_root_nodes() -> Scope {
    GLOBAL_CONTEXT.lock().unwrap().take_root()
}

// ---------------------------------------------------------
// unittest
// ---------------------------------------------------------
#[cfg(test)]
pub mod test_utils {
    use std::sync::{LazyLock, Mutex};
    pub static GLOBAL_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_data_creation_script() {
        let mut node = NodeData::new("math_1".to_string(), "ShaderNodeMath".to_string());

        node.properties
            .insert("operation".to_string(), "'ADD'".to_string());
        node.inputs.insert(0, ("1.5".to_string(), true));
        node.inputs
            .insert(1, ("other_node.outputs['Value']".to_string(), false));
        node.output_defaults.insert(0, "0.0".to_string());

        let script = node.creation_script();

        assert!(script.contains("math_1 = tree.nodes.new('ShaderNodeMath')"));
        assert!(script.contains("math_1.operation = 'ADD'"));
        assert!(script.contains("math_1.inputs[0].default_value = 1.5"));
        assert!(!script.contains("math_1.inputs[1].default_value = other_node.outputs"));
        assert!(script.contains("math_1.outputs[0].default_value = 0.0"));
    }

    #[test]
    fn test_node_data_links_script() {
        let mut node = NodeData::new("math_1".to_string(), "ShaderNodeMath".to_string());

        node.inputs.insert(0, ("1.5".to_string(), true));
        node.inputs
            .insert(1, ("other_node.outputs['Value']".to_string(), false));

        let script = node.links_script();

        assert!(script.contains("tree.links.new(other_node.outputs['Value'], math_1.inputs[1])"));
        assert!(!script.contains("1.5"));
    }

    #[test]
    fn test_build_context_updates() {
        let mut ctx = BuildContext::new();
        let node = NodeData::new("test_node".to_string(), "TestNodeType".to_string());

        ctx.add_node(node);

        ctx.update_property("test_node", "prop1", "100".to_string());
        ctx.update_input("test_node", 2, "200".to_string(), true);

        let root_nodes = ctx.take_root();
        assert_eq!(root_nodes.len(), 1);

        let extracted_node = &root_nodes[0];
        assert_eq!(extracted_node.properties.get("prop1").unwrap(), "100");
        assert_eq!(extracted_node.inputs.get(&2).unwrap().0, "200");
    }

    #[test]
    fn test_scope_management() {
        let mut ctx = BuildContext::new();

        ctx.add_node(NodeData::new("node_A".to_string(), "TypeA".to_string()));

        ctx.enter_scope();
        ctx.add_node(NodeData::new("node_B".to_string(), "TypeB".to_string()));

        let sub_nodes = ctx.exit_scope();
        assert_eq!(sub_nodes.len(), 1);
        assert_eq!(sub_nodes[0].name, "node_B");

        let root_nodes = ctx.take_root();
        assert_eq!(root_nodes.len(), 1);
        assert_eq!(root_nodes[0].name, "node_A");
    }

    #[test]
    fn test_scope_safety_guard() {
        let mut ctx = BuildContext::new();

        ctx.add_node(NodeData::new("root_node".to_string(), "Root".to_string()));

        let empty_nodes = ctx.exit_scope();

        assert_eq!(empty_nodes.len(), 0);

        let root_nodes = ctx.take_root();
        assert_eq!(root_nodes.len(), 1);
        assert_eq!(root_nodes[0].name, "root_node");
    }
}
