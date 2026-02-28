use blender_ramen::core::nodes::{
    CompositorNodeAlphaOver, CompositorNodeGlare, CompositorNodeLensdist, CompositorNodeRLayers,
    CompositorNodeRgb, CompositorNodeViewer, GeometryNodeCurvePrimitiveCircle,
    GeometryNodeCurvePrimitiveLine, GeometryNodeCurveToMesh, GeometryNodeInputPosition,
    GeometryNodeJoinGeometry, GeometryNodeSetMaterial, GeometryNodeStoreNamedAttribute,
    GeometryNodeStoreNamedAttributeDataType, GeometryNodeTransform, NodeGroupOutput,
    ShaderNodeAttribute, ShaderNodeCombineXyz, ShaderNodeEmission, ShaderNodeOutputMaterial,
    ShaderNodeSeparateXyz,
};
use blender_ramen::core::project::BlenderProject;
use blender_ramen::core::types::{Geo, NodeSocket, Vector};
use blender_ramen::core::zone::repeat_zone;
use ramen_macros::ramen_math;

// ==========================================
// Params (Math)
// ==========================================
const ITERATIONS: i32 = 50000;
const DT: f32 = 0.01;
const A: f32 = 0.95;
const B: f32 = 0.7;
const C: f32 = 0.6;
const D: f32 = 3.5;
const E: f32 = 0.25;
const F: f32 = 0.1;
const INITIAL_POS: (f32, f32, f32) = (0.1, 0.0, 0.0);

// ==========================================
// Params (Design)
// ==========================================
const WIRE_RADIUS: f32 = 0.008;
const WIRE_RESOLUTION: i32 = 6;
const NEON_STRENGTH: f32 = 5.0;
const TRANSFORM_SCALE: f32 = 3.0;
const TRANSFORM_Z_OFFSET: f32 = -1.5;

// Compositor
const GLARE_FADE: f32 = 0.8;
const BG_COLOR: (f32, f32, f32, f32) = (0.01, 0.01, 0.02, 1.0);
const LENS_DISPERSION: f32 = 0.04;

// ==========================================
// Names
// ==========================================
const GEO_NAME: &str = "AizawaAttractorGeo";
const MAT_NEON: &str = "NeonMat";
const COMP_NAME: &str = "AizawaComp";
const POS_ATTR_NAME: &str = "PosAttr";

//noinspection DuplicatedCode
fn main() {
    BlenderProject::new()
        .add_shader_tree(MAT_NEON, || {
            let attr = ShaderNodeAttribute::new().with_attribute_name(POS_ATTR_NAME);
            let sep = ShaderNodeSeparateXyz::new().with_vector(attr.out_vector());

            let x = sep.out_x();
            let y = sep.out_y();
            let z = sep.out_z();

            let r = ramen_math!(z * 0.8 + 0.2);
            let g = ramen_math!(abs(x) * 1.5);
            let b = ramen_math!(1.0 - abs(y));

            let color = ShaderNodeCombineXyz::new()
                .with_x(r)
                .with_y(g)
                .with_z(b)
                .out_vector();

            let emission = ShaderNodeEmission::new()
                .set_input(ShaderNodeEmission::PIN_COLOR, color)
                .with_strength(NEON_STRENGTH);

            ShaderNodeOutputMaterial::new().with_surface(emission.out_emission());
        })
        .add_geometry_tree(GEO_NAME, || {
            let initial_pos = NodeSocket::<Vector>::from(INITIAL_POS);
            let initial_geo = GeometryNodeCurvePrimitiveLine::new()
                .with_start(NodeSocket::<Vector>::from(INITIAL_POS))
                .with_end(NodeSocket::<Vector>::from(INITIAL_POS))
                .out_curve()
                .cast::<Geo>();

            // Aizawa Attractor creation loop
            let (_final_pos, final_geo) =
                repeat_zone(ITERATIONS, (initial_pos, initial_geo), |(pos, geo)| {
                    let sep = ShaderNodeSeparateXyz::new().with_vector(pos);
                    let x = sep.out_x();
                    let y = sep.out_y();
                    let z = sep.out_z();

                    let dx = ramen_math!(((z - B) * x - D * y) * DT);
                    let dy = ramen_math!((D * x + (z - B) * y) * DT);
                    let dz = ramen_math!(
                        (C + A * z
                            - pow(z, 3.0) / 3.0
                            - (pow(x, 2.0) + pow(y, 2.0)) * (1.0 + E * z)
                            + F * z * pow(x, 3.0))
                            * DT
                    );

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

            // line materialization
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

            let glare = CompositorNodeGlare::new()
                .with_fade(GLARE_FADE)
                .set_input(CompositorNodeGlare::PIN_IMAGE, render_layers.out_image());

            let bg_color = CompositorNodeRgb::new().default_color(BG_COLOR);

            let alpha_over = CompositorNodeAlphaOver::new()
                .set_input(
                    CompositorNodeAlphaOver::PIN_BACKGROUND,
                    bg_color.out_color(),
                )
                .set_input(CompositorNodeAlphaOver::PIN_FOREGROUND, glare.out_image());

            let lens_dist = CompositorNodeLensdist::new()
                .set_input(CompositorNodeLensdist::PIN_IMAGE, alpha_over.out_image())
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
