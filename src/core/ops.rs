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

use crate::core::nodes::{
    ShaderNodeMath, ShaderNodeMathOperation, ShaderNodeVectorMath, ShaderNodeVectorMathOperation,
};
use crate::core::types::{Float, NodeSocket, Vector};

macro_rules! impl_node_op {
    ($Trait:ident, $method:ident, $Node:ident, $op_enum:expr, $out:ident, $Type:ident) => {
        impl std::ops::$Trait<NodeSocket<$Type>> for NodeSocket<$Type> {
            type Output = NodeSocket<$Type>;
            fn $method(self, rhs: NodeSocket<$Type>) -> Self::Output {
                $Node::new()
                    .with_operation($op_enum)
                    .set_input(0, self)
                    .set_input(1, rhs)
                    .$out()
            }
        }
    };
}

// Float (ShaderNodeMath)
impl_node_op!(
    Add,
    add,
    ShaderNodeMath,
    ShaderNodeMathOperation::Add,
    out_value,
    Float
);
impl_node_op!(
    Sub,
    sub,
    ShaderNodeMath,
    ShaderNodeMathOperation::Subtract,
    out_value,
    Float
);
impl_node_op!(
    Mul,
    mul,
    ShaderNodeMath,
    ShaderNodeMathOperation::Multiply,
    out_value,
    Float
);
impl_node_op!(
    Div,
    div,
    ShaderNodeMath,
    ShaderNodeMathOperation::Divide,
    out_value,
    Float
);

// Vector (ShaderNodeVectorMath)
impl_node_op!(
    Add,
    add,
    ShaderNodeVectorMath,
    ShaderNodeVectorMathOperation::Add,
    out_vector,
    Vector
);
impl_node_op!(
    Sub,
    sub,
    ShaderNodeVectorMath,
    ShaderNodeVectorMathOperation::Subtract,
    out_vector,
    Vector
);
impl_node_op!(
    Mul,
    mul,
    ShaderNodeVectorMath,
    ShaderNodeVectorMathOperation::Multiply,
    out_vector,
    Vector
);
impl_node_op!(
    Div,
    div,
    ShaderNodeVectorMath,
    ShaderNodeVectorMathOperation::Divide,
    out_vector,
    Vector
);

// op(NodeSocket<Vector>, NodeSocket<Float>) -----------------------------------
macro_rules! impl_vector_float_op {
    ($Trait:ident, $method:ident, $op_enum:expr) => {
        // Vector + Float
        impl std::ops::$Trait<NodeSocket<Float>> for NodeSocket<Vector> {
            type Output = NodeSocket<Vector>;
            fn $method(self, rhs: NodeSocket<Float>) -> Self::Output {
                ShaderNodeVectorMath::new()
                    .with_operation($op_enum)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR, self)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR_0, rhs)
                    .out_vector()
            }
        }
        // Float + Vector
        impl std::ops::$Trait<NodeSocket<Vector>> for NodeSocket<Float> {
            type Output = NodeSocket<Vector>;
            fn $method(self, rhs: NodeSocket<Vector>) -> Self::Output {
                ShaderNodeVectorMath::new()
                    .with_operation($op_enum)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR, self)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR_0, rhs)
                    .out_vector()
            }
        }
    };
}

impl_vector_float_op!(Add, add, ShaderNodeVectorMathOperation::Add);
impl_vector_float_op!(Sub, sub, ShaderNodeVectorMathOperation::Subtract);
impl_vector_float_op!(Mul, mul, ShaderNodeVectorMathOperation::Multiply);
impl_vector_float_op!(Div, div, ShaderNodeVectorMathOperation::Divide);

