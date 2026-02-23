#![allow(dead_code)]

pub struct Geo;
pub struct Float;
pub struct Int;
pub struct Vector;
pub struct Color;
pub struct StringType;
pub struct Bool;
pub struct Material;
pub struct Object;
pub struct Collection;
pub struct Image;
pub struct Texture;
pub struct Any;

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

#[derive(Debug, PartialEq, Eq)]
pub struct NodeSocket<T> {
    pub python_expr: String,
    pub _marker: std::marker::PhantomData<T>,
}

impl<T> Clone for NodeSocket<T> {
    fn clone(&self) -> Self {
        Self {
            python_expr: self.python_expr.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl<T> NodeSocket<T> {
    pub fn new_expr(expr: impl Into<String>) -> Self {
        Self {
            python_expr: expr.into(),
            _marker: std::marker::PhantomData,
        }
    }

    pub fn cast<U>(self) -> NodeSocket<U> {
        NodeSocket {
            python_expr: self.python_expr,
            _marker: std::marker::PhantomData,
        }
    }
}

impl From<f32> for NodeSocket<Float> {
    fn from(v: f32) -> Self {
        Self::new_expr(fmt_f32(v))
    }
}

impl From<i32> for NodeSocket<Int> {
    fn from(v: i32) -> Self {
        Self::new_expr(v.to_string())
    }
}

impl From<bool> for NodeSocket<Bool> {
    fn from(v: bool) -> Self {
        Self::new_expr(if v { "True" } else { "False" })
    }
}

impl From<&str> for NodeSocket<StringType> {
    fn from(s: &str) -> Self {
        Self::new_expr(python_string_literal(s))
    }
}

impl From<String> for NodeSocket<StringType> {
    fn from(s: String) -> Self {
        Self::new_expr(python_string_literal(&s))
    }
}

impl From<(f32, f32, f32)> for NodeSocket<Vector> {
    fn from(v: (f32, f32, f32)) -> Self {
        Self::new_expr(format!(
            "({}, {}, {})",
            fmt_f32(v.0),
            fmt_f32(v.1),
            fmt_f32(v.2)
        ))
    }
}

impl From<(f32, f32, f32, f32)> for NodeSocket<Color> {
    fn from(c: (f32, f32, f32, f32)) -> Self {
        Self::new_expr(format!(
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

impl<T> From<&NodeSocket<T>> for NodeSocket<T> {
    fn from(socket: &NodeSocket<T>) -> Self {
        socket.clone()
    }
}

pub trait SocketDef {
    fn socket_type() -> &'static str;
    fn default_name() -> &'static str;
    fn blender_socket_type() -> &'static str;
}

pub trait NodeGroupInputExt {
    fn socket<T>(&self, name: &str) -> NodeSocket<T>;
}

impl NodeGroupInputExt for crate::core::nodes::NodeGroupInput {
    fn socket<T>(&self, name: &str) -> NodeSocket<T> {
        NodeSocket::new_expr(format!(
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
        NodeSocket::new_expr(format!(
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
        NodeSocket::new_expr(format!(
            "{}.outputs[{}]",
            self.name,
            python_string_literal(name)
        ))
    }
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
impl_socket_def!(Vector, "VECTOR", "Vector", "NodeSocketVector");
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
impl_socket_def!(Texture, "TEXTURE", "Texture", "NodeSocketTexture");

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
    Geo, Float, Int, Vector, Color, StringType, Bool, Material, Object, Collection, Image, Texture
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
            NodeSocket::<Float>::from(std::f32::consts::PI).python_expr,
            "3.1416"
        );
        assert_eq!(
            NodeSocket::<Float>::from(f32::NAN).python_expr,
            "float('nan')"
        );
        assert_eq!(NodeSocket::<Int>::from(42).python_expr, "42");
        assert_eq!(NodeSocket::<Bool>::from(true).python_expr, "True");
        assert_eq!(NodeSocket::<Bool>::from(false).python_expr, "False");
    }

    #[test]
    fn test_string_escaping() {
        let s1 = NodeSocket::<StringType>::from("Hello");
        assert_eq!(s1.python_expr, "\"Hello\"");

        let s2 = NodeSocket::<StringType>::from("It's an \"apple\"\nNext line");
        assert_eq!(s2.python_expr, "\"It's an \\\"apple\\\"\\nNext line\"");
    }

    #[test]
    fn test_tuple_conversions() {
        let v = NodeSocket::<Vector>::from((1.0, 0.5, -2.1));
        assert_eq!(v.python_expr, "(1.0000, 0.5000, -2.1000)");

        let c = NodeSocket::<Color>::from((1.0, 0.0, 0.0, 1.0));
        assert_eq!(c.python_expr, "(1.0000, 0.0000, 0.0000, 1.0000)");
    }

    #[test]
    fn test_socket_casting() {
        let vec = NodeSocket::<Vector>::new_expr("some_node.outputs[0]");
        let color: NodeSocket<Color> = vec.into();
        assert_eq!(color.python_expr, "some_node.outputs[0]");

        let any: NodeSocket<Any> = color.into();
        assert_eq!(any.python_expr, "some_node.outputs[0]");
    }
}
