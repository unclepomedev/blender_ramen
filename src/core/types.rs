#![allow(dead_code)]

pub struct Geo;
pub struct Float;
pub struct Int;
pub struct Vector2D;
pub struct Vector;
pub struct Vector4D;
pub struct Color;
pub struct StringType;
pub struct Bool;
pub struct Material;
pub struct Object;
pub struct Collection;
pub struct Image;
pub struct Shader;
pub struct Matrix;
pub struct Rotation;
pub struct Menu;
pub struct Bundle;
pub struct Any;

// helpers ===============================================================================
pub fn python_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '\n' => out.push_str(r"\n"),
            '\r' => out.push_str(r"\r"),
            '\t' => out.push_str(r"\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn fmt_f32(v: f32) -> String {
    if v.is_nan() {
        "float('nan')".to_string()
    } else if v.is_infinite() && v.is_sign_positive() {
        "float('inf')".to_string()
    } else if v.is_infinite() {
        "float('-inf')".to_string()
    } else {
        format!("{:.4}", v)
    }
}

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

#[derive(Default)]
struct ExprArena {
    exprs: Vec<String>,
    ids: HashMap<String, usize>,
}

// common ===============================================================================
static EXPR_ARENA: LazyLock<Mutex<ExprArena>> = LazyLock::new(|| Mutex::new(ExprArena::default()));

fn intern_expr(expr: String) -> usize {
    let mut arena = EXPR_ARENA.lock().unwrap();
    if let Some(id) = arena.ids.get(&expr) {
        return *id;
    }
    let id = arena.exprs.len();
    arena.exprs.push(expr.clone());
    arena.ids.insert(expr, id);
    id
}

fn get_expr(id: usize) -> Option<String> {
    let arena = EXPR_ARENA.lock().unwrap();
    arena.exprs.get(id).cloned()
}

#[derive(Debug, PartialEq, Eq)]
pub struct NodeSocket<T> {
    expr_id: usize,
    pub is_literal: bool,
    pub _marker: std::marker::PhantomData<T>,
}

impl<T> Copy for NodeSocket<T> {}

