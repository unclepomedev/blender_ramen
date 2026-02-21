use crate::core::nodes::{ShaderNodeMath, ShaderNodeVectorMath};
use crate::core::types::{Float, NodeSocket, Vector};

macro_rules! impl_node_op {
    ($Trait:ident, $method:ident, $Node:ident, $op_str:expr, $in1:ident, $in2:ident, $out:ident, $Type:ident) => {
        // 1. &A + &B
        impl std::ops::$Trait<&NodeSocket<$Type>> for &NodeSocket<$Type> {
            type Output = NodeSocket<$Type>;
            fn $method(self, rhs: &NodeSocket<$Type>) -> Self::Output {
                $Node::new().operation($op_str).$in1(self).$in2(rhs).$out()
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
impl_node_op!(
    Add,
    add,
    ShaderNodeMath,
    "ADD",
    value,
    value_1,
    out_value,
    Float
);
impl_node_op!(
    Sub,
    sub,
    ShaderNodeMath,
    "SUBTRACT",
    value,
    value_1,
    out_value,
    Float
);
impl_node_op!(
    Mul,
    mul,
    ShaderNodeMath,
    "MULTIPLY",
    value,
    value_1,
    out_value,
    Float
);
impl_node_op!(
    Div,
    div,
    ShaderNodeMath,
    "DIVIDE",
    value,
    value_1,
    out_value,
    Float
);

// Vector (ShaderNodeVectorMath)
impl_node_op!(
    Add,
    add,
    ShaderNodeVectorMath,
    "ADD",
    vector,
    vector_1,
    out_vector,
    Vector
);
impl_node_op!(
    Sub,
    sub,
    ShaderNodeVectorMath,
    "SUBTRACT",
    vector,
    vector_1,
    out_vector,
    Vector
);
impl_node_op!(
    Mul,
    mul,
    ShaderNodeVectorMath,
    "MULTIPLY",
    vector,
    vector_1,
    out_vector,
    Vector
);
impl_node_op!(
    Div,
    div,
    ShaderNodeVectorMath,
    "DIVIDE",
    vector,
    vector_1,
    out_vector,
    Vector
);

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

    #[test]
    fn test_math_operations_without_clone() {
        context::enter_zone();

        let a = NodeSocket::<Float>::from(10.0);
        let b = NodeSocket::<Float>::from(2.0);

        let _add = &a + &b;
        let _sub = &a - &b;

        let _mul = &a * 5.0;
        let _div = 100.0 / &a;

        let _reused = &a + &b + &a;

        let nodes = context::exit_zone();
        assert!(!nodes.is_empty());
    }
}
