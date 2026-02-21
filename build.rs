use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

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
struct NodeProperty {
    identifier: String,
    name: String,
    #[serde(rename = "type")]
    type_name: String,
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

const APP_RESERVED_NAMES: &[&str] = &[
    "new",
    "name",
    "inputs",
    "outputs",
    "output_defaults",
    "properties",
    "set_input",
    "creation_script",
    "links_script",
    "from",
    "into",
    "id",
    "add",
];

fn sanitize_identifier(name: &str, index: usize, prefix: &str) -> String {
    let mut s = name.to_snake_case();
    if s.is_empty() {
        return format!("{}_{}", prefix, index);
    }
    if s.chars().next().unwrap().is_numeric() {
        s = format!("_{}", s);
    }
    if APP_RESERVED_NAMES.contains(&s.as_str()) || syn::parse_str::<syn::Ident>(&s).is_err() {
        return format!("{}_", s);
    }
    s
}

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

fn generate_node_struct(node_id: &str, def: &NodeDef) -> TokenStream {
    let struct_name = format_ident!("{}", node_id.to_pascal_case());
    let blender_idname = &def.bl_idname;
    let struct_name_str = struct_name.to_string();
    let mut used_names = HashSet::new();
    for k in APP_RESERVED_NAMES {
        used_names.insert(k.to_string());
    }

    let input_methods: Vec<_> = def.inputs.iter().enumerate().map(|(i, socket)| {
        let mut safe_name = sanitize_identifier(&socket.name, i, "input");
        while used_names.contains(&safe_name) { safe_name = format!("{}_{}", safe_name, i); }
        used_names.insert(safe_name.clone());
        let method_name = format_ident!("{}", safe_name);
        let rust_type = map_blender_type_to_rust(&socket.type_name);
        quote! { pub fn #method_name(self, val: impl Into<crate::core::types::NodeSocket<#rust_type>>) -> Self { crate::core::context::update_input(&self.name, #i, val.into().python_expr); self } }
    }).collect();

    let output_defaults: Vec<_> = def.outputs.iter().enumerate().map(|(i, socket)| {
        let mut safe_name = sanitize_identifier(&socket.name, i, "output");
        while used_names.contains(&safe_name) { safe_name = format!("{}_{}", safe_name, i); }
        used_names.insert(safe_name.clone());
        let method_name = format_ident!("{}", safe_name);
        let rust_type = map_blender_type_to_rust(&socket.type_name);
        quote! { pub fn #method_name(self, val: impl Into<crate::core::types::NodeSocket<#rust_type>>) -> Self { crate::core::context::update_output_default(&self.name, #i, val.into().python_expr); self } }
    }).collect();

    let output_getters: Vec<_> = def.outputs.iter().enumerate().map(|(i, socket)| {
        let mut safe_name = format!("out_{}", sanitize_identifier(&socket.name, i, "val"));
        while used_names.contains(&safe_name) { safe_name = format!("{}_{}", safe_name, i); }
        used_names.insert(safe_name.clone());
        let method_name = format_ident!("{}", safe_name);
        let rust_type = map_blender_type_to_rust(&socket.type_name);
        let socket_name = &socket.name;
        quote! { pub fn #method_name(&self) -> crate::core::types::NodeSocket<#rust_type> { crate::core::types::NodeSocket::new_expr(format!("{}.outputs['{}']", self.name, #socket_name)) } }
    }).collect();

    let property_methods: Vec<_> = def.properties.iter().enumerate().map(|(i, prop)| {
        let mut safe_name = sanitize_identifier(&prop.identifier, i, "prop");
        while used_names.contains(&safe_name) { safe_name = format!("{}_{}", safe_name, i); }
        used_names.insert(safe_name.clone());
        let method_name = format_ident!("{}", safe_name);
        let prop_id = &prop.identifier;
        match prop.type_name.as_str() {
            "INT" => quote! { pub fn #method_name(self, val: i32) -> Self { crate::core::context::update_property(&self.name, #prop_id, val.to_string()); self } },
            "FLOAT" => quote! { pub fn #method_name(self, val: f32) -> Self { crate::core::context::update_property(&self.name, #prop_id, format!("{:.4}", val)); self } },
            "BOOLEAN" => quote! { pub fn #method_name(self, val: bool) -> Self { crate::core::context::update_property(&self.name, #prop_id, if val { "True".to_string() } else { "False".to_string() }); self } },
            _ => quote! { pub fn #method_name(self, val: &str) -> Self { crate::core::context::update_property(&self.name, #prop_id, format!("'{}'", val)); self } }
        }
    }).collect();

    quote! {
        #[derive(Clone, Debug)]
        pub struct #struct_name { pub name: String }
        impl #struct_name {
            pub fn new() -> Self {
                let uuid_str = uuid::Uuid::new_v4().simple().to_string();
                let name = format!("{}_{}", #struct_name_str, uuid_str.chars().take(8).collect::<String>());
                crate::core::context::add_node(crate::core::context::NodeData::new(name.clone(), #blender_idname.to_string()));
                Self { name }
            }
            #(#input_methods)*
            #(#output_defaults)*
            #(#output_getters)*
            #(#property_methods)*
            pub fn set_input(self, index: usize, val: impl Into<crate::core::types::NodeSocket<crate::core::types::Any>>) -> Self { crate::core::context::update_input(&self.name, index, val.into().python_expr); self }
        }
    }
}

fn main() {
    let json_path = "blender_nodes_dump.json";
    println!("cargo:rerun-if-changed={}", json_path);
    let json_content = fs::read_to_string(json_path).unwrap_or_default();
    if json_content.is_empty() {
        return;
    }
    let dump: DumpRoot = serde_json::from_str(&json_content).expect("Failed to parse JSON");
    let mut unique_nodes = HashMap::new();
    unique_nodes.extend(dump.GeometryNodes);
    unique_nodes.extend(dump.ShaderNodes);
    unique_nodes.extend(dump.CompositorNodes);
    let mut structs = Vec::new();
    let mut sorted_keys: Vec<_> = unique_nodes.keys().collect();
    sorted_keys.sort();
    for key in sorted_keys {
        structs.push(generate_node_struct(key, &unique_nodes[key]));
    }
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("nodes.rs");
    let raw_code = quote! { #(#structs)* }.to_string();
    let formatted_code = raw_code.replace("} ", "}\n");
    let final_code = format!(
        "#[allow(warnings)]\n#[allow(clippy::all)]\n\n{}",
        formatted_code
    );
    fs::write(&dest_path, final_code).unwrap();
}