// op(Node, f32) -----------------------------------------------------------------
macro_rules! impl_scalar_op {
    ($Trait:ident, $method:ident) => {
        // Node + f32
        impl std::ops::$Trait<f32> for NodeSocket<Float> {
            type Output = NodeSocket<Float>;
            fn $method(self, rhs: f32) -> Self::Output {
                self.$method(NodeSocket::<Float>::from(rhs))
            }
        }
        // f32 + Node
        impl std::ops::$Trait<NodeSocket<Float>> for f32 {
            type Output = NodeSocket<Float>;
            fn $method(self, rhs: NodeSocket<Float>) -> Self::Output {
                NodeSocket::<Float>::from(self).$method(rhs)
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
        // Vector + f32
        impl std::ops::$Trait<f32> for NodeSocket<Vector> {
            type Output = NodeSocket<Vector>;
            fn $method(self, rhs: f32) -> Self::Output {
                self.$method(NodeSocket::<Vector>::from((rhs, rhs, rhs)))
            }
        }
        // f32 + Vector
        impl std::ops::$Trait<NodeSocket<Vector>> for f32 {
            type Output = NodeSocket<Vector>;
            fn $method(self, rhs: NodeSocket<Vector>) -> Self::Output {
                NodeSocket::<Vector>::from((self, self, self)).$method(rhs)
            }
        }
    };
}

impl_vector_scalar_op!(Add, add);
impl_vector_scalar_op!(Sub, sub);
impl_vector_scalar_op!(Mul, mul);
impl_vector_scalar_op!(Div, div);

// op(Vector2D, Vector2D)-----------------------------------------------------------------
use crate::core::types::Vector2D;
macro_rules! impl_vector2d_op {
    ($Trait:ident, $method:ident, $op_enum:expr) => {
        impl std::ops::$Trait<NodeSocket<Vector2D>> for NodeSocket<Vector2D> {
            type Output = NodeSocket<Vector2D>;
            fn $method(self, rhs: NodeSocket<Vector2D>) -> Self::Output {
                ShaderNodeVectorMath::new()
                    .with_operation($op_enum)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR, self)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR_0, rhs)
                    .out_vector()
                    .cast::<Vector2D>() // downcast
            }
        }
    };
}

impl_vector2d_op!(Add, add, ShaderNodeVectorMathOperation::Add);
impl_vector2d_op!(Sub, sub, ShaderNodeVectorMathOperation::Subtract);
impl_vector2d_op!(Mul, mul, ShaderNodeVectorMathOperation::Multiply);
impl_vector2d_op!(Div, div, ShaderNodeVectorMathOperation::Divide);

// op(Vector2D, Float) --------------------------------------------------------
macro_rules! impl_vector2d_float_op {
    ($Trait:ident, $method:ident, $op_enum:expr) => {
        // Vector2D + Float
        impl std::ops::$Trait<NodeSocket<Float>> for NodeSocket<Vector2D> {
            type Output = NodeSocket<Vector2D>;
            fn $method(self, rhs: NodeSocket<Float>) -> Self::Output {
                ShaderNodeVectorMath::new()
                    .with_operation($op_enum)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR, self)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR_0, rhs)
                    .out_vector()
                    .cast::<Vector2D>() // downcast
            }
        }
        // Float + Vector2D
        impl std::ops::$Trait<NodeSocket<Vector2D>> for NodeSocket<Float> {
            type Output = NodeSocket<Vector2D>;
            fn $method(self, rhs: NodeSocket<Vector2D>) -> Self::Output {
                ShaderNodeVectorMath::new()
                    .with_operation($op_enum)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR, self)
                    .set_input(ShaderNodeVectorMath::PIN_VECTOR_0, rhs)
                    .out_vector()
                    .cast::<Vector2D>() // downcast
            }
        }
    };
}

impl_vector2d_float_op!(Add, add, ShaderNodeVectorMathOperation::Add);
impl_vector2d_float_op!(Sub, sub, ShaderNodeVectorMathOperation::Subtract);
impl_vector2d_float_op!(Mul, mul, ShaderNodeVectorMathOperation::Multiply);
impl_vector2d_float_op!(Div, div, ShaderNodeVectorMathOperation::Divide);

// op(Vector2D, f32) ---------------------------------------------------------------
macro_rules! impl_vector2d_scalar_op {
    ($Trait:ident, $method:ident) => {
        // Vector2D op f32
        impl std::ops::$Trait<f32> for NodeSocket<Vector2D> {
            type Output = NodeSocket<Vector2D>;
            fn $method(self, rhs: f32) -> Self::Output {
                self.$method(NodeSocket::<Vector2D>::from((rhs, rhs)))
            }
        }
        // f32 op Vector2D
        impl std::ops::$Trait<NodeSocket<Vector2D>> for f32 {
            type Output = NodeSocket<Vector2D>;
            fn $method(self, rhs: NodeSocket<Vector2D>) -> Self::Output {
                NodeSocket::<Vector2D>::from((self, self)).$method(rhs)
            }
        }
    };
}

