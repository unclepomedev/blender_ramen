use blender_ramen::core::nodes::{
    CompositorNodeAlphaOver, CompositorNodeBlur, CompositorNodeLensdist, CompositorNodeRLayers,
    CompositorNodeRgb, CompositorNodeViewer, GeometryNodeCurvePrimitiveCircle,
    GeometryNodeCurvePrimitiveLine, GeometryNodeCurveToMesh, GeometryNodeInputPosition,
    GeometryNodeJoinGeometry, GeometryNodeSetMaterial, GeometryNodeStoreNamedAttribute,
    GeometryNodeStoreNamedAttributeDataType, GeometryNodeTransform, NodeGroupOutput,
    ShaderNodeAttribute, ShaderNodeCombineXyz, ShaderNodeEmission, ShaderNodeLayerWeight,
    ShaderNodeOutputMaterial, ShaderNodeSeparateXyz,
};
use blender_ramen::core::project::BlenderProject;
use blender_ramen::core::types::{Geo, NodeSocket, Vector};
use blender_ramen::core::zone::repeat_zone;
use ramen_macros::ramen_math;

// ==========================================
// Params (Math)
// ==========================================
const ITERATIONS: i32 = 50000;
const DT: f32 = 0.05;
const B: f32 = 0.19;
const INITIAL_POS: (f32, f32, f32) = (0.1, 0.0, 0.0);

// ==========================================
// Params (Design)
// ==========================================
const WIRE_RADIUS: f32 = 0.01;
const WIRE_RESOLUTION: i32 = 8;
const NEON_STRENGTH: f32 = 12.0;
const TRANSFORM_SCALE: f32 = 3.0;
const TRANSFORM_Z_OFFSET: f32 = -1.5;

// Shader: Hologram Scanlines
const SCANLINE_FREQ: f32 = 40.0;

// Compositor: Soft Bloom & CRT
const BG_COLOR: (f32, f32, f32, f32) = (0.005, 0.01, 0.015, 1.0);
const CRT_DISTORTION: f32 = -0.03;
const LENS_DISPERSION: f32 = 0.05;

// ==========================================
// Names
// ==========================================
const GEO_NAME: &str = "AizawaAttractorGeo";
const MAT_NEON: &str = "HologramMat";
const COMP_NAME: &str = "CinematicComp";
const POS_ATTR_NAME: &str = "PosAttr";

