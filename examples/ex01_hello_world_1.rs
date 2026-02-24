use blender_ramen::core::live_link::send_to_blender;
use blender_ramen::core::nodes::{
    GeometryNodeDeleteGeometry, GeometryNodeInputPosition, GeometryNodeMeshGrid,
    GeometryNodeSetMaterial, GeometryNodeStoreNamedAttribute,
    GeometryNodeStoreNamedAttributeDataType, GeometryNodeStoreNamedAttributeDomain,
    NodeGroupOutput, ShaderNodeAttribute, ShaderNodeEmission, ShaderNodeOutputMaterial,
    ShaderNodeSeparateXyz,
};
use blender_ramen::core::tree::{NodeTree, generate_script_header};
use blender_ramen::core::types::Vector;
use ramen_macros::ramen_math;

const SHARED_UV_ATTR: &str = "Procedural_UV";
const MAT_NAME: &str = "MyRustMat";
const GEO_NAME: &str = "LinkTest";

fn main() {
    let mut final_script = generate_script_header();

    // ==========================================
    // Shader Node Tree
    // ==========================================
    let shader_script = NodeTree::new_shader(MAT_NAME).build(|| {
        let attr_node = ShaderNodeAttribute::new().with_attribute_name(SHARED_UV_ATTR);
        let emission = ShaderNodeEmission::new().with_color(attr_node.out_vector());
        ShaderNodeOutputMaterial::new().with_surface(emission.out_emission());
    });

    final_script.push_str(&shader_script);

    // ==========================================
    // Geometry Node Tree
    // ==========================================
    let geo_script = NodeTree::new_geometry(GEO_NAME).build(|| {
        let grid = GeometryNodeMeshGrid::new()
            .with_size_x(5.0)
            .with_size_y(5.0)
            .with_vertices_x(20)
            .with_vertices_y(20);

        let pos = GeometryNodeInputPosition::new().out_position();
        let sep_pos = ShaderNodeSeparateXyz::new().with_vector(pos);
        let x = sep_pos.out_x();
        let y = sep_pos.out_y();

        let cond = ramen_math!(x > 0.0 && y < 0.0);

        let delete = GeometryNodeDeleteGeometry::new()
            .with_geometry(grid.out_mesh())
            .with_selection(cond);

        let store_attr = GeometryNodeStoreNamedAttribute::new()
            .with_geometry(delete.out_geometry())
            .with_name(SHARED_UV_ATTR)
            .with_data_type(GeometryNodeStoreNamedAttributeDataType::FloatVector)
            .with_domain(GeometryNodeStoreNamedAttributeDomain::Point)
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
