use blender_ramen::core::live_link::send_to_blender;
use blender_ramen::core::nodes::{
    GeometryNodeMeshGrid, GeometryNodeSetPosition, NodeGroupOutput, ShaderNodeCombineXyz,
};
use blender_ramen::core::tree::{NodeTree, generate_script_header};
use blender_ramen::core::types::Float;
use blender_ramen::core::zone::repeat_zone;
use ramen_macros::ramen_math;

const MAIN_TREE_NAME: &str = "HelloWorld3_RepeatZone";

fn main() {
    let mut final_script = generate_script_header();

    let main_script = NodeTree::new_geometry(MAIN_TREE_NAME).build(|| {
        let grid = GeometryNodeMeshGrid::new()
            .with_size_x(2.0)
            .with_size_y(2.0)
            .with_vertices_x(10)
            .with_vertices_y(10);

        let initial_geo = grid.out_mesh();
        let initial_offset = blender_ramen::core::types::NodeSocket::<Float>::from(0.5);

        // Repeat Zone: Iterate 5 times
        let (out_geo, _final_offset) =
            repeat_zone(5, (initial_geo, initial_offset), |(geo, offset)| {
                // Inside the zone, we translate the geometry up by `offset` each step
                let offset_vec = ShaderNodeCombineXyz::new()
                    .with_z(offset.clone())
                    .out_vector();

                let set_pos = GeometryNodeSetPosition::new()
                    .with_geometry(geo)
                    .with_offset(offset_vec);

                // Increase the offset for the next iteration using ramen_math
                let next_offset = ramen_math!(offset * 1.5);

                (set_pos.out_geometry(), next_offset)
            });

        NodeGroupOutput::new().set_input(0, out_geo);
    });

    final_script.push_str(&main_script);

    println!("{}", final_script);
    send_to_blender(&final_script);
}
