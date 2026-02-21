use crate::core::nodes::{ShaderNodeMath, ShaderNodeVectorMath};
use crate::core::types::{Float, NodeSocket, Vector};

macro_rules! impl_node_op {
    ($Trait:ident, $method:ident, $Node:ident, $op_str:expr, $out:ident, $Type:ident) => {
        // 1. &A + &B
        impl std::ops::$Trait<&NodeSocket<$Type>> for &NodeSocket<$Type> {
            type Output = NodeSocket<$Type>;
            fn $method(self, rhs: &NodeSocket<$Type>) -> Self::Output {
                $Node::new()
                    .operation($op_str)
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

// ----------------------------------------------------------------------------
// unittest
// ----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::context;
    use once_cell::sync::Lazy;
    use std::sync::Mutex;
    static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn test_float_math_ownership_variants() {
        let _lock = TEST_LOCK.lock().unwrap();

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
        let _lock = TEST_LOCK.lock().unwrap();

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
        let _lock = TEST_LOCK.lock().unwrap();

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
        let _lock = TEST_LOCK.lock().unwrap();

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
}
