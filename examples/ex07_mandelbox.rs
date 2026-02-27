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

/// Iteration count
const ITERATIONS: i32 = 4;

/// VolumeCube resolution
const RESOLUTION: i32 = 512;

/// Calculation bound
const BOUND_EXTENT: f32 = 2.0;

/// Meshing threshold
const THRESHOLD: f32 = 0.0001;
/// Mandelbox scale factor (negative for classic Mandelbox)
const SCALE: f32 = -2.4;
/// Fixed radius for sphere fold
const FIXED_RADIUS: f32 = 1.0;
/// Minimum radius for sphere fold (prevents division by zero)
const MIN_RADIUS: f32 = 0.05;

// ==========================================
// Names
// ==========================================
const SUB_NAME: &str = "MandelboxStep";
const MAIN_TREE_NAME: &str = "MandelboxGeo";
const MAT_NAME: &str = "MandelboxMat";
const COMP_NAME: &str = "MandelboxComp";

//noinspection DuplicatedCode
fn main() {
    mod sub_sockets {
        pub const IN_X: usize = 0;
        pub const IN_Y: usize = 1;
        pub const IN_Z: usize = 2;
        pub const IN_CX: usize = 3;
        pub const IN_CY: usize = 4;
        pub const IN_CZ: usize = 5;

        pub const OUT_X: usize = 0;
        pub const OUT_Y: usize = 1;
        pub const OUT_Z: usize = 2;
    }
    let subtree = NodeTree::new_geometry_group(SUB_NAME)
        .with_input::<Float>("X")
        .with_input::<Float>("Y")
        .with_input::<Float>("Z")
        .with_input::<Float>("CX")
        .with_input::<Float>("CY")
        .with_input::<Float>("CZ")
        .with_output::<Float>("OutX")
        .with_output::<Float>("OutY")
        .with_output::<Float>("OutZ")
        .build(|| {
            let group_in = NodeGroupInput::new();
            let x = group_in.socket::<Float>("X");
            let y = group_in.socket::<Float>("Y");
            let z = group_in.socket::<Float>("Z");
            let cx = group_in.socket::<Float>("CX");
            let cy = group_in.socket::<Float>("CY");
            let cz = group_in.socket::<Float>("CZ");

            let scale = NodeSocket::<Float>::from(SCALE);
            let fixed_r2 = NodeSocket::<Float>::from(FIXED_RADIUS * FIXED_RADIUS);
            let min_r2 = NodeSocket::<Float>::from(MIN_RADIUS * MIN_RADIUS);

            // box (clamp) fold
            let bx = ramen_math!(min(max(x, -1.0), 1.0) * 2.0 - x);
            let by = ramen_math!(min(max(y, -1.0), 1.0) * 2.0 - y);
            let bz = ramen_math!(min(max(z, -1.0), 1.0) * 2.0 - z);

            // sphere fold
            let r2 = ramen_math!(pow(bx, 2.0) + pow(by, 2.0) + pow(bz, 2.0));
            let fold_mult = ramen_math!(fixed_r2 / min(max(r2, min_r2), fixed_r2));

            // scaling / calculate offset
            let final_mult = ramen_math!(fold_mult * scale);

            let out_x = ramen_math!(bx * final_mult + cx);
            let out_y = ramen_math!(by * final_mult + cy);
            let out_z = ramen_math!(bz * final_mult + cz);

            NodeGroupOutput::new()
                .set_input(sub_sockets::OUT_X, out_x)
                .set_input(sub_sockets::OUT_Y, out_y)
                .set_input(sub_sockets::OUT_Z, out_z);
        });

    BlenderProject::new()
        .add_script(&subtree)
        .add_shader_tree(MAT_NAME, || {
            let ao = ShaderNodeAmbientOcclusion::new().with_samples(16);

            // want the value to be larger the lower the AO
            let crevice_mask = ramen_math!(pow(1.0 - ao.out_ao(), 3.0) * 10.0);

            // base texture
            let diffuse = ShaderNodeBsdfDiffuse::new().with_color((0.02, 0.01, 0.04, 1.0));

            let emission = ShaderNodeEmission::new()
                .with_color((0.8, 0.1, 1.0, 1.0))
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

            let initial_state = (cur_x, cur_y, cur_z);

            let (final_x, final_y, final_z) =
                repeat_zone(ITERATIONS, initial_state, |(x, y, z)| {
                    let step = call_geometry_group(SUB_NAME)
                        .set_input(sub_sockets::IN_X, x)
                        .set_input(sub_sockets::IN_Y, y)
                        .set_input(sub_sockets::IN_Z, z)
                        .set_input(sub_sockets::IN_CX, cx)
                        .set_input(sub_sockets::IN_CY, cy)
                        .set_input(sub_sockets::IN_CZ, cz);

                    (
                        step.out_socket::<Float>("OutX"),
                        step.out_socket::<Float>("OutY"),
                        step.out_socket::<Float>("OutZ"),
                    )
                });

            let r_final = ramen_math!(sqrt(
                pow(final_x, 2.0) + pow(final_y, 2.0) + pow(final_z, 2.0)
            ));

            let density = ramen_math!(6.0 - r_final);

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
                .with_fade(0.75)
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
