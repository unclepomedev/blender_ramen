//! # Math Operations Module
//!
//! This module defines the standard arithmetic operations (`+`, `-`, `*`, `/`) for `NodeSocket`s.
//!
//! **[Important Design Consideration]**
//!
//! When connecting node input pins, this module intentionally avoids using auto-generated methods like `.value_0()` produced by `build.rs`.
//! Because Blender nodes (such as the Math node) frequently contain multiple pins with the exact same name (e.g., "Value"), relying on auto-generated, sequentially sanitized names creates a fragile dependency. If Blender updates alter the pin order or add new pins, these generated suffix numbers could shift, leading to fatal bugs in the node graph.
//!
//! To eliminate this vulnerability, our core operational logic adopts a robust design that explicitly targets pins by their physical, immutable indices using `.set_input(0, ...)`.

use crate::core::nodes::{ShaderNodeMath, ShaderNodeVectorMath};
use crate::core::types::{Float, NodeSocket, Vector};

macro_rules! impl_node_op {
    ($Trait:ident, $method:ident, $Node:ident, $op_str:expr, $out:ident, $Type:ident) => {
        // 1. &A + &B
        impl std::ops::$Trait<&NodeSocket<$Type>> for &NodeSocket<$Type> {
            type Output = NodeSocket<$Type>;
            fn $method(self, rhs: &NodeSocket<$Type>) -> Self::Output {
                $Node::new()
                    .with_operation($op_str)
                    .set_input(0, self.clone())
                    .set_input(1, rhs.clone())
                    .$out()
            }
        }
        // 2. A + &B
        impl std::ops::$Trait<&NodeSocket<$Type>> for NodeSocket<$Type> {
            type Output = NodeSocket<$Type>;
            fn $method(self, rhs: &NodeSocket<$Type>) -> Self::Output {
                (&self).$method(rhs)
            }
        }
        // 3. &A + B
        impl std::ops::$Trait<NodeSocket<$Type>> for &NodeSocket<$Type> {
            type Output = NodeSocket<$Type>;
            fn $method(self, rhs: NodeSocket<$Type>) -> Self::Output {
                self.$method(&rhs)
            }
        }
        // 4. A + B
        impl std::ops::$Trait<NodeSocket<$Type>> for NodeSocket<$Type> {
            type Output = NodeSocket<$Type>;
            fn $method(self, rhs: NodeSocket<$Type>) -> Self::Output {
                (&self).$method(&rhs)
            }
        }
    };
}

// Float (ShaderNodeMath)
impl_node_op!(Add, add, ShaderNodeMath, "ADD", out_value, Float);
impl_node_op!(Sub, sub, ShaderNodeMath, "SUBTRACT", out_value, Float);
impl_node_op!(Mul, mul, ShaderNodeMath, "MULTIPLY", out_value, Float);
impl_node_op!(Div, div, ShaderNodeMath, "DIVIDE", out_value, Float);

// Vector (ShaderNodeVectorMath)
impl_node_op!(Add, add, ShaderNodeVectorMath, "ADD", out_vector, Vector);
impl_node_op!(
    Sub,
    sub,
    ShaderNodeVectorMath,
    "SUBTRACT",
    out_vector,
    Vector
);
impl_node_op!(
    Mul,
    mul,
    ShaderNodeVectorMath,
    "MULTIPLY",
    out_vector,
    Vector
);
impl_node_op!(Div, div, ShaderNodeVectorMath, "DIVIDE", out_vector, Vector);

// op(Node, f32) -----------------------------------------------------------------
macro_rules! impl_scalar_op {
    ($Trait:ident, $method:ident) => {
        // &Node + f32
        impl std::ops::$Trait<f32> for &NodeSocket<Float> {
            type Output = NodeSocket<Float>;
            fn $method(self, rhs: f32) -> Self::Output {
                self.$method(&NodeSocket::<Float>::from(rhs))
            }
        }
        // Node + f32
        impl std::ops::$Trait<f32> for NodeSocket<Float> {
            type Output = NodeSocket<Float>;
            fn $method(self, rhs: f32) -> Self::Output {
                (&self).$method(rhs)
            }
        }
        // f32 + &Node
        impl std::ops::$Trait<&NodeSocket<Float>> for f32 {
            type Output = NodeSocket<Float>;
            fn $method(self, rhs: &NodeSocket<Float>) -> Self::Output {
                NodeSocket::<Float>::from(self).$method(rhs)
            }
        }
        // f32 + Node
        impl std::ops::$Trait<NodeSocket<Float>> for f32 {
            type Output = NodeSocket<Float>;
            fn $method(self, rhs: NodeSocket<Float>) -> Self::Output {
                self.$method(&rhs)
            }
        }
    };
}

impl_scalar_op!(Add, add);
impl_scalar_op!(Sub, sub);
impl_scalar_op!(Mul, mul);
impl_scalar_op!(Div, div);

