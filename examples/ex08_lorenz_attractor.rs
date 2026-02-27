use blender_ramen::core::nodes::{
    CompositorNodeAlphaOver, CompositorNodeGlare, CompositorNodeLensdist, CompositorNodeRLayers,
    CompositorNodeRgb, CompositorNodeViewer, GeometryNodeCurvePrimitiveCircle,
    GeometryNodeCurvePrimitiveLine, GeometryNodeCurveToMesh, GeometryNodeJoinGeometry,
    GeometryNodeMeshLine, GeometryNodeSetMaterial, GeometryNodeTransform, NodeGroupOutput,
    ShaderNodeCombineXyz, ShaderNodeEmission, ShaderNodeOutputMaterial, ShaderNodeSeparateXyz,
};
use blender_ramen::core::project::BlenderProject;
use blender_ramen::core::types::{Geo, NodeSocket, Vector};
use blender_ramen::core::zone::repeat_zone;
use ramen_macros::ramen_math;

// ==========================================
// Params (Math)
// ==========================================
const ITERATIONS: i32 = 20000;
const DT: f32 = 0.005;
const P: f32 = 10.0;
const R: f32 = 28.0;
const B: f32 = 2.6666;
const INITIAL_POS: (f32, f32, f32) = (0.1, 0.0, 0.0);

// ==========================================
// Params (Design)
// ==========================================
const WIRE_RADIUS: f32 = 0.04;
const WIRE_RESOLUTION: i32 = 6;
const NEON_COLOR: (f32, f32, f32, f32) = (0.0, 0.8, 1.0, 1.0);
const NEON_STRENGTH: f32 = 15.0;
const TRANSFORM_SCALE: f32 = 0.15;
const TRANSFORM_Z_OFFSET: f32 = -3.5;

// Compositor
const GLARE_FADE: f32 = 0.8;
const BG_COLOR: (f32, f32, f32, f32) = (0.01, 0.01, 0.02, 1.0);
const LENS_DISPERSION: f32 = 0.04;

// ==========================================
// Names
// ==========================================
const GEO_NAME: &str = "LorenzAttractorGeo";
const MAT_NEON: &str = "NeonMat";
const COMP_NAME: &str = "LorenzComp";

fn main() {
    BlenderProject::new()
        .add_shader_tree(MAT_NEON, || {
            let emission = ShaderNodeEmission::new()
                .with_color(NEON_COLOR)
                .with_strength(NEON_STRENGTH);
            ShaderNodeOutputMaterial::new().with_surface(emission.out_emission());
        })
        .add_geometry_tree(GEO_NAME, || {
            let initial_pos = NodeSocket::<Vector>::from(INITIAL_POS);
            let initial_geo = GeometryNodeMeshLine::new().with_count(0).out_mesh();

            // Lorenz Attractor creation loop
            let (_final_pos, final_geo) =
                repeat_zone(ITERATIONS, (initial_pos, initial_geo), |(pos, geo)| {
                    let sep = ShaderNodeSeparateXyz::new().with_vector(pos);
                    let x = sep.out_x();
                    let y = sep.out_y();
                    let z = sep.out_z();

                    let dx = ramen_math!((P * (y - x)) * DT);
                    let dy = ramen_math!((x * (R - z) - y) * DT);
                    let dz = ramen_math!((x * y - B * z) * DT);

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

            let transform = GeometryNodeTransform::new()
                .with_geometry(with_mat.out_geometry())
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
