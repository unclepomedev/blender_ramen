use blender_ramen::core::nodes::{
    GeometryNodeBlurAttribute, GeometryNodeBlurAttributeDataType, GeometryNodeCurvePrimitiveCircle,
    GeometryNodeCurveToMesh, GeometryNodeInputIndex, GeometryNodeInputNormal,
    GeometryNodeInputPosition, GeometryNodeResampleCurve, GeometryNodeSetMaterial,
    GeometryNodeSetPosition, GeometryNodeTransform, NodeGroupOutput, ShaderNodeBsdfPrincipled,
    ShaderNodeCombineXyz, ShaderNodeOutputMaterial, ShaderNodeTexNoise,
};
use blender_ramen::core::project::BlenderProject;
use blender_ramen::core::types::{Color, Float, Geo, NodeSocket, Vector};
use blender_ramen::core::zone::repeat_zone;
use ramen_macros::ramen_math;

// ==========================================
// Params
// ==========================================
const ITERATIONS: i32 = 120;
const INITIAL_RADIUS: f32 = 1.9;
const RESAMPLE_LENGTH: f32 = 0.05;
const GROWTH_RATE: f32 = 0.3;
const BLUR_ITERATIONS: i32 = 5;
const THICKNESS: f32 = 15.0;
const WIRE_RADIUS: f32 = 0.25;
const WIRE_RESOLUTION: i32 = 8;
const TRANSFORM_SCALE: f32 = 0.4;
const Z_FREQUENCY: f32 = 0.005;

// ==========================================
// Names
// ==========================================
const GEO_NAME: &str = "RamenGeo";
const MAT_RAMEN: &str = "RamenMat";

fn main() {
    BlenderProject::new()
        .add_geometry_tree(GEO_NAME, || {
            let initial_geo = GeometryNodeCurvePrimitiveCircle::new()
                .with_radius(INITIAL_RADIUS)
                .with_resolution(64)
                .out_curve()
                .cast::<Geo>();

            let (final_geo,) = repeat_zone(ITERATIONS, (initial_geo,), |(geo,)| {
                // Growth
                let normal = GeometryNodeInputNormal::new().out_normal();
                let noise = ShaderNodeTexNoise::new().with_scale(1.0);
                let noise_fac = noise.out_factor();

                let growth_offset = ramen_math!(normal * noise_fac * GROWTH_RATE);

                let grown_geo = GeometryNodeSetPosition::new()
                    .with_geometry(geo)
                    .with_offset(growth_offset)
                    .out_geometry();

                // Resample
                let resampled = GeometryNodeResampleCurve::new()
                    .with_curve(grown_geo)
                    .with_mode("Length")
                    .with_length(RESAMPLE_LENGTH)
                    .out_curve()
                    .cast::<Geo>();

                // Blur
                let pos = GeometryNodeInputPosition::new().out_position();
                let blurred_pos = GeometryNodeBlurAttribute::new()
                    .with_data_type(GeometryNodeBlurAttributeDataType::FloatVector)
                    .set_input(GeometryNodeBlurAttribute::PIN_VALUE, pos)
                    .with_iterations(BLUR_ITERATIONS)
                    .out_value()
                    .cast::<Vector>();

                // Relax
                let relaxed = GeometryNodeSetPosition::new()
                    .with_geometry(resampled)
                    .with_position(blurred_pos)
                    .out_geometry();

                // Flatten
                let current_pos = GeometryNodeInputPosition::new().out_position();
                let flat_pos =
                    ramen_math!(current_pos * NodeSocket::<Vector>::from((1.0, 1.0, 0.0)));
                let flattened = GeometryNodeSetPosition::new()
                    .with_geometry(relaxed)
                    .with_position(flat_pos)
                    .out_geometry();

                (flattened,)
            });

            // Post-Process
            let index = GeometryNodeInputIndex::new().out_index();

            // Wave
            let noise_coord_x = ramen_math!(NodeSocket::cast::<Float>(index) * Z_FREQUENCY);
            let noise_coord = ShaderNodeCombineXyz::new()
                .with_x(noise_coord_x)
                .out_vector();
            let z_noise = ShaderNodeTexNoise::new()
                .set_input(ShaderNodeTexNoise::PIN_VECTOR, noise_coord)
                .with_scale(1.0);

            let z_noise_color = z_noise.out_color().cast::<Vector>();

            // Thickness (Z-direction)
            let z_offset = ramen_math!(
                (z_noise_color - NodeSocket::<Vector>::from((0.5, 0.5, 0.5)))
                    * NodeSocket::<Vector>::from((0.0, 0.0, THICKNESS))
            );

            let thickened_geo = GeometryNodeSetPosition::new()
                .with_geometry(final_geo)
                .with_offset(z_offset)
                .out_geometry();

            let profile_circle = GeometryNodeCurvePrimitiveCircle::new()
                .with_radius(WIRE_RADIUS)
                .with_resolution(WIRE_RESOLUTION);

            let mesh = GeometryNodeCurveToMesh::new()
                .with_curve(thickened_geo)
                .with_profile_curve(profile_circle.out_curve());

            let with_mat = GeometryNodeSetMaterial::new()
                .with_geometry(mesh.out_mesh())
                .with_material(MAT_RAMEN);

            let transform = GeometryNodeTransform::new()
                .with_geometry(with_mat.out_geometry())
                .with_scale(NodeSocket::<Vector>::from((
                    TRANSFORM_SCALE,
                    TRANSFORM_SCALE,
                    TRANSFORM_SCALE,
                )));

            NodeGroupOutput::new().set_input(0, transform.out_geometry());
        })
        .add_shader_tree(MAT_RAMEN, || {
            // Ramen Yellow
            let base_color = NodeSocket::<Color>::from((0.85, 0.65, 0.25, 1.00));

            let principled = ShaderNodeBsdfPrincipled::new()
                .with_base_color(base_color)
                .with_roughness(0.35)
                .with_subsurface_weight(0.1);

            ShaderNodeOutputMaterial::new().with_surface(principled.out_bsdf());
        })
        .send();
}
