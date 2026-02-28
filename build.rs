use heck::{ToPascalCase, ToSnakeCase};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::Path;

// structs to parse json --------------------------------------------------------------------------
#[derive(Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum BlenderSocketType {
    NodeSocketBool,
    NodeSocketBundle,
    NodeSocketClosure,
    NodeSocketCollection,
    NodeSocketColor,
    NodeSocketFloat,
    NodeSocketFloatAngle,
    NodeSocketFloatColorTemperature,
    NodeSocketFloatDistance,
    NodeSocketFloatFactor,
    NodeSocketFloatTimeAbsolute,
    NodeSocketFloatWavelength,
    NodeSocketGeometry,
    NodeSocketImage,
    NodeSocketInt,
    NodeSocketIntUnsigned,
    NodeSocketMaterial,
    NodeSocketMatrix,
    NodeSocketMenu,
    NodeSocketObject,
    NodeSocketRotation,
    NodeSocketShader,
    NodeSocketString,
    NodeSocketStringFilePath,
    NodeSocketVector,
    NodeSocketVector2D,
    NodeSocketVectorDirection,
    NodeSocketVectorEuler,
    NodeSocketVectorFactor,
    NodeSocketVectorFactor2D,
    NodeSocketVectorTranslation,
    NodeSocketVectorVelocity4D,
    NodeSocketVectorXYZ,
    NodeSocketVectorXYZ2D,
    NodeSocketVirtual,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct NodeSocket {
    name: String,
    identifier: String,
    #[serde(rename = "type")]
    type_name: BlenderSocketType,
    default: Option<serde_json::Value>,
    is_multi_input: bool,
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
            s = format!("idx_{}", fallback_index);
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
        let debug_mode = env::var("RAMEN_DEBUG_NODES").is_ok();
        if counter > 0 && debug_mode {
            println!(
                "cargo:warning=API naming collision: '{}_{}' was renamed to '{}'",
                prefix, s, final_name
            );
        }

        self.used_names.insert(final_name.clone());
        final_name
    }
}

// type mapping -----------------------------------------------------------------------

fn map_blender_type_to_rust(socket_type: &BlenderSocketType) -> TokenStream {
    match socket_type {
        BlenderSocketType::NodeSocketGeometry => quote! { crate::core::types::Geo },
        BlenderSocketType::NodeSocketFloat
        | BlenderSocketType::NodeSocketFloatDistance
        | BlenderSocketType::NodeSocketFloatFactor
        | BlenderSocketType::NodeSocketFloatAngle
        | BlenderSocketType::NodeSocketFloatTimeAbsolute
        | BlenderSocketType::NodeSocketFloatColorTemperature
        | BlenderSocketType::NodeSocketFloatWavelength => quote! { crate::core::types::Float },
        BlenderSocketType::NodeSocketInt | BlenderSocketType::NodeSocketIntUnsigned => {
            quote! { crate::core::types::Int }
        }
        BlenderSocketType::NodeSocketVector
        | BlenderSocketType::NodeSocketVectorTranslation
        | BlenderSocketType::NodeSocketVectorDirection
        | BlenderSocketType::NodeSocketVectorXYZ
        | BlenderSocketType::NodeSocketVectorFactor
        | BlenderSocketType::NodeSocketVectorEuler => quote! { crate::core::types::Vector },
        BlenderSocketType::NodeSocketVector2D
        | BlenderSocketType::NodeSocketVectorFactor2D
        | BlenderSocketType::NodeSocketVectorXYZ2D => quote! { crate::core::types::Vector2D },
        BlenderSocketType::NodeSocketVectorVelocity4D => quote! { crate::core::types::Vector4D },
        BlenderSocketType::NodeSocketColor => quote! { crate::core::types::Color },
        BlenderSocketType::NodeSocketBool => quote! { crate::core::types::Bool },
        BlenderSocketType::NodeSocketMaterial => quote! { crate::core::types::Material },
        BlenderSocketType::NodeSocketObject => quote! { crate::core::types::Object },
        BlenderSocketType::NodeSocketCollection => quote! { crate::core::types::Collection },
        BlenderSocketType::NodeSocketImage => quote! { crate::core::types::Image },
        BlenderSocketType::NodeSocketString | BlenderSocketType::NodeSocketStringFilePath => {
            quote! { crate::core::types::StringType }
        }
        BlenderSocketType::NodeSocketShader | BlenderSocketType::NodeSocketClosure => {
            quote! { crate::core::types::Shader }
        }
        BlenderSocketType::NodeSocketMatrix => quote! { crate::core::types::Matrix },
        BlenderSocketType::NodeSocketRotation => quote! { crate::core::types::Rotation },
        BlenderSocketType::NodeSocketMenu => quote! { crate::core::types::Menu },
        BlenderSocketType::NodeSocketBundle => quote! { crate::core::types::Bundle },
        BlenderSocketType::NodeSocketVirtual => quote! { crate::core::types::Any }, // seems amorphous
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
        let mut counter = 0;
        while used_consts.contains(&final_const_name) {
            final_const_name = format!("{}_{}", safe_const_name, counter);
            counter += 1;
        }
        used_consts.insert(final_const_name.clone());

        let const_ident = format_ident!("{}", final_const_name);
        constants.push(quote! {
            pub const #const_ident: usize = #i;
        });

        let prefix = if socket.is_multi_input {
            "append"
        } else {
            "with"
        };
        let safe_name = sanitizer.sanitize_and_register(&socket.name, i, prefix);
        let method_name = format_ident!("{}", safe_name);
        let rust_type = map_blender_type_to_rust(&socket.type_name);

        if socket.is_multi_input {
            methods.push(quote! {
                pub fn #method_name(self, val: impl Into<crate::core::types::NodeSocket<#rust_type>>) -> Self {
                    let socket = val.into();
                    crate::core::context::append_input(&self.name, #i, socket.python_expr(), socket.is_literal);
                    self
                }
            });
        } else {
            methods.push(quote! {
                pub fn #method_name(self, val: impl Into<crate::core::types::NodeSocket<#rust_type>>) -> Self {
                    let socket = val.into();
                    crate::core::context::update_input(&self.name, #i, socket.python_expr(), socket.is_literal);
                    self
                }
            });
        }
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
                crate::core::context::update_output_default(&self.name, #i, val.into().python_expr());
                self
            }
        });

        let getter_name = sanitizer.sanitize_and_register(&socket.name, i, "out");
        let method_getter = format_ident!("{}", getter_name);
        getters.push(quote! {
            pub fn #method_getter(&self) -> crate::core::types::NodeSocket<#rust_type> {
                crate::core::types::NodeSocket::new_output(
                    format!("{}.outputs[{}]", self.name, crate::core::types::python_string_literal(#socket_name))
                )
            }
        });
    }

    (defaults, getters)
}

