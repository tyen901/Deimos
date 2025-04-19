use d3d11::dxgi;
use deimos_core::{convar::ConVars, package::package_manager};
use deimos_data::tfx::{
    features::{
        dynamic::RenderStageSubscription,
        light::{SLight, SShadowingLight},
    },
    PrimitiveType, RenderStage,
};
use dxbc_patcher as dxbc;
use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};
use tiger_pkg::TagHash;

use crate::{
    tfx::{
        externs::{self, DeferredLight, SimpleGeometry, VolumeFog},
        packet::CompactTransform,
        technique::{ShaderModule, Technique},
    },
    util::geometry,
    Renderer,
};

use super::FeatureRenderer;

pub struct LightRenderer {
    technique_lighting_apply: Technique,
    technique_volumetrics: Option<Technique>,
    // technique_light_probe_apply: Technique,

    // TODO(cohae): This should be a shared resource (eg. a struct in the renderer that we can use instead of recreating it for every light/cubemap)
    vb: d3d11::Buffer,
    ib: d3d11::Buffer,

    local_to_world: glam::Mat4,
    light_space_transform: glam::Mat4,
}

impl LightRenderer {
    pub fn new(renderer: &Renderer, light: &SLight) -> anyhow::Result<Box<Self>> {
        Self::new_impl(
            renderer,
            light.technique_lighting_apply,
            light.technique_volumetrics,
            // light.technique_light_probe_apply,
            light.light_space_transform,
        )
    }

    pub fn new_shadowing(
        renderer: &Renderer,
        light: &SShadowingLight,
    ) -> anyhow::Result<Box<Self>> {
        Self::new_impl(
            renderer,
            light.technique_lighting_apply,
            light.technique_volumetrics,
            light.light_space_transform,
        )
    }

    fn new_impl(
        renderer: &Renderer,
        technique_shading: TagHash,
        technique_volumetrics: TagHash,
        // technique_light_probe: TagHash,
        light_space_transform: Mat4,
    ) -> anyhow::Result<Box<Self>> {
        let vb = renderer.gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_VERTICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::VERTEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_VERTICES)),
        )?;

        let ib = renderer.gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_INDICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::INDEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_INDICES)),
        )?;

        let mut tech = Technique::load(&renderer.gpu, technique_shading)?;
        if ConVars::get_flag("render.patch_light_shader") {
            patch_light_shader(&mut tech);
        }

        Ok(Box::new(Self {
            technique_lighting_apply: tech,
            technique_volumetrics: technique_volumetrics
                .is_some()
                .then(|| Technique::load(&renderer.gpu, technique_volumetrics))
                .transpose()?,
            // technique_light_probe_apply: Technique::load(&renderer.gpu, technique_light_probe)?,
            vb,
            ib,
            local_to_world: Mat4::IDENTITY,
            light_space_transform,
        }))
    }
}

impl FeatureRenderer for LightRenderer {
    fn extract_and_prepare(
        &mut self,
        _renderer: &crate::Renderer,
        _data: &mut dyn super::FeatureRendererData,
        extracted_data: &dyn std::any::Any,
    ) {
        if let Some(transform) = extracted_data.downcast_ref::<CompactTransform>() {
            self.local_to_world = transform.to_mat4();
        }
    }