impl<T> Clone for NodeSocket<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> NodeSocket<T> {
    pub fn new_literal(expr: impl Into<String>) -> Self {
        Self {
            expr_id: intern_expr(expr.into()),
            is_literal: true,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn new_output(expr: impl Into<String>) -> Self {
        Self {
            expr_id: intern_expr(expr.into()),
            is_literal: false,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn cast<U>(self) -> NodeSocket<U> {
        NodeSocket {
            expr_id: self.expr_id,
            is_literal: self.is_literal,
            _marker: std::marker::PhantomData,
        }
    }

    pub fn python_expr(&self) -> String {
        get_expr(self.expr_id).expect("internal error: invalid expression id")
    }
}

// float ===============================================================================

impl From<f32> for NodeSocket<Float> {
    fn from(v: f32) -> Self {
        Self::new_literal(fmt_f32(v))
    }
}

macro_rules! impl_from_int_for_float_socket {
    ($($t:ty),*) => {
        $(
            impl From<$t> for NodeSocket<Float> {
                fn from(v: $t) -> Self {
                    Self::new_literal(fmt_f32(v as f32))
                }
            }
        )*
    };
}
impl_from_int_for_float_socket!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

// int ===============================================================================
macro_rules! impl_from_int_for_int_socket {
    ($($t:ty),*) => {
        $(
            impl From<$t> for NodeSocket<Int> {
                fn from(v: $t) -> Self {
                    Self::new_literal(v.to_string())
                }
            }
        )*
    };
}
impl_from_int_for_int_socket!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize);

// bool ===============================================================================
impl From<bool> for NodeSocket<Bool> {
    fn from(v: bool) -> Self {
        Self::new_literal(if v { "True" } else { "False" })
    }
}

// string ===============================================================================
impl From<&str> for NodeSocket<StringType> {
    fn from(s: &str) -> Self {
        Self::new_literal(python_string_literal(s))
    }
}

impl From<String> for NodeSocket<StringType> {
    fn from(s: String) -> Self {
        Self::new_literal(python_string_literal(&s))
    }
}

impl From<&str> for NodeSocket<Menu> {
    fn from(s: &str) -> Self {
        Self::new_literal(python_string_literal(s))
    }
}

impl From<String> for NodeSocket<Menu> {
    fn from(s: String) -> Self {
        Self::new_literal(python_string_literal(&s))
    }
}

// vector like ========================================================================
impl From<(f32, f32)> for NodeSocket<Vector2D> {
    fn from(v: (f32, f32)) -> Self {
        Self::new_literal(format!("({}, {})", fmt_f32(v.0), fmt_f32(v.1)))
    }
}

impl From<(f32, f32, f32)> for NodeSocket<Vector> {
    fn from(v: (f32, f32, f32)) -> Self {
        Self::new_literal(format!(
            "({}, {}, {})",
            fmt_f32(v.0),
            fmt_f32(v.1),
            fmt_f32(v.2)
        ))
    }
}

impl From<(f32, f32, f32, f32)> for NodeSocket<Vector4D> {
    fn from(v: (f32, f32, f32, f32)) -> Self {
        Self::new_literal(format!(
            "({}, {}, {}, {})",
            fmt_f32(v.0),
            fmt_f32(v.1),
            fmt_f32(v.2),
            fmt_f32(v.3)
        ))
    }
}

impl From<(f32, f32, f32, f32)> for NodeSocket<Color> {
    fn from(c: (f32, f32, f32, f32)) -> Self {
        Self::new_literal(format!(
            "({}, {}, {}, {})",
            fmt_f32(c.0),
            fmt_f32(c.1),
            fmt_f32(c.2),
            fmt_f32(c.3)
        ))
    }
}

impl From<NodeSocket<Vector>> for NodeSocket<Color> {
    fn from(socket: NodeSocket<Vector>) -> Self {
        socket.cast::<Color>()
    }
}

impl From<NodeSocket<Color>> for NodeSocket<Vector> {
    fn from(socket: NodeSocket<Color>) -> Self {
        socket.cast::<Vector>()
    }
}

impl From<(f32, f32, f32)> for NodeSocket<Rotation> {
    fn from(v: (f32, f32, f32)) -> Self {
        Self::new_literal(format!(
            "({}, {}, {})",
            fmt_f32(v.0),
            fmt_f32(v.1),
            fmt_f32(v.2)
        ))
    }
}

// reference =======================================================================
impl From<&str> for NodeSocket<Material> {
    fn from(mat_name: &str) -> Self {
        Self::new_literal(format!(
            "bpy.data.materials[{}]",
            python_string_literal(mat_name)
        ))
    }
}

impl From<String> for NodeSocket<Material> {
    fn from(mat_name: String) -> Self {
        NodeSocket::<Material>::from(mat_name.as_str())
    }
}

impl From<&str> for NodeSocket<Object> {
    fn from(name: &str) -> Self {
        Self::new_literal(format!(
            "bpy.data.objects.get({})",
            python_string_literal(name)
        ))
    }
}

impl From<String> for NodeSocket<Object> {
    fn from(name: String) -> Self {
        NodeSocket::<Object>::from(name.as_str())
    }
}

impl From<&str> for NodeSocket<Collection> {
    fn from(name: &str) -> Self {
        Self::new_literal(format!(
            "bpy.data.collections.get({})",
            python_string_literal(name)
        ))
    }
}

impl From<String> for NodeSocket<Collection> {
    fn from(name: String) -> Self {
        NodeSocket::<Collection>::from(name.as_str())
    }
}

impl From<&str> for NodeSocket<Image> {
    fn from(name: &str) -> Self {
        Self::new_literal(format!(
            "bpy.data.images.get({})",
            python_string_literal(name)
        ))
    }
}

impl From<String> for NodeSocket<Image> {
    fn from(name: String) -> Self {
        NodeSocket::<Image>::from(name.as_str())
    }
}

// socket def ===============================================================================
pub trait SocketDef {
    fn socket_type() -> &'static str;
    fn default_name() -> &'static str;
    fn blender_socket_type() -> &'static str;
}

macro_rules! impl_socket_def {
    ($type:ident, $sock_type:expr, $def_name:expr, $blender_sock:expr) => {
        impl SocketDef for $type {
            fn socket_type() -> &'static str {
                $sock_type
            }
            fn default_name() -> &'static str {
                $def_name
            }
            fn blender_socket_type() -> &'static str {
                $blender_sock
            }
        }
    };
}

