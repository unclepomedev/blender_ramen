use blender_ramen::core::nodes::{
    CompositorNodeGlare, CompositorNodeLensdist, CompositorNodeRLayers, CompositorNodeViewer,
    GeometryNodeInputPosition, GeometryNodeSetMaterial, GeometryNodeVolumeCube,
    GeometryNodeVolumeToMesh, NodeGroupInput, NodeGroupOutput, ShaderNodeAddShader,
    ShaderNodeAmbientOcclusion, ShaderNodeBsdfDiffuse, ShaderNodeEmission,
    ShaderNodeOutputMaterial, ShaderNodeSeparateXyz,
};
use blender_ramen::core::project::BlenderProject;
use blender_ramen::core::tree::{NodeTree, call_geometry_group};
use blender_ramen::core::types::{Float, GeometryNodeGroupExt, NodeGroupInputExt, NodeSocket};
use blender_ramen::core::zone::repeat_zone;
use ramen_macros::ramen_math;

// ==========================================
// Params
// ==========================================

/// Exponent of the Mandelbulb
const POWER: f32 = 8.0;

/// Iteration count (detail)
const ITERATIONS: i32 = 6;

/// VolumeCube resolution
const RESOLUTION: i32 = 512;

/// Calculation bound
const BOUND_EXTENT: f32 = 1.2;

/// Meshing threshold
const THRESHOLD: f32 = 0.0;

// ==========================================
// Names
// ==========================================
const SUB_NAME: &str = "MandelbulbSDFStep";
const MAIN_TREE_NAME: &str = "MandelbulbSDFGeo";
const MAT_NAME: &str = "MandelbulbMat";
const COMP_NAME: &str = "MandelbulbComp";

