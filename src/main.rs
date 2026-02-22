mod core;

use crate::core::live_link::send_to_blender;
use crate::core::nodes::{
    GeometryNodeMeshGrid, GeometryNodeSetMaterial, GeometryNodeStoreNamedAttribute,
    NodeGroupOutput, ShaderNodeAttribute, ShaderNodeEmission, ShaderNodeOutputMaterial,
};
use crate::core::tree::{NodeTree, generate_script_header};
use crate::core::types::{Material, NodeSocket, Vector};

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
        let grid = GeometryNodeMeshGrid::new()
            .with_size_x(5.0)
            .with_vertices_x(10);

        let store_attr = GeometryNodeStoreNamedAttribute::new()
            .with_geometry(grid.out_mesh())
            .with_name(SHARED_UV_ATTR)
            .with_data_type("FLOAT_VECTOR")
            .with_domain("POINT")
            .set_input(
                GeometryNodeStoreNamedAttribute::PIN_VALUE,
                grid.out_uv_map().cast::<Vector>(),
            );

        let mat_socket =
            NodeSocket::<Material>::new_expr(format!("bpy.data.materials['{}']", MAT_NAME));

        let set_mat = GeometryNodeSetMaterial::new()
            .with_geometry(store_attr.out_geometry())
            .with_material(mat_socket);

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