impl_vector2d_scalar_op!(Add, add);
impl_vector2d_scalar_op!(Sub, sub);
impl_vector2d_scalar_op!(Mul, mul);
impl_vector2d_scalar_op!(Div, div);

// int ops ---------------------------------------------------------------
use crate::core::types::Int;
macro_rules! impl_int_op {
    ($Trait:ident, $method:ident, $op_enum:expr) => {
        impl std::ops::$Trait<NodeSocket<Int>> for NodeSocket<Int> {
            type Output = NodeSocket<Int>;
            fn $method(self, rhs: NodeSocket<Int>) -> Self::Output {
                ShaderNodeMath::new()
                    .with_operation($op_enum)
                    .set_input(0, self.cast::<Float>())
                    .set_input(1, rhs.cast::<Float>())
                    .out_value()
                    .cast::<Int>()
            }
        }
    };
}

impl_int_op!(Add, add, ShaderNodeMathOperation::Add);
impl_int_op!(Sub, sub, ShaderNodeMathOperation::Subtract);
impl_int_op!(Mul, mul, ShaderNodeMathOperation::Multiply);
impl_int_op!(Div, div, ShaderNodeMathOperation::Divide);

// op(Int, i32) ---------------------------------------------------------------
macro_rules! impl_int_scalar_op {
    ($Trait:ident, $method:ident) => {
        // Node + i32
        impl std::ops::$Trait<i32> for NodeSocket<Int> {
            type Output = NodeSocket<Int>;
            fn $method(self, rhs: i32) -> Self::Output {
                self.$method(NodeSocket::<Int>::from(rhs))
            }
        }
        // i32 + Node
        impl std::ops::$Trait<NodeSocket<Int>> for i32 {
            type Output = NodeSocket<Int>;
            fn $method(self, rhs: NodeSocket<Int>) -> Self::Output {
                NodeSocket::<Int>::from(self).$method(rhs)
            }
        }
    };
}

