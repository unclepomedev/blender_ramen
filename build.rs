use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

// structs to parse json --------------------------------------------------------------------------

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct NodeSocket {
    name: String,
    identifier: String,
    #[serde(rename = "type")]
    type_name: String,
    default: Option<serde_json::Value>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct EnumItem {
    identifier: String,
    name: String,
    description: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct NodeProperty {
    identifier: String,
    name: String,
    #[serde(rename = "type")]
    type_name: String,
    enum_items: Option<Vec<EnumItem>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct NodeDef {
    bl_idname: String,
    bl_label: String,
    inputs: Vec<NodeSocket>,
    outputs: Vec<NodeSocket>,
    #[serde(default)]
    properties: Vec<NodeProperty>,
}

#[derive(Deserialize, Debug)]
#[allow(non_snake_case)]
struct DumpRoot {
    GeometryNodes: HashMap<String, NodeDef>,
    ShaderNodes: HashMap<String, NodeDef>,
    CompositorNodes: HashMap<String, NodeDef>,
}

// name sanitize ----------------------------------------------------
struct NameSanitizer {
    used_names: HashSet<String>,
}

impl NameSanitizer {
    fn new() -> Self {
        Self {
            used_names: HashSet::new(),
        }
    }

    fn sanitize_and_register(
        &mut self,
        base_name: &str,
        fallback_index: usize,
        prefix: &str,
    ) -> String {
        let mut s = base_name.to_snake_case();

        if s.is_empty() {
            s = format!("{}", fallback_index);
        } else if s.chars().next().unwrap().is_numeric() {
            s = format!("_{}", s);
        }

        if syn::parse_str::<syn::Ident>(&s).is_err() {
            s = format!("{}_", s);
        }

        let mut final_name = format!("{}_{}", prefix, s);
        let mut counter = 0;

        while self.used_names.contains(&final_name) {
            final_name = format!("{}_{}_{}", prefix, s, counter);
            counter += 1;
        }

        self.used_names.insert(final_name.clone());
        final_name
    }
}

// type mapping -----------------------------------------------------------------------

fn map_blender_type_to_rust(socket_type: &str) -> TokenStream {
    match socket_type {
        "NodeSocketGeometry" => quote! { crate::core::types::Geo },
        "NodeSocketFloat"
        | "NodeSocketFloatDistance"
        | "NodeSocketFloatFactor"
        | "NodeSocketFloatAngle"
        | "NodeSocketFloatTime"
        | "NodeSocketFloatUnsigned" => quote! { crate::core::types::Float },
        "NodeSocketInt"
        | "NodeSocketIntFactor"
        | "NodeSocketIntUnsigned"
        | "NodeSocketIntPercentage"
        | "NodeSocketIntCircle" => quote! { crate::core::types::Int },
        "NodeSocketVector"
        | "NodeSocketVectorTranslation"
        | "NodeSocketVectorDirection"
        | "NodeSocketVectorVelocity"
        | "NodeSocketVectorAcceleration"
        | "NodeSocketVectorEuler"
        | "NodeSocketVectorXYZ"
        | "NodeSocketVectorXYZ2D" => quote! { crate::core::types::Vector },
        "NodeSocketColor" => quote! { crate::core::types::Color },
        "NodeSocketBool" => quote! { crate::core::types::Bool },
        "NodeSocketString" => quote! { crate::core::types::StringType },
        "NodeSocketMaterial" => quote! { crate::core::types::Material },
        "NodeSocketObject" => quote! { crate::core::types::Object },
        "NodeSocketCollection" => quote! { crate::core::types::Collection },
        "NodeSocketImage" => quote! { crate::core::types::Image },
        "NodeSocketTexture" => quote! { crate::core::types::Texture },
        _ => quote! { crate::core::types::Any },
    }
}

// code generator body -----------------------------------------------------------------------------

fn generate_inputs(
    def: &NodeDef,
    sanitizer: &mut NameSanitizer,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut methods = Vec::new();
    let mut constants = Vec::new();
    let mut used_consts = HashSet::new();

    for (i, socket) in def.inputs.iter().enumerate() {
        let base_const_name = socket.name.to_snake_case().to_uppercase();
        let safe_const_name =
            if base_const_name.is_empty() || base_const_name.chars().next().unwrap().is_numeric() {
                format!("PIN_{}", i)
            } else {
                format!("PIN_{}", base_const_name)
            };

        let mut final_const_name = safe_const_name.clone();
        let mut counter = 1;
        while used_consts.contains(&final_const_name) {
            final_const_name = format!("{}_{}", safe_const_name, counter);
            counter += 1;
        }
        used_consts.insert(final_const_name.clone());

        let const_ident = format_ident!("{}", final_const_name);
        constants.push(quote! {
            pub const #const_ident: usize = #i;
        });

        let safe_name = sanitizer.sanitize_and_register(&socket.name, i, "with");
        let method_name = format_ident!("{}", safe_name);
        let rust_type = map_blender_type_to_rust(&socket.type_name);
        methods.push(quote! {
            pub fn #method_name(self, val: impl Into<crate::core::types::NodeSocket<#rust_type>>) -> Self {
                crate::core::context::update_input(&self.name, #i, val.into().python_expr);
                self
            }
        });
    }

    (methods, constants)
}

fn generate_outputs(
    def: &NodeDef,
    sanitizer: &mut NameSanitizer,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut defaults = Vec::new();
    let mut getters = Vec::new();

    for (i, socket) in def.outputs.iter().enumerate() {
        let rust_type = map_blender_type_to_rust(&socket.type_name);
        let socket_name = &socket.name;

        let default_name = sanitizer.sanitize_and_register(&socket.name, i, "default");
        let method_default = format_ident!("{}", default_name);
        defaults.push(quote! {
            pub fn #method_default(self, val: impl Into<crate::core::types::NodeSocket<#rust_type>>) -> Self {
                crate::core::context::update_output_default(&self.name, #i, val.into().python_expr);
                self
            }
        });

        let getter_name = sanitizer.sanitize_and_register(&socket.name, i, "out");
        let method_getter = format_ident!("{}", getter_name);
        getters.push(quote! {
            pub fn #method_getter(&self) -> crate::core::types::NodeSocket<#rust_type> {
                crate::core::types::NodeSocket::new_expr(
                    format!("{}.outputs[{}]", self.name, crate::core::types::python_string_literal(#socket_name))
                )
            }
        });
    }

    (defaults, getters)
}

fn generate_properties(def: &NodeDef, sanitizer: &mut NameSanitizer) -> Vec<TokenStream> {
    def.properties.iter().enumerate().map(|(i, prop)| {
        let safe_name = sanitizer.sanitize_and_register(&prop.identifier, i, "with");
        let method_name = format_ident!("{}", safe_name);
        let prop_id = &prop.identifier;

        match prop.type_name.as_str() {
            "INT" => quote! { pub fn #method_name(self, val: i32) -> Self { crate::core::context::update_property(&self.name, #prop_id, val.to_string()); self } },
            "FLOAT" => quote! { pub fn #method_name(self, val: f32) -> Self { crate::core::context::update_property(&self.name, #prop_id, crate::core::types::fmt_f32(val)); self } },
            "BOOLEAN" => quote! { pub fn #method_name(self, val: bool) -> Self { crate::core::context::update_property(&self.name, #prop_id, if val { "True".to_string() } else { "False".to_string() }); self } },
            // TODO: generate rust enum for type safety
            _ => quote! { pub fn #method_name(self, val: &str) -> Self { crate::core::context::update_property(&self.name, #prop_id, crate::core::types::python_string_literal(val)); self } }
        }
    }).collect()
}

fn generate_node_struct(node_id: &str, def: &NodeDef) -> TokenStream {
    let struct_name = format_ident!("{}", node_id.to_pascal_case());
    let struct_name_str = struct_name.to_string();
    let blender_idname = &def.bl_idname;

    let mut sanitizer = NameSanitizer::new();

    let (input_methods, input_constants) = generate_inputs(def, &mut sanitizer);
    let (output_defaults, output_getters) = generate_outputs(def, &mut sanitizer);
    let property_methods = generate_properties(def, &mut sanitizer);

    quote! {
        #[derive(Clone, Debug)]
        pub struct #struct_name { pub name: String }

        impl #struct_name {
            #(#input_constants)*

            pub fn new() -> Self {
                let uuid_str = uuid::Uuid::new_v4().simple().to_string();
                let name = format!("{}_{}", #struct_name_str, uuid_str.chars().take(12).collect::<String>());
                crate::core::context::add_node(crate::core::context::NodeData::new(name.clone(), #blender_idname.to_string()));
                Self { name }
            }

            #(#input_methods)*
            #(#output_defaults)*
            #(#output_getters)*
            #(#property_methods)*

            pub fn set_input<T>(self, index: usize, val: crate::core::types::NodeSocket<T>) -> Self {
                crate::core::context::update_input(&self.name, index, val.python_expr);
                self
            }
        }
    }
}

// main ===================================

fn main() {
    let json_path = "blender_nodes_dump.json";
    println!("cargo:rerun-if-changed={}", json_path);

    let json_content = fs::read_to_string(json_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", json_path, e));
    if json_content.trim().is_empty() {
        panic!("{} is empty — cannot generate node bindings", json_path);
    }

    let dump: DumpRoot = serde_json::from_str(&json_content).expect("Failed to parse JSON");

    let debug_mode = env::var("RAMEN_DEBUG_NODES").is_ok();
    let mut unique_nodes = HashMap::new();
    for (category, nodes) in [
        ("GeometryNodes", dump.GeometryNodes),
        ("ShaderNodes", dump.ShaderNodes),
        ("CompositorNodes", dump.CompositorNodes),
    ] {
        for (key, def) in nodes {
            if let Some(_existing) = unique_nodes.get(&key)
                && debug_mode
            {
                println!(
                    "cargo:warning=Duplicate node key '{}' in {} (already present), overwriting",
                    key, category
                );
            }
            unique_nodes.insert(key, def);
        }
    }

    let mut structs = Vec::new();
    let mut sorted_keys: Vec<_> = unique_nodes.keys().collect();
    sorted_keys.sort();
    let mut seen_struct_names = HashSet::new();

    for key in sorted_keys {
        let struct_name_str = key.to_pascal_case();

        if seen_struct_names.contains(&struct_name_str) {
            panic!(
                "PascalCase collision: node ID '{}' conflicts with another node resulting in '{}'",
                key, struct_name_str
            );
        }
        seen_struct_names.insert(struct_name_str);
        structs.push(generate_node_struct(key, &unique_nodes[key]));
    }

    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("nodes.rs");

    let raw_code = quote! { #(#structs)* }.to_string();
    fs::write(&dest_path, raw_code).unwrap();
}