    fn submit(&self, cmd: &mut crate::gpu::command_list::CommandList, stage: RenderStage) {
        if stage == RenderStage::LightProbeApply {
            // TODO
            return;
        }

        {
            let (scale, _rotation, _translation) =
                self.local_to_world.to_scale_rotation_translation();

            let local_to_world_scaled = self.local_to_world * self.light_space_transform;
            let externs = Renderer::instance().externs.get_mut();
            externs.simple_geometry = SimpleGeometry {
                local_to_world: externs.view.world_to_projective
                    * local_to_world_scaled
                    * Mat4::from_scale(Vec3::NEG_ONE),
            };

            let view_translation_inverse_mat4 = Mat4::from_translation(-externs.view.position());
            let local_to_world_relative = view_translation_inverse_mat4 * self.local_to_world;

            let (min, max) = compute_light_bounds(self.light_space_transform);
            let light_local_to_world = compute_light_local_to_world(self.local_to_world, min, max);

            externs.deferred_light = DeferredLight {
                unk40: local_to_world_relative.inverse().transpose(),
                unk80: (view_translation_inverse_mat4 * light_local_to_world).inverse(),
                unkc0: local_to_world_relative,

                unk100: 1.0,
                unk104: 0.0,
                unk108: 0.0,
                unk10c: scale.x,
                unk110: 1.0,
            };

            externs.rigid_model = externs::RigidModel {
                local_to_world: light_local_to_world,
                ..Default::default()
            };

            if stage == RenderStage::Volumetrics {
                let mut fog = VolumeFog::default();
                fog.unk00 = light_local_to_world.inverse();
                fog.unk40 = fog.unk00 * externs.view.target_pixel_to_world;
                fog.unka0 = (max - min).extend(1.);
                fog.unkb0 = 1.0;

                let p = fog.unk00.mul_vec4(externs.view.position().extend(1.0));
                let point_w_abs = (-p.wwww()).abs();
                fog.unk80 = Vec4::select(point_w_abs.cmpge(Vec4::splat(0.0001)), p / p.wwww(), p);
                // fog.unk80 = fog
                //     .unk00
                //     .project_point3(externs.view.position())
                //     .extend(1.0);

                if ((fog.unk80.x < -0.2) || (1.2 < fog.unk80.x))
                    || ((fog.unk80.y < -0.2) || (1.2 < fog.unk80.y))
                    || ((fog.unk80.z < -0.2) || (1.2 < fog.unk80.z))
                {
                    // cmd.state =
                    //     cmd.state
                    //         .select(&PipelineState::new(Some(0xf), Some(3), None, None));
                    fog.unkb4 = -1.0;
                } else {
                    // cmd.state = cmd
                    //     .state
                    //     .select(&PipelineState::new(Some(1), Some(2), None, None));
                    fog.unkb4 = 1.0;
                }

                externs.volume_fog = fog;
            } else {
                // cmd.state = cmd
                //     .state
                //     .select(&PipelineState::new(Some(8), None, None, None));
            }
        }

        if stage == RenderStage::Volumetrics {
            if let Some(ref technique) = self.technique_volumetrics {
                technique.bind(cmd).unwrap();
            } else {
                return;
            }
        } else {
            self.technique_lighting_apply.bind(cmd).unwrap();
        }

        cmd.set_input_topology(PrimitiveType::Triangles);
        cmd.set_input_layout(1); // float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12

        cmd.input_assembler_set_index_buffer(&self.ib, dxgi::Format::R16Uint, 0);
        cmd.input_assembler_set_vertex_buffers(
            0,
            &[Some(self.vb.clone())],
            Some(&[12]),
            Some(&[0]),
        );

        cmd.draw_indexed(geometry::CUBE_INDICES.len() as u32, 0, 0);
        cmd.flush_states();
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        RenderStageSubscription::LIGHTING_APPLY
            | RenderStageSubscription::LIGHT_PROBE_APPLY
            | RenderStageSubscription::VOLUMETRICS
    }
}

