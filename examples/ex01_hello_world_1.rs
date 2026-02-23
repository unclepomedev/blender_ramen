use blender_ramen::core::live_link::send_to_blender;
use blender_ramen::core::nodes::{
    GeometryNodeMeshGrid, GeometryNodeSetMaterial, GeometryNodeStoreNamedAttribute,
    NodeGroupOutput, ShaderNodeAttribute, ShaderNodeEmission, ShaderNodeOutputMaterial,
};
use blender_ramen::core::tree::{generate_script_header, NodeTree};
use blender_ramen::core::types::{AttrDomain, AttrType, Vector};
use ramen_macros::ramen_math;

const SHARED_UV_ATTR: &str = "Procedural_UV";
const MAT_NAME: &str = "MyRustMat";

fn main() {
    let mut final_script = generate_script_header();

    // ==========================================
    // 1. Shader Node Tree
    // ==========================================
    let shader_script = NodeTree::new_shader(MAT_NAME).build(|| {
        let attr_node = ShaderNodeAttribute::new().with_attribute_name(SHARED_UV_ATTR);
        let emission = ShaderNodeEmission::new().with_color(attr_node.out_vector());
        ShaderNodeOutputMaterial::new().with_surface(emission.out_emission());
    });

    final_script.push_str(&shader_script);

    // ==========================================
    // 2. Geometry Node Tree
    // ==========================================
    let geo_script = NodeTree::new_geometry("LinkTest").build(|| {
        let result = ramen_math!(sin(10.0 + 5.0) * 2.0 / 2.0);
        let grid = GeometryNodeMeshGrid::new()
            .with_size_x(result)
            .with_vertices_x(10);

        let store_attr = GeometryNodeStoreNamedAttribute::new()
            .with_geometry(grid.out_mesh())
            .with_name(SHARED_UV_ATTR)
            .with_data_type(AttrType::VECTOR)
            .with_domain(AttrDomain::POINT)
            .set_input(
                GeometryNodeStoreNamedAttribute::PIN_VALUE,
                grid.out_uv_map().cast::<Vector>(),
            );

        let set_mat = GeometryNodeSetMaterial::new()
            .with_geometry(store_attr.out_geometry())
            .with_material(MAT_NAME);

        // Note on magic numbers for Group Input/Output nodes:
        // Unlike standard built-in nodes, `NodeGroupOutput` and `NodeGroupInput` have dynamic sockets
        // that depend entirely on the custom interface defined for the specific Node Tree.
        // In our `tree.rs` setup script, we explicitly created a single 'Geometry' output socket first:
        // `tree.interface.new_socket('Geometry', in_out='OUTPUT', ...)`
        // Therefore, this socket physically resides at index `0`.
        //
        // Rule of thumb: Always use raw physical indices (0, 1, 2...) for Group Input/Output nodes,
        // corresponding to the exact order the sockets were registered in the tree's interface.
        // Do not rely on auto-generated `PIN_*` constants for these dynamic nodes.
        NodeGroupOutput::new().set_input(0, set_mat.out_geometry());
    });

    final_script.push_str(&geo_script);

    println!("{}", final_script);
    send_to_blender(&final_script);
}