// op(Vector, f32) -----------------------------------------------------------------
macro_rules! impl_vector_scalar_op {
    ($Trait:ident, $method:ident) => {
        // &Vector + f32
        impl std::ops::$Trait<f32> for &NodeSocket<Vector> {
            type Output = NodeSocket<Vector>;
            fn $method(self, rhs: f32) -> Self::Output {
                self.$method(&NodeSocket::<Vector>::from((rhs, rhs, rhs)))
            }
        }
        // Vector + f32
        impl std::ops::$Trait<f32> for NodeSocket<Vector> {
            type Output = NodeSocket<Vector>;
            fn $method(self, rhs: f32) -> Self::Output {
                (&self).$method(rhs)
            }
        }
        // f32 + &Vector
        impl std::ops::$Trait<&NodeSocket<Vector>> for f32 {
            type Output = NodeSocket<Vector>;
            fn $method(self, rhs: &NodeSocket<Vector>) -> Self::Output {
                NodeSocket::<Vector>::from((self, self, self)).$method(rhs)
            }
        }
        // f32 + Vector
        impl std::ops::$Trait<NodeSocket<Vector>> for f32 {
            type Output = NodeSocket<Vector>;
            fn $method(self, rhs: NodeSocket<Vector>) -> Self::Output {
                self.$method(&rhs)
            }
        }
    };
}

impl_vector_scalar_op!(Add, add);
impl_vector_scalar_op!(Sub, sub);
impl_vector_scalar_op!(Mul, mul);
impl_vector_scalar_op!(Div, div);

// ----------------------------------------------------------------------------
// unittest
// ----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context;
    use crate::core::context::test_utils::GLOBAL_TEST_LOCK;

    #[test]
    fn test_float_math_ownership_variants() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let a = NodeSocket::<Float>::from(10.0);
        let b = NodeSocket::<Float>::from(2.0);

        let _ = &a + &b;
        let _ = a.clone() + &b;
        let _ = &a + b.clone();
        let _ = a.clone() + b.clone();

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 4);
        for node in nodes {
            assert_eq!(node.bl_idname, "ShaderNodeMath");
            assert_eq!(node.properties.get("operation").unwrap(), "\"ADD\"");
        }
    }

    #[test]
    fn test_float_math_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let a = NodeSocket::<Float>::from(10.0);
        let b = NodeSocket::<Float>::from(2.0);

        let _ = &a + &b;
        let _ = &a - &b;
        let _ = &a * &b;
        let _ = &a / &b;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 4);

        assert_eq!(nodes[0].properties.get("operation").unwrap(), "\"ADD\"");
        assert_eq!(
            nodes[1].properties.get("operation").unwrap(),
            "\"SUBTRACT\""
        );
        assert_eq!(
            nodes[2].properties.get("operation").unwrap(),
            "\"MULTIPLY\""
        );
        assert_eq!(nodes[3].properties.get("operation").unwrap(), "\"DIVIDE\"");
    }

    #[test]
    fn test_scalar_operations_and_order() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let a = NodeSocket::<Float>::from(5.0);

        let _ = &a - 2.0;
        let _ = 100.0 / &a;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 2);

        assert_eq!(
            nodes[0].properties.get("operation").unwrap(),
            "\"SUBTRACT\""
        );
        assert_eq!(nodes[0].inputs.get(&0).unwrap(), &a.python_expr);
        assert_eq!(nodes[0].inputs.get(&1).unwrap(), "2.0000");

        assert_eq!(nodes[1].properties.get("operation").unwrap(), "\"DIVIDE\"");
        assert_eq!(nodes[1].inputs.get(&0).unwrap(), "100.0000");
        assert_eq!(nodes[1].inputs.get(&1).unwrap(), &a.python_expr);
    }

    #[test]
    fn test_vector_math_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let v1 = NodeSocket::<Vector>::from((1.0, 2.0, 3.0));
        let v2 = NodeSocket::<Vector>::from((0.0, -1.0, 0.5));

        let _ = &v1 + &v2;
        let _ = &v1 - &v2;
        let _ = &v1 * &v2;
        let _ = &v1 / &v2;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 4);

        for node in &nodes {
            assert_eq!(node.bl_idname, "ShaderNodeVectorMath");
        }

        assert_eq!(nodes[0].properties.get("operation").unwrap(), "\"ADD\"");
        assert_eq!(
            nodes[1].properties.get("operation").unwrap(),
            "\"SUBTRACT\""
        );
        assert_eq!(
            nodes[2].properties.get("operation").unwrap(),
            "\"MULTIPLY\""
        );
        assert_eq!(nodes[3].properties.get("operation").unwrap(), "\"DIVIDE\"");
    }

    #[test]
    fn test_vector_scalar_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let v = NodeSocket::<Vector>::from((1.0, 2.0, 3.0));

        let _ = &v * 5.0;
        let _ = 10.0 / &v;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 2);

        assert_eq!(
            nodes[0].properties.get("operation").unwrap(),
            "\"MULTIPLY\""
        );
        assert_eq!(nodes[0].inputs.get(&1).unwrap(), "(5.0000, 5.0000, 5.0000)");

        assert_eq!(nodes[1].properties.get("operation").unwrap(), "\"DIVIDE\"");
        assert_eq!(
            nodes[1].inputs.get(&0).unwrap(),
            "(10.0000, 10.0000, 10.0000)"
        );
    }
}
