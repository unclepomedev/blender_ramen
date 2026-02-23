use blender_ramen::core::live_link::send_to_blender;
use blender_ramen::core::nodes::{
    GeometryNodeInputPosition, GeometryNodeMeshGrid, GeometryNodeSetPosition, NodeGroupInput,
    NodeGroupOutput, ShaderNodeCombineXyz, ShaderNodeSeparateXyz,
};
use blender_ramen::core::tree::{NodeTree, call_geometry_group, generate_script_header};
use blender_ramen::core::types::{Float, GeometryNodeGroupExt, NodeGroupInputExt};
use ramen_macros::ramen_math;
const SUB_NAME: &str = "ComplexSquare";
const MAIN_TREE_NAME: &str = "MainTree";

fn main() {
    let mut final_script = generate_script_header();

    // ==========================================
    // subroutine: Z^2 s.t. Z = X + iY
    // ==========================================
    let complex_sq_script = NodeTree::new_geometry_group(SUB_NAME)
        .with_input::<Float>("X")
        .with_input::<Float>("Y")
        .with_output::<Float>("OutX")
        .with_output::<Float>("OutY")
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
    let main_script = NodeTree::new_geometry(MAIN_TREE_NAME).build(|| {
        let grid = GeometryNodeMeshGrid::new()
            .with_size_x(10.0)
            .with_size_y(10.0)
            .with_vertices_x(50)
            .with_vertices_y(50);

        let pos = GeometryNodeInputPosition::new().out_position();
        let sep_pos = ShaderNodeSeparateXyz::new().with_vector(pos);
        let x = sep_pos.out_x();
        let y = sep_pos.out_y();

        let step = call_geometry_group("ComplexSquare")
            .set_input(0, x)
            .set_input(1, y);

        let combine = ShaderNodeCombineXyz::new()
            .with_x(0.0)
            .with_y(0.0)
            .with_z(step.out_socket::<Float>("OutX") * 0.2); // Re

        let set_pos = GeometryNodeSetPosition::new()
            .with_geometry(grid.out_mesh())
            .with_offset(combine.out_vector());

        NodeGroupOutput::new().set_input(0, set_pos.out_geometry());
    });
    final_script.push_str(&main_script);

    println!("{}", final_script);
    send_to_blender(&final_script);
}