//noinspection DuplicatedCode
fn main() {
    mod sub_sockets {
        pub const IN_X: usize = 0;
        pub const IN_Y: usize = 1;
        pub const IN_Z: usize = 2;
        pub const IN_CX: usize = 3;
        pub const IN_CY: usize = 4;
        pub const IN_CZ: usize = 5;
        pub const IN_DR: usize = 6;

        pub const OUT_X: usize = 0;
        pub const OUT_Y: usize = 1;
        pub const OUT_Z: usize = 2;
        pub const OUT_DR: usize = 3;
    }
    let subtree = NodeTree::new_geometry_group(SUB_NAME)
        .with_input::<Float>("X")
        .with_input::<Float>("Y")
        .with_input::<Float>("Z")
        .with_input::<Float>("CX")
        .with_input::<Float>("CY")
        .with_input::<Float>("CZ")
        .with_input::<Float>("DR")
        .with_output::<Float>("OutX")
        .with_output::<Float>("OutY")
        .with_output::<Float>("OutZ")
        .with_output::<Float>("OutDR")
        .build(|| {
            let group_in = NodeGroupInput::new();
            let x = group_in.socket::<Float>("X");
            let y = group_in.socket::<Float>("Y");
            let z = group_in.socket::<Float>("Z");
            let cx = group_in.socket::<Float>("CX");
            let cy = group_in.socket::<Float>("CY");
            let cz = group_in.socket::<Float>("CZ");
            let dr = group_in.socket::<Float>("DR");

            let p = NodeSocket::<Float>::from(POWER);

            let r = ramen_math!(sqrt(pow(x, 2.0) + pow(y, 2.0) + pow(z, 2.0)));
            let out_dr = ramen_math!(p * pow(r, p - 1.0) * dr + 1.0);

            let r_pow = ramen_math!(pow(r, p));
            let theta_p = ramen_math!(atan2(y, x) * p);
            let phi_p = ramen_math!(asin(z / (r + 0.000001)) * p);

            let out_x = ramen_math!(r_pow * cos(phi_p) * cos(theta_p) + cx);
            let out_y = ramen_math!(r_pow * cos(phi_p) * sin(theta_p) + cy);
            let out_z = ramen_math!(r_pow * sin(phi_p) + cz);

            NodeGroupOutput::new()
                .set_input(sub_sockets::OUT_X, out_x)
                .set_input(sub_sockets::OUT_Y, out_y)
                .set_input(sub_sockets::OUT_Z, out_z)
                .set_input(sub_sockets::OUT_DR, out_dr);
        });

    BlenderProject::new()
        .add_script(&subtree)
        .add_shader_tree(MAT_NAME, || {
            let ao = ShaderNodeAmbientOcclusion::new().with_samples(16);

            // want the value to be larger the lower the AO
            let crevice_mask = ramen_math!(pow(1.0 - ao.out_ao(), 3.0) * 20.0);

            // base texture
            let diffuse = ShaderNodeBsdfDiffuse::new().with_color((0.02, 0.02, 0.03, 1.0)); // dark blue gray

            // cyan light in the valley
            let emission = ShaderNodeEmission::new()
                .with_color((0.0, 0.8, 1.0, 1.0))
                .set_input(ShaderNodeEmission::PIN_STRENGTH, crevice_mask);

            // additive composition of Diffuse and Emission
            let add_shader = ShaderNodeAddShader::new()
                .set_input(ShaderNodeAddShader::PIN_SHADER, diffuse.out_bsdf())
                .set_input(ShaderNodeAddShader::PIN_SHADER_0, emission.out_emission());

            ShaderNodeOutputMaterial::new().with_surface(add_shader.out_shader());
        })
        .add_geometry_tree(MAIN_TREE_NAME, || {
            let pos = GeometryNodeInputPosition::new().out_position();
            let sep_pos = ShaderNodeSeparateXyz::new().with_vector(pos);

            let cur_x = sep_pos.out_x();
            let cur_y = sep_pos.out_y();
            let cur_z = sep_pos.out_z();
            let cx = sep_pos.out_x();
            let cy = sep_pos.out_y();
            let cz = sep_pos.out_z();

            let dr_init = NodeSocket::<Float>::from(1.0_f32);
            let initial_state = (cur_x, cur_y, cur_z, dr_init);

            let (final_x, final_y, final_z, final_dr) =
                repeat_zone(ITERATIONS, initial_state, |(x, y, z, dr)| {
                    let step = call_geometry_group(SUB_NAME)
                        .set_input(sub_sockets::IN_X, x)
                        .set_input(sub_sockets::IN_Y, y)
                        .set_input(sub_sockets::IN_Z, z)
                        .set_input(sub_sockets::IN_CX, cx)
                        .set_input(sub_sockets::IN_CY, cy)
                        .set_input(sub_sockets::IN_CZ, cz)
                        .set_input(sub_sockets::IN_DR, dr);

                    (
                        step.out_socket::<Float>("OutX"),
                        step.out_socket::<Float>("OutY"),
                        step.out_socket::<Float>("OutZ"),
                        step.out_socket::<Float>("OutDR"),
                    )
                });

            let r_final = ramen_math!(sqrt(
                pow(final_x, 2.0) + pow(final_y, 2.0) + pow(final_z, 2.0)
            ));
            let sdf = ramen_math!(
                0.5 / log(r_final, std::f32::consts::E) * (r_final + 0.000001) / final_dr
            );
            let density = ramen_math!(-sdf);

            let volume_cube = GeometryNodeVolumeCube::new()
                .with_resolution_x(RESOLUTION)
                .with_resolution_y(RESOLUTION)
                .with_resolution_z(RESOLUTION)
                .with_min((-BOUND_EXTENT, -BOUND_EXTENT, -BOUND_EXTENT))
                .with_max((BOUND_EXTENT, BOUND_EXTENT, BOUND_EXTENT))
                .set_input(GeometryNodeVolumeCube::PIN_DENSITY, density);

            let to_mesh = GeometryNodeVolumeToMesh::new()
                .with_volume(volume_cube.out_volume())
                .with_threshold(THRESHOLD);

            let set_mat = GeometryNodeSetMaterial::new()
                .with_geometry(to_mesh.out_mesh())
                .with_material(MAT_NAME);

            NodeGroupOutput::new().set_input(0, set_mat.out_geometry());
        })
        .add_compositor_tree(COMP_NAME, || {
            let render_layers = CompositorNodeRLayers::new();

            // Glare (Fog Glow)
            let glare = CompositorNodeGlare::new()
                .set_input(CompositorNodeGlare::PIN_IMAGE, render_layers.out_image());

            // scaling by dispersion
            let lens_dist = CompositorNodeLensdist::new()
                .set_input(CompositorNodeLensdist::PIN_IMAGE, glare.out_image())
                .set_input(
                    CompositorNodeLensdist::PIN_DISTORTION,
                    NodeSocket::from(0.02_f32),
                )
                .set_input(
                    CompositorNodeLensdist::PIN_DISPERSION,
                    NodeSocket::from(0.15_f32),
                );

            NodeGroupOutput::new().set_input(0, lens_dist.out_image());
            CompositorNodeViewer::new()
                .set_input(CompositorNodeViewer::PIN_IMAGE, lens_dist.out_image());
        })
        .send();
}
