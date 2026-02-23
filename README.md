# ramen

- Blender 5+
- Arbitrary code execution from localhost is allowed via the Blender Server. So, please ensure that this tool is only used in a trusted local development environment.

memo
```rust
mod core;

use crate::core::live_link::send_to_blender;
use crate::core::nodes::{
    GeometryNodeMeshGrid, GeometryNodeSetPosition, NodeGroupInput, NodeGroupOutput,
    ShaderNodeCombineXyz,
};
use crate::core::tree::{NodeTree, call_geometry_group, generate_script_header};
use crate::core::types::{Float, GeometryNodeGroupExt, NodeGroupInputExt, NodeSocket};
use ramen_macros::ramen_math;

fn main() {
    let mut final_script = generate_script_header();

    // ==========================================
    // subroutine: Z^2 s.t. Z = X + iY
    // ==========================================
    let complex_sq_script = NodeTree::new_geometry_group("ComplexSquare")
        .with_input("X", "NodeSocketFloat")
        .with_input("Y", "NodeSocketFloat")
        .with_output("OutX", "NodeSocketFloat")
        .with_output("OutY", "NodeSocketFloat")
        .build(|| {
            let group_in = NodeGroupInput::new();

            let x = group_in.socket::<Float>("X");
            let y = group_in.socket::<Float>("Y");

            let out_x = ramen_math!(pow(x, 2.0) - pow(y, 2.0));
            let out_y = ramen_math!(2.0 * x * y);

            NodeGroupOutput::new()
                .set_input(0, out_x)
                .set_input(1, out_y);
        });
    final_script.push_str(&complex_sq_script);

    // ==========================================
    // main tree
    // ==========================================
    let main_script = NodeTree::new_geometry("MandelbrotBase").build(|| {
        let grid = GeometryNodeMeshGrid::new()
            .with_size_x(10.0)
            .with_size_y(10.0)
            .with_vertices_x(50)
            .with_vertices_y(50);

        let sample_x = NodeSocket::<Float>::from(1.5);
        let sample_y = NodeSocket::<Float>::from(-0.5);

        let complex_sq_node = call_geometry_group("ComplexSquare")
            .set_input(0, sample_x)
            .set_input(1, sample_y);

        let result_x = complex_sq_node.out_socket::<Float>("OutX");

        let combine = ShaderNodeCombineXyz::new()
            .with_z(result_x);

        let set_pos = GeometryNodeSetPosition::new()
            .with_geometry(grid.out_mesh())
            .with_offset(combine.out_vector());

        NodeGroupOutput::new().set_input(0, set_pos.out_geometry());
    });
    final_script.push_str(&main_script);

    println!("{}", final_script);
    send_to_blender(&final_script);
}

```