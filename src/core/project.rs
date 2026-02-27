use crate::core::live_link::send_to_blender;
use crate::core::tree::{NodeTree, generate_script_header};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
pub struct ProjectItem {
    pub name: String,
    pub script: String,
    pub dependencies: Vec<String>,
}

pub struct BlenderProject {
    header: String,
    items: Vec<ProjectItem>,
}

impl Default for BlenderProject {
    fn default() -> Self {
        Self::new()
    }
}

impl BlenderProject {
    pub fn new() -> Self {
        Self {
            header: generate_script_header(),
            items: Vec::new(),
        }
    }

    pub fn add_shader_tree<F>(mut self, tree_name: &str, builder: F) -> Self
    where
        F: FnOnce(),
    {
        let script = NodeTree::new_shader(tree_name).build(builder);
        self.items.push(ProjectItem {
            name: tree_name.to_string(),
            script,
            dependencies: vec![],
        });
        self
    }

    pub fn add_geometry_tree<F>(mut self, tree_name: &str, builder: F) -> Self
    where
        F: FnOnce(),
    {
        let script = NodeTree::new_geometry(tree_name).build(builder);
        self.items.push(ProjectItem {
            name: tree_name.to_string(),
            script,
            dependencies: vec![],
        });
        self
    }

    pub fn add_compositor_tree<F>(mut self, tree_name: &str, builder: F) -> Self
    where
        F: FnOnce(),
    {
        let script = NodeTree::new_compositor(tree_name).build(builder);
        self.items.push(ProjectItem {
            name: tree_name.to_string(),
            script,
            dependencies: vec![],
        });
        self
    }

    pub fn add_script(mut self, script: &str) -> Self {
        self.items.push(ProjectItem {
            name: format!("_script_{}", self.items.len()),
            script: script.to_string(),
            dependencies: vec![],
        });
        self
    }

    pub fn send(&self) {
        let mut final_script = self.header.clone();

        let sorted_items = resolve_dependencies(&self.items);

        for item in sorted_items {
            final_script.push_str(&item.script);
        }

        #[cfg(debug_assertions)]
        eprintln!("{}", final_script);
        send_to_blender(&final_script);
    }
}

/// Topological Sort
fn resolve_dependencies(items: &[ProjectItem]) -> Vec<&ProjectItem> {
    let all_names: Vec<String> = items.iter().map(|i| i.name.clone()).collect();
    let mut graph = HashMap::new();
    let mut item_map = HashMap::new();

    for item in items {
        let mut deps = item.dependencies.clone();
        for name in &all_names {
            // If the script contains the name of another tree, assume it's a dependency
            // Also ignore auto-generated script names
            if name != &item.name && !name.starts_with("_script_") && item.script.contains(name) {
                deps.push(name.clone());
            }
        }
        graph.insert(item.name.clone(), deps);
        item_map.insert(item.name.clone(), item);
    }

    // DFS
    let mut sorted_names = Vec::new();
    let mut visited = HashSet::new();
    let mut visiting = HashSet::new();

    fn visit(
        name: &String,
        graph: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        visiting: &mut HashSet<String>,
        sorted_names: &mut Vec<String>,
    ) {
        if visited.contains(name) {
            return;
        }
        if visiting.contains(name) {
            // Cyclic dependency detected, just return to avoid infinite loop
            return;
        }

        visiting.insert(name.clone());
        if let Some(deps) = graph.get(name) {
            for dep in deps {
                visit(dep, graph, visited, visiting, sorted_names);
            }
        }
        visiting.remove(name);
        visited.insert(name.clone());
        sorted_names.push(name.clone());
    }

    for item in items {
        visit(
            &item.name,
            &graph,
            &mut visited,
            &mut visiting,
            &mut sorted_names,
        );
    }

    sorted_names
        .into_iter()
        .filter_map(|name| item_map.remove(&name))
        .collect()
}