impl_socket_def!(Geo, "GEOMETRY", "Geometry", "NodeSocketGeometry");
impl_socket_def!(Float, "FLOAT", "Value", "NodeSocketFloat");
impl_socket_def!(Int, "INT", "Value", "NodeSocketInt");
impl_socket_def!(Vector2D, "VECTOR2D", "Vector", "NodeSocketVector2D");
impl_socket_def!(Vector, "VECTOR", "Vector", "NodeSocketVector");
impl_socket_def!(Vector4D, "VECTOR4D", "Vector", "NodeSocketVector4D");
impl_socket_def!(Color, "RGBA", "Color", "NodeSocketColor");
impl_socket_def!(Bool, "BOOLEAN", "Boolean", "NodeSocketBool");
impl_socket_def!(StringType, "STRING", "String", "NodeSocketString");
impl_socket_def!(Material, "MATERIAL", "Material", "NodeSocketMaterial");
impl_socket_def!(Object, "OBJECT", "Object", "NodeSocketObject");
impl_socket_def!(
    Collection,
    "COLLECTION",
    "Collection",
    "NodeSocketCollection"
);
impl_socket_def!(Image, "IMAGE", "Image", "NodeSocketImage");
impl_socket_def!(Shader, "SHADER", "Shader", "NodeSocketShader");
impl_socket_def!(Matrix, "MATRIX", "Matrix", "NodeSocketMatrix");
impl_socket_def!(Rotation, "ROTATION", "Rotation", "NodeSocketRotation");
impl_socket_def!(Menu, "MENU", "Menu", "NodeSocketMenu");
impl_socket_def!(Bundle, "BUNDLE", "Bundle", "NodeSocketBundle");

// extensions ==========================================================================
pub trait NodeGroupInputExt {
    fn socket<T>(&self, name: &str) -> NodeSocket<T>;
}

impl NodeGroupInputExt for crate::core::nodes::NodeGroupInput {
    fn socket<T>(&self, name: &str) -> NodeSocket<T> {
        NodeSocket::new_output(format!(
            "{}.outputs[{}]",
            self.name,
            python_string_literal(name)
        ))
    }
}

pub trait GeometryNodeGroupExt {
    fn out_socket<T>(&self, name: &str) -> NodeSocket<T>;
}

impl GeometryNodeGroupExt for crate::core::nodes::GeometryNodeGroup {
    fn out_socket<T>(&self, name: &str) -> NodeSocket<T> {
        NodeSocket::new_output(format!(
            "{}.outputs[{}]",
            self.name,
            python_string_literal(name)
        ))
    }
}

pub trait ShaderNodeGroupExt {
    fn out_socket<T>(&self, name: &str) -> NodeSocket<T>;
}

impl ShaderNodeGroupExt for crate::core::nodes::ShaderNodeGroup {
    fn out_socket<T>(&self, name: &str) -> NodeSocket<T> {
        NodeSocket::new_output(format!(
            "{}.outputs[{}]",
            self.name,
            python_string_literal(name)
        ))
    }
}