fn generate_enum_property(
    node_id: &str,
    prop: &NodeProperty,
    items: &[EnumItem],
    method_name: &syn::Ident,
) -> (TokenStream, TokenStream) {
    let enum_name_str = format!(
        "{}{}",
        node_id.to_pascal_case(),
        prop.identifier.to_pascal_case()
    );
    let enum_ident = format_ident!("{}", enum_name_str);

    let mut variants = Vec::new();
    let mut match_arms = Vec::new();

    let mut enum_sanitizer = NameSanitizer::new();

    for (item_i, item) in items.iter().enumerate() {
        // Empty prefix "" forces a leading '_' for safe namespace separation (trimmed later).
        // Fallback uses format!("Variant{{}}") for empty or numeric-starting results.
        let safe_variant_str = enum_sanitizer
            .sanitize_and_register(&item.identifier, item_i, "")
            .trim_start_matches('_')
            .to_pascal_case();
        let safe_variant_str = if safe_variant_str.is_empty()
            || safe_variant_str.chars().next().unwrap().is_numeric()
        {
            format!("Variant{}", safe_variant_str)
        } else {
            safe_variant_str
        };
        let variant_ident = format_ident!("{}", safe_variant_str);
        let item_id = &item.identifier;

        variants.push(quote! { #variant_ident });
        match_arms.push(quote! { Self::#variant_ident => #item_id });
    }

    let enum_def = quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum #enum_ident {
            #(#variants),*
        }
        impl #enum_ident {
            pub fn as_str(&self) -> &'static str {
                match self {
                    #(#match_arms),*
                }
            }
        }
        impl std::fmt::Display for #enum_ident {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }
    };

    let prop_id = &prop.identifier;
    let method_def = quote! {
        pub fn #method_name(self, val: #enum_ident) -> Self {
            crate::core::context::update_property(&self.name, #prop_id, crate::core::types::python_string_literal(val.as_str()));
            self
        }
    };

    (method_def, enum_def)
}

fn generate_properties(
    node_id: &str,
    def: &NodeDef,
    sanitizer: &mut NameSanitizer,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut methods = Vec::new();
    let mut enums = Vec::new();

    for (i, prop) in def.properties.iter().enumerate() {
        let safe_name = sanitizer.sanitize_and_register(&prop.identifier, i, "with");
        let method_name = format_ident!("{}", safe_name);
        let prop_id = &prop.identifier;

        match prop.type_name.as_str() {
            "INT" => methods.push(quote! { pub fn #method_name(self, val: i32) -> Self { crate::core::context::update_property(&self.name, #prop_id, val.to_string()); self } }),
            "FLOAT" => methods.push(quote! { pub fn #method_name(self, val: f32) -> Self { crate::core::context::update_property(&self.name, #prop_id, crate::core::types::fmt_f32(val)); self } }),
            "BOOLEAN" => methods.push(quote! { pub fn #method_name(self, val: bool) -> Self { crate::core::context::update_property(&self.name, #prop_id, if val { "True".to_string() } else { "False".to_string() }); self } }),
            "ENUM" => {
                if let Some(items) = &prop.enum_items
                    && !items.is_empty() {
                        let (method, enum_def) = generate_enum_property(node_id, prop, items, &method_name);
                        enums.push(enum_def);
                        methods.push(method);
                        continue;
                    }
                methods.push(quote! { pub fn #method_name(self, val: &str) -> Self { crate::core::context::update_property(&self.name, #prop_id, crate::core::types::python_string_literal(val)); self } })
            },
            _ => methods.push(quote! { pub fn #method_name(self, val: &str) -> Self { crate::core::context::update_property(&self.name, #prop_id, crate::core::types::python_string_literal(val)); self } })
        }
    }
    (methods, enums)
}

fn generate_node_struct(node_id: &str, def: &NodeDef) -> TokenStream {
    let struct_name = format_ident!("{}", node_id.to_pascal_case());
    let struct_name_str = struct_name.to_string();
    let blender_idname = &def.bl_idname;

    let mut sanitizer = NameSanitizer::new();

    let (input_methods, input_constants) = generate_inputs(def, &mut sanitizer);
    let (output_defaults, output_getters) = generate_outputs(def, &mut sanitizer);
    let (property_methods, property_enums) = generate_properties(node_id, def, &mut sanitizer);

    quote! {
        #(#property_enums)*

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
                crate::core::context::update_input(&self.name, index, val.python_expr(), val.is_literal);
                self
            }
            pub fn append_input<T>(self, index: usize, val: crate::core::types::NodeSocket<T>) -> Self {
                crate::core::context::append_input(&self.name, index, val.python_expr(), val.is_literal);
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