impl_int_scalar_op!(Add, add);
impl_int_scalar_op!(Sub, sub);
impl_int_scalar_op!(Mul, mul);
impl_int_scalar_op!(Div, div);

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

        let _ = a + b;
        let _ = a + b;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 2);
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

        let _ = a + b;
        let _ = a - b;
        let _ = a * b;
        let _ = a / b;

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

        let _ = a - 2.0;
        let _ = 100.0 / a;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 2);

        assert_eq!(
            nodes[0].properties.get("operation").unwrap(),
            "\"SUBTRACT\""
        );
        assert_eq!(nodes[0].inputs.get(&0).unwrap()[0].expr, a.python_expr());
        assert_eq!(nodes[0].inputs.get(&1).unwrap()[0].expr, "2.0000");
        assert!(nodes[0].inputs.get(&0).unwrap()[0].is_literal);
        assert!(nodes[0].inputs.get(&1).unwrap()[0].is_literal);

        assert_eq!(nodes[1].properties.get("operation").unwrap(), "\"DIVIDE\"");
        assert_eq!(nodes[1].inputs.get(&0).unwrap()[0].expr, "100.0000");
        assert_eq!(nodes[1].inputs.get(&1).unwrap()[0].expr, a.python_expr());
        assert!(nodes[1].inputs.get(&0).unwrap()[0].is_literal);
        assert!(nodes[1].inputs.get(&1).unwrap()[0].is_literal);
    }

    #[test]
    fn test_vector_math_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let v1 = NodeSocket::<Vector>::from((1.0, 2.0, 3.0));
        let v2 = NodeSocket::<Vector>::from((0.0, -1.0, 0.5));

        let _ = v1 + v2;
        let _ = v1 - v2;
        let _ = v1 * v2;
        let _ = v1 / v2;

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

        let _ = v * 5.0;
        let _ = 10.0 / v;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 2);

        assert_eq!(
            nodes[0].properties.get("operation").unwrap(),
            "\"MULTIPLY\""
        );
        assert_eq!(
            nodes[0].inputs.get(&1).unwrap()[0].expr,
            "(5.0000, 5.0000, 5.0000)"
        );

        assert_eq!(nodes[1].properties.get("operation").unwrap(), "\"DIVIDE\"");
        assert_eq!(
            nodes[1].inputs.get(&0).unwrap()[0].expr,
            "(10.0000, 10.0000, 10.0000)"
        );
    }

    #[test]
    fn test_vector_float_node_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let v = NodeSocket::<Vector>::from((1.0, 2.0, 3.0));
        let f = NodeSocket::<Float>::from(5.0);

        let _ = v * f;
        let _ = f / v;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 2);

        assert_eq!(
            nodes[0].properties.get("operation").unwrap(),
            "\"MULTIPLY\""
        );
        assert_eq!(nodes[0].inputs.get(&0).unwrap()[0].expr, v.python_expr());
        assert_eq!(nodes[0].inputs.get(&1).unwrap()[0].expr, f.python_expr());
        assert_eq!(nodes[1].properties.get("operation").unwrap(), "\"DIVIDE\"");
        assert_eq!(nodes[1].inputs.get(&0).unwrap()[0].expr, f.python_expr());
        assert_eq!(nodes[1].inputs.get(&1).unwrap()[0].expr, v.python_expr());
    }

    #[test]
    fn test_vector2d_math_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let v1 = NodeSocket::<Vector2D>::from((1.0, 2.0));
        let v2 = NodeSocket::<Vector2D>::from((0.5, -1.0));

        let _ = v1 + v2;
        let _ = v1 - v2;
        let _ = v1 * v2;
        let _ = v1 / v2;

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
    fn test_vector2d_float_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let v = NodeSocket::<Vector2D>::from((1.0, 2.0));
        let f = NodeSocket::<Float>::from(3.0);

        let _ = v + f;
        let _ = f * v; // Commutativity check

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 2);

        for node in &nodes {
            assert_eq!(node.bl_idname, "ShaderNodeVectorMath");
        }

        assert_eq!(nodes[0].properties.get("operation").unwrap(), "\"ADD\"");
        assert_eq!(nodes[0].inputs.get(&0).unwrap()[0].expr, v.python_expr());
        assert_eq!(nodes[0].inputs.get(&1).unwrap()[0].expr, f.python_expr());

        assert_eq!(
            nodes[1].properties.get("operation").unwrap(),
            "\"MULTIPLY\""
        );
        assert_eq!(nodes[1].inputs.get(&0).unwrap()[0].expr, f.python_expr());
        assert_eq!(nodes[1].inputs.get(&1).unwrap()[0].expr, v.python_expr());
    }

    #[test]
    fn test_vector2d_scalar_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let v = NodeSocket::<Vector2D>::from((1.0, 2.0));

        let _ = v * 5.0;
        let _ = 10.0 / v;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 2);

        assert_eq!(
            nodes[0].properties.get("operation").unwrap(),
            "\"MULTIPLY\""
        );
        assert_eq!(nodes[0].inputs.get(&1).unwrap()[0].expr, "(5.0000, 5.0000)");

        assert_eq!(nodes[1].properties.get("operation").unwrap(), "\"DIVIDE\"");
        assert_eq!(
            nodes[1].inputs.get(&0).unwrap()[0].expr,
            "(10.0000, 10.0000)"
        );
    }

    #[test]
    fn test_int_math_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let a = NodeSocket::<Int>::from(10);
        let b = NodeSocket::<Int>::from(2);

        let _ = a + b;
        let _ = a - b;
        let _ = a * b;
        let _ = a / b;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 4);

        for node in &nodes {
            assert_eq!(node.bl_idname, "ShaderNodeMath"); // Should use Float math internally
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
    fn test_int_scalar_operations() {
        let _lock = GLOBAL_TEST_LOCK.lock().unwrap();

        context::enter_zone();
        let a = NodeSocket::<Int>::from(5);

        let _ = a - 2;
        let _ = 100 / a;

        let nodes = context::exit_zone();
        assert_eq!(nodes.len(), 2);

        assert_eq!(
            nodes[0].properties.get("operation").unwrap(),
            "\"SUBTRACT\""
        );
        assert_eq!(nodes[0].inputs.get(&0).unwrap()[0].expr, a.python_expr());
        assert_eq!(nodes[0].inputs.get(&1).unwrap()[0].expr, "2"); // Check if scalar formatting is correct

        assert_eq!(nodes[1].properties.get("operation").unwrap(), "\"DIVIDE\"");
        assert_eq!(nodes[1].inputs.get(&0).unwrap()[0].expr, "100");
        assert_eq!(nodes[1].inputs.get(&1).unwrap()[0].expr, a.python_expr());
    }
}