//noinspection DuplicatedCode
fn main() {
    BlenderProject::new()
        .add_shader_tree(MAT_NEON, || {
            let attr = ShaderNodeAttribute::new().with_attribute_name(POS_ATTR_NAME);
            let sep = ShaderNodeSeparateXyz::new().with_vector(attr.out_vector());

            let z = sep.out_z();

            // blue <=> gold
            let r = ramen_math!(z * 1.5);
            let g = ramen_math!(0.8);
            let b = ramen_math!(2.0 - z * 2.0);
            let color = ShaderNodeCombineXyz::new()
                .with_x(r)
                .with_y(g)
                .with_z(b)
                .out_vector();

            let layer_weight = ShaderNodeLayerWeight::new().with_blend(0.5);
            let edge_glow = ramen_math!(pow(1.0 - layer_weight.out_facing(), 3.0));

            let scanline = ramen_math!(sin(z * SCANLINE_FREQ) * 0.5 + 0.5);

            let intensity = ramen_math!((edge_glow + scanline * 0.3) * NEON_STRENGTH);

            let emission = ShaderNodeEmission::new()
                .set_input(ShaderNodeEmission::PIN_COLOR, color)
                .set_input(ShaderNodeEmission::PIN_STRENGTH, intensity);

            ShaderNodeOutputMaterial::new().with_surface(emission.out_emission());
        })
        .add_geometry_tree(GEO_NAME, || {
            let initial_pos = NodeSocket::<Vector>::from(INITIAL_POS);
            let initial_geo = GeometryNodeCurvePrimitiveLine::new()
                .with_start(NodeSocket::<Vector>::from(INITIAL_POS))
                .with_end(NodeSocket::<Vector>::from(INITIAL_POS))
                .out_curve()
                .cast::<Geo>();

            let (_final_pos, final_geo) =
                repeat_zone(ITERATIONS, (initial_pos, initial_geo), |(pos, geo)| {
                    let sep = ShaderNodeSeparateXyz::new().with_vector(pos);
                    let x = sep.out_x();
                    let y = sep.out_y();
                    let z = sep.out_z();

                    let dx = ramen_math!((sin(y) - B * x) * DT);
                    let dy = ramen_math!((sin(z) - B * y) * DT);
                    let dz = ramen_math!((sin(x) - B * z) * DT);

                    let delta = ShaderNodeCombineXyz::new()
                        .with_x(dx)
                        .with_y(dy)
                        .with_z(dz)
                        .out_vector();

                    let next_pos = ramen_math!(pos + delta);

                    let segment = GeometryNodeCurvePrimitiveLine::new()
                        .with_start(pos)
                        .with_end(next_pos);

                    let joined = GeometryNodeJoinGeometry::new()
                        .append_geometry(geo)
                        .append_geometry(segment.out_curve().cast::<Geo>())
                        .out_geometry();
                    (next_pos, joined)
                });

            let profile_circle = GeometryNodeCurvePrimitiveCircle::new()
                .with_radius(WIRE_RADIUS)
                .with_resolution(WIRE_RESOLUTION);

            let mesh = GeometryNodeCurveToMesh::new()
                .with_curve(final_geo)
                .with_profile_curve(profile_circle.out_curve());

            let with_mat = GeometryNodeSetMaterial::new()
                .with_geometry(mesh.out_mesh())
                .with_material(MAT_NEON);

            let store_pos = GeometryNodeStoreNamedAttribute::new()
                .with_geometry(with_mat.out_geometry())
                .with_name(POS_ATTR_NAME)
                .with_data_type(GeometryNodeStoreNamedAttributeDataType::FloatVector)
                .set_input(
                    GeometryNodeStoreNamedAttribute::PIN_VALUE,
                    GeometryNodeInputPosition::new().out_position(),
                );

            let transform = GeometryNodeTransform::new()
                .with_geometry(store_pos.out_geometry())
                .with_scale(NodeSocket::<Vector>::from((
                    TRANSFORM_SCALE,
                    TRANSFORM_SCALE,
                    TRANSFORM_SCALE,
                )))
                .with_translation(NodeSocket::<Vector>::from((0.0, 0.0, TRANSFORM_Z_OFFSET)));

            NodeGroupOutput::new().set_input(0, transform.out_geometry());
        })
        .add_compositor_tree(COMP_NAME, || {
            let render_layers = CompositorNodeRLayers::new();
            let bg_color = CompositorNodeRgb::new().default_color(BG_COLOR);

            let base_image = CompositorNodeAlphaOver::new()
                .set_input(
                    CompositorNodeAlphaOver::PIN_BACKGROUND,
                    bg_color.out_color(),
                )
                .set_input(
                    CompositorNodeAlphaOver::PIN_FOREGROUND,
                    render_layers.out_image(),
                );

            let blur = CompositorNodeBlur::new()
                .with_size(NodeSocket::from((1.0, 1.0)))
                .set_input(CompositorNodeBlur::PIN_IMAGE, base_image.out_image());

            let lens_dist = CompositorNodeLensdist::new()
                .set_input(CompositorNodeLensdist::PIN_IMAGE, blur.out_image())
                .set_input(
                    CompositorNodeLensdist::PIN_DISTORTION,
                    NodeSocket::from(CRT_DISTORTION),
                )
                .set_input(
                    CompositorNodeLensdist::PIN_DISPERSION,
                    NodeSocket::from(LENS_DISPERSION),
                );

            NodeGroupOutput::new().set_input(0, lens_dist.out_image());
            CompositorNodeViewer::new()
                .set_input(CompositorNodeViewer::PIN_IMAGE, lens_dist.out_image());
        })
        .send();
}
