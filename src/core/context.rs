use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Clone, Debug)]
pub struct NodeData {
    pub name: String,
    pub bl_idname: String,
    pub properties: HashMap<String, String>,
    pub inputs: HashMap<usize, String>,
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
        if !self.bl_idname.is_empty() {
            let mut code = format!("{} = tree.nodes.new('{}')\n", self.name, self.bl_idname);
            for (k, v) in &self.properties {
                code.push_str(&format!("{}.{} = {}\n", self.name, k, v));
            }
            for (idx, expr) in &self.inputs {
                if !expr.contains(".outputs[") {
                    code.push_str(&format!(
                        "{}.inputs[{}].default_value = {}\n",
                        self.name, idx, expr
                    ));
                }
            }
            for (idx, expr) in &self.output_defaults {
                code.push_str(&format!(
                    "{}.outputs[{}].default_value = {}\n",
                    self.name, idx, expr
                ));
            }
            code.push_str(&self.post_creation_script);
            code
        } else {
            String::new()
        }
    }

    pub fn links_script(&self) -> String {
        let mut code = String::new();
        if !self.bl_idname.is_empty() {
            for (idx, expr) in &self.inputs {
                if expr.contains(".outputs[") {
                    code.push_str(&format!(
                        "tree.links.new({}, {}.inputs[{}])\n",
                        expr, self.name, idx
                    ));
                }
            }
        }
        code.push_str(&self.custom_links_script);
        code
    }
}

pub type Scope = Vec<NodeData>;

pub struct BuildContext {
    stack: Vec<Scope>,
}

impl BuildContext {
    fn new() -> Self {
        Self {
            stack: vec![Vec::new()],
        }
    }
    pub fn add_node(&mut self, data: NodeData) {
        self.stack.last_mut().unwrap().push(data);
    }

    pub fn update_property(&mut self, name: &str, key: &str, val: String) {
        for scope in self.stack.iter_mut().rev() {
            if let Some(n) = scope.iter_mut().find(|n| n.name == name) {
                n.properties.insert(key.to_string(), val);
                return;
            }
        }
    }
    pub fn update_input(&mut self, name: &str, index: usize, val: String) {
        for scope in self.stack.iter_mut().rev() {
            if let Some(n) = scope.iter_mut().find(|n| n.name == name) {
                n.inputs.insert(index, val);
                return;
            }
        }
    }
    pub fn update_output_default(&mut self, name: &str, index: usize, val: String) {
        for scope in self.stack.iter_mut().rev() {
            if let Some(n) = scope.iter_mut().find(|n| n.name == name) {
                n.output_defaults.insert(index, val);
                return;
            }
        }
    }
    pub fn update_post_creation(&mut self, name: &str, script: String) {
        for scope in self.stack.iter_mut().rev() {
            if let Some(n) = scope.iter_mut().find(|n| n.name == name) {
                n.post_creation_script = script;
                return;
            }
        }
    }
    pub fn enter_scope(&mut self) {
        self.stack.push(Vec::new());
    }
    pub fn exit_scope(&mut self) -> Scope {
        self.stack.pop().unwrap()
    }
    pub fn take_root(&mut self) -> Scope {
        std::mem::take(&mut self.stack[0])
    }
}

pub static GLOBAL_CONTEXT: Lazy<Mutex<BuildContext>> =
    Lazy::new(|| Mutex::new(BuildContext::new()));

pub fn add_node(data: NodeData) {
    GLOBAL_CONTEXT.lock().unwrap().add_node(data);
}
pub fn update_property(name: &str, key: &str, val: String) {
    GLOBAL_CONTEXT
        .lock()
        .unwrap()
        .update_property(name, key, val);
}
pub fn update_input(name: &str, index: usize, val: String) {
    GLOBAL_CONTEXT
        .lock()
        .unwrap()
        .update_input(name, index, val);
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
pub fn enter_zone() {
    GLOBAL_CONTEXT.lock().unwrap().enter_scope();
}
pub fn exit_zone() -> Scope {
    GLOBAL_CONTEXT.lock().unwrap().exit_scope()
}
pub fn take_root_nodes() -> Scope {
    GLOBAL_CONTEXT.lock().unwrap().take_root()
}