// any ===============================================================================
macro_rules! impl_into_any {
    ($($t:ty),*) => {
        $(
            impl From<NodeSocket<$t>> for NodeSocket<Any> {
                fn from(socket: NodeSocket<$t>) -> Self {
                    socket.cast::<Any>()
                }
            }
        )*
    };
}

impl_into_any!(
    Geo, Float, Int, Vector2D, Vector, Vector4D, Color, StringType, Bool, Material, Object,
    Collection, Image, Shader, Matrix, Rotation, Menu, Bundle
);

// ---------------------------------------------------------
// unittest
// ---------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_conversions() {
        assert_eq!(
            NodeSocket::<Float>::from(std::f32::consts::PI).python_expr(),
            "3.1416"
        );
        assert_eq!(
            NodeSocket::<Float>::from(f32::NAN).python_expr(),
            "float('nan')"
        );
        assert_eq!(NodeSocket::<Int>::from(42).python_expr(), "42");
        assert_eq!(NodeSocket::<Bool>::from(true).python_expr(), "True");
        assert_eq!(NodeSocket::<Bool>::from(false).python_expr(), "False");
    }

    #[test]
    fn test_extended_numeric_conversions() {
        assert_eq!(NodeSocket::<Float>::from(42_i32).python_expr(), "42.0000");
        assert_eq!(
            NodeSocket::<Float>::from(100_usize).python_expr(),
            "100.0000"
        );

        assert_eq!(NodeSocket::<Int>::from(42_i32).python_expr(), "42");
        assert_eq!(NodeSocket::<Int>::from(100_usize).python_expr(), "100");
    }

    #[test]
    fn test_string_escaping() {
        let s1 = NodeSocket::<StringType>::from("Hello");
        assert_eq!(s1.python_expr(), "\"Hello\"");

        let s2 = NodeSocket::<StringType>::from("It's an \"apple\"\nNext line");
        assert_eq!(s2.python_expr(), "\"It's an \\\"apple\\\"\\nNext line\"");
    }

    #[test]
    fn test_tuple_conversions() {
        let v = NodeSocket::<Vector>::from((1.0, 0.5, -2.1));
        assert_eq!(v.python_expr(), "(1.0000, 0.5000, -2.1000)");

        let c = NodeSocket::<Color>::from((1.0, 0.0, 0.0, 1.0));
        assert_eq!(c.python_expr(), "(1.0000, 0.0000, 0.0000, 1.0000)");

        let v2 = NodeSocket::<Vector2D>::from((1.0, 0.4));
        assert_eq!(v2.python_expr(), "(1.0000, 0.4000)");

        let rot = NodeSocket::<Rotation>::from((0.0, 1.57, 0.0));
        assert_eq!(rot.python_expr(), "(0.0000, 1.5700, 0.0000)");

        let menu = NodeSocket::<Menu>::from("LINEAR");
        assert_eq!(menu.python_expr(), "\"LINEAR\"");
    }

    #[test]
    fn test_socket_casting() {
        let vec = NodeSocket::<Vector>::new_output("some_node.outputs[0]");
        let color: NodeSocket<Color> = vec.into();
        assert_eq!(color.python_expr(), "some_node.outputs[0]");

        let any: NodeSocket<Any> = color.into();
        assert_eq!(any.python_expr(), "some_node.outputs[0]");
    }

    #[test]
    fn test_reference_types() {
        let obj = NodeSocket::<Object>::from("TargetCube");
        assert_eq!(obj.python_expr(), "bpy.data.objects.get(\"TargetCube\")");

        let mat = NodeSocket::<Material>::from("NeonMat");
        assert_eq!(mat.python_expr(), "bpy.data.materials[\"NeonMat\"]");

        let col = NodeSocket::<Collection>::from("Environment");
        assert_eq!(
            col.python_expr(),
            "bpy.data.collections.get(\"Environment\")"
        );

        let img = NodeSocket::<Image>::from("Noise.png");
        assert_eq!(img.python_expr(), "bpy.data.images.get(\"Noise.png\")");
    }
}