fn patch_light_shader(tech: &mut Technique) {
    if let Some(ref mut stage) = tech.stage_pixel {
        let Some(shader_ref) = package_manager().get_entry(stage.shader.shader) else {
            error!("Failed to get light shader for patching");
            return;
        };
        let Ok(mut new_shader) = package_manager().read_tag(shader_ref.reference) else {
            error!("Failed to read light shader for patching");
            return;
        };

        let ops = match dxbc_patcher::disassemble(&new_shader) {
            Ok(ops) => ops,
            Err(e) => {
                error!(
                    "Failed to disassemble light shader {} for patching: {}",
                    stage.shader.shader, e
                );
                return;
            }
        };

        let new_shader_words: &mut [u32] = bytemuck::cast_slice_mut(&mut new_shader);
        for op in ops {
            match op.op {
                // Weird dot product that we need to patch out to avoid lights clipping (this was removed in beyond light)
                dxbc::Opcode::Dp4 => {
                    new_shader_words[op.offset..op.offset + op.size]
                        .fill(dxbc::Opcode::Nop as u32 | 0x01 << 24);
                    break;
                }
                dxbc::Opcode::Sample
                | dxbc::Opcode::SampleC
                | dxbc::Opcode::SampleCLz
                | dxbc::Opcode::SampleL
                | dxbc::Opcode::SampleD
                | dxbc::Opcode::SampleB => {
                    break;
                }
                // The relevant dot product happens before the if statement in the standard light shader, so we can stop here
                dxbc::Opcode::If => {
                    break;
                }
                _ => {}
            }
        }

        if !dxbc::hash::update_dxbc_file_hash(&mut new_shader) {
            // No changes were made
            return;
        }

        match ShaderModule::load_raw(
            &Renderer::instance().gpu,
            &new_shader,
            deimos_data::tfx::ShaderStage::Pixel,
        ) {
            Ok(s) => {
                s.set_name(&format!(
                    "PS {} (Technique {}) (Patched DP4)",
                    stage.shader.shader, tech.hash
                ));
                stage.shader_module = s
            }
            Err(e) => error!("Failed to load patched light shader: {}", e),
        }
    }
}

fn compute_light_bounds(light_space_transform: Mat4) -> (Vec3, Vec3) {
    let mut points = [
        Vec3::new(-1.0, -1.0, -1.0),
        Vec3::new(-1.0, -1.0, 1.0),
        Vec3::new(-1.0, 1.0, -1.0),
        Vec3::new(-1.0, 1.0, 1.0),
        Vec3::new(1.0, -1.0, -1.0),
        Vec3::new(1.0, -1.0, 1.0),
        Vec3::new(1.0, 1.0, -1.0),
        Vec3::new(1.0, 1.0, 1.0),
    ];

    for point in &mut points {
        let p = light_space_transform.mul_vec4(point.extend(1.0));
        let point_w_abs = (-p.wwww()).abs();
        *point = Vec4::select(
            point_w_abs.cmpge(Vec4::splat(0.0001)),
            p / p.wwww(),
            Vec4::W,
        )
        .truncate();
    }

    points
        .iter()
        .fold((Vec3::MAX, Vec3::MIN), |(min, max), &point| {
            (min.min(point), max.max(point))
        })
}

fn compute_light_local_to_world(node_local_to_world: Mat4, min: Vec3, max: Vec3) -> Mat4 {
    let bounds_center = min.midpoint(max);
    let bounds_half_extents = (max - min) / 2.0;

    // First matrix operation ("mat"):
    // Each column is computed by scaling one of node_local_to_world’s axes by the corresponding component of bounds_half_extents,
    // except for the w-axis which is a linear combination of the x, y, and z axes plus the original w-axis.
    let mat = Mat4 {
        x_axis: node_local_to_world.x_axis * bounds_half_extents.x,
        y_axis: node_local_to_world.y_axis * bounds_half_extents.y,
        z_axis: node_local_to_world.z_axis * bounds_half_extents.z,
        w_axis: node_local_to_world.x_axis * bounds_center.x
            + node_local_to_world.y_axis * bounds_center.y
            + node_local_to_world.z_axis * bounds_center.z
            + node_local_to_world.w_axis,
    };

    // Second matrix operation ("mat_scaled"):
    // Scale the x, y, and z axes by 2, and subtract all three from the w-axis.
    let mat_scaled = Mat4 {
        x_axis: mat.x_axis * 2.0,
        y_axis: mat.y_axis * 2.0,
        z_axis: mat.z_axis * 2.0,
        w_axis: mat.w_axis - mat.x_axis - mat.y_axis - mat.z_axis,
    };

    // Third matrix operation (computing light_local_to_world):
    // Rearrange the columns of mat_scaled: swap the x and z axes, leaving y and w unchanged.
    let light_local_to_world = Mat4 {
        x_axis: mat_scaled.z_axis,
        y_axis: mat_scaled.y_axis,
        z_axis: mat_scaled.x_axis,
        w_axis: mat_scaled.w_axis,
    };

    light_local_to_world
}
