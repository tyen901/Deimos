use std::{any::Any, sync::Arc, u32};

use d3d11::dxgi;
use deimos_data::tfx::{
    features::{cubemap::SCubemapComponent, dynamic::RenderStageSubscription},
    PrimitiveType, RenderStage,
};
use glam::{Mat4, Quat, UVec4, Vec4};

use crate::{
    asset::{texture::Texture, Handle},
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList},
    tfx::packet::CompactTransform,
    util::geometry,
    Gpu, Renderer,
};

use super::FeatureRenderer;

pub struct CubemapRenderer {
    vb: d3d11::Buffer,
    ib: d3d11::Buffer,

    cb_vs: ConstantBuffer<CubemapTransform>,
    cb_ps: ConstantBuffer<CubemapPixelConstants>,

    texture_cubemap_specular: Handle<Texture>,
    texture_cubemap_alpha: Handle<Texture>,
    texture_voxel_diffuse: Handle<Texture>,

    cubemap: SCubemapComponent,
}

impl CubemapRenderer {
    pub fn load(gpu: &Arc<Gpu>, cubemap: &SCubemapComponent) -> anyhow::Result<Self> {
        let vb = gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_VERTICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::VERTEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_VERTICES)),
        )?;

        let ib = gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width(std::mem::size_of_val(geometry::CUBE_INDICES) as u32)
                .usage(d3d11::Usage::Immutable)
                .bind_flags(d3d11::BindFlags::INDEX_BUFFER)
                .build(),
            Some(bytemuck::cast_slice(geometry::CUBE_INDICES)),
        )?;

        Ok(Self {
            vb,
            ib,
            cb_vs: ConstantBuffer::create(gpu, None)?,
            cb_ps: ConstantBuffer::create(gpu, None)?,
            texture_cubemap_specular: Renderer::instance()
                .asset_manager
                .load(cubemap.texture_cube_specular),
            texture_cubemap_alpha: Renderer::instance()
                .asset_manager
                .load(cubemap.texture_cube_alpha),
            texture_voxel_diffuse: Renderer::instance()
                .asset_manager
                .load(cubemap.texture_voxel_diffuse),
            cubemap: cubemap.clone(),
        })
    }
}

impl FeatureRenderer for CubemapRenderer {
    fn extract_and_prepare(
        &mut self,
        renderer: &crate::Renderer,
        data: &mut dyn super::FeatureRendererData,
        _extracted_data: &dyn std::any::Any,
    ) {
        let obj_local_to_world = (data as &mut dyn Any)
            .downcast_mut::<CompactTransform>()
            .expect("Invalid data type, expected CompactTransform");
        // *obj_local_to_world = extracted_data
        //     .downcast_ref::<CompactTransform>()
        //     .expect("Invalid extracted data type")
        //     .clone();

        let local_to_world = obj_local_to_world.to_mat4();
        let rotation = local_to_world.to_scale_rotation_translation().1;
        let rotation_neg = -rotation;

        self.cb_vs
            .write(
                &renderer.gpu.context(),
                &CubemapTransform {
                    translation: obj_local_to_world.translation().extend(1.0),
                    rotation: rotation_neg,
                    scale: self.cubemap.volume_extents,
                },
            )
            .unwrap();

        // Need param_13 (param_21), param_14 (param_20), param_18 (param_25)
        let param_9 = {
            let mut fvar4 = self.cubemap.probes_resolution[2] as f32;
            if 0.0 <= 1.0 - fvar4 {
                fvar4 = 1.0;
            }
            let mut fvar3 = self.cubemap.probes_resolution[1] as f32;
            if 0.0 <= 1.0 - fvar3 {
                fvar3 = 1.0;
            }
            let mut fvar6 = self.cubemap.probes_resolution[0] as f32;
            if 0.0 <= 1.0 - fvar6 {
                fvar6 = 1.0;
            }
            Vec4::new(1.0 / fvar6, 1.0 / fvar3, 1.0 / fvar4, self.cubemap.unk13c)
        };

        let param_12 = self.cubemap.use_probes() as u8 as f32;

        let param_13 = {
            let v10_x_abs = self.cubemap.unk90.x.abs();
            let v10_y_abs = self.cubemap.unk90.y.abs();
            let v10_z_abs = self.cubemap.unk90.z.abs();
            let mut v = Vec4::ZERO;
            v.x = v10_x_abs + v10_x_abs + self.cubemap.unk80.x;
            v.y = v10_y_abs + v10_y_abs + self.cubemap.unk80.y;
            v.z = v10_z_abs + v10_z_abs + self.cubemap.unk80.z;
            v.w = self.cubemap.unk80.w;
            v
        };

        // let param_14 = self.cubemap.unk70;
        let param_18 = self.cubemap.unk70;

        let mut fvar27 = 0.05;
        if 0.05 - param_13.w < 0.0 {
            fvar27 = param_13.w;
        }

        let mut fvar26 = 0.0001;
        if 0.0001 - param_18 < 0.0 {
            fvar26 = param_18;
        }

        let unk16 = {
            fvar27 = 2.0 / fvar27;
            Vec4::new(1.0 / fvar27, 1.0 / fvar26, -(1.0 - fvar26) / fvar26, 0.0)
        };

        let param_11 = self.cubemap.unk140.with_w(self.cubemap.unk74);
        let mut fvar27 = param_11.z;
        if 0.5 < fvar27 {
            fvar27 = ((fvar27 - 0.5) + (fvar27 - 0.5)).powf(4.0);
            fvar27 = fvar27 * 60.0 + 1.0;
        } else {
            fvar27 = fvar27 + fvar27;
        }
        let unk14 = param_11.with_z(fvar27);
        let param_19 = self.cubemap.unk30;

        let param_5 = 1.0 - self.cubemap.unk40;
        // let param_5 = {
        //     let fVar6 = self.cubemap.unka0.z.abs();
        //     let fVar5 = self.cubemap.unka0.x.abs();
        //     let fVar7 = self.cubemap.unka0.y.abs();
        //     let fVar4 = self.cubemap.unk90.w;
        //     let fVar9 = self.cubemap.unk90.z;
        //     let fVar3 = self.cubemap.unk90.y;

        //     Vec4::new(
        //     fVar5 + fVar5 + self.cubemap.unk90.x,
        //     fVar7 + fVar7 + fVar3,
        //     fVar6 + fVar6 + fVar9,
        //     fVar4,
        //     )
        // };
        let auvar35 = Vec4::ONE / (1.0 - (1.0 - param_5));
        let auvar35_u32 = UVec4::new(
            auvar35.x.to_bits(),
            auvar35.y.to_bits(),
            auvar35.z.to_bits(),
            auvar35.w.to_bits(),
        );

        // let auVar21 = (Vec4::ONE - param_5).abs();
        // let auVar21cmp = auVar21.cmpge(Vec4::splat(0.0001));
        // let auVar21cmp = UVec4::from(auVar21cmp) * u32::MAX;

        // let unk0_u32 = auvar35_u32 & auVar21cmp | !auVar21cmp & 0x447a0000;
        // let mut unk0 = Vec4::ZERO;
        // unk0[0] = f32::from_bits(unk0_u32.x);
        // unk0[1] = f32::from_bits(unk0_u32.y);
        // unk0[2] = f32::from_bits(unk0_u32.z);
        // unk0[3] = param_12;

        // if self.texture_voxel_diffuse.tag().0 == 0x80f21815
        //     && Renderer::instance().asset_manager.count_loading() != 0
        // {
        //     // println!("Cubemap data {:#X?}", self.cubemap);
        //     // println!("param_9 {:?}", param_9);
        //     println!("param_13 {:?}", param_13);
        //     println!("auVar21 {:?}", auVar21);
        //     println!("auvar35 {:?}", auvar35);
        //     // println!("---");
        //     println!("cb11[0]  = {:?}", unk0);
        //     println!("cb11[14] = {:?}", unk14);
        //     println!("cb11[16] = {:?}", unk16);
        //     // std::process::exit(0);
        // }

        // TODO(cohae): cb11[12] doesn't seem to be used, so i can't verify this
        let mut unk9 = self.cubemap.unkf0;
        unk9.w_axis = Vec4::W;

        self.cb_ps
            .write(
                &renderer.gpu.context(),
                &CubemapPixelConstants {
                    unk0: Vec4::new(50.00005, 50.00005, 24.99999, 1.00),
                    unk1: Vec4::new(8.33333, 50.00005, 24.99999, 0.0001),
                    unk2: Vec4::new(49.00005, 49.00005, 23.99999, 10.00),
                    unk3: Vec4::new(7.33333, 49.00005, 23.99999, 8.50),

                    unk4: Vec4::Z,
                    unk5: self.cubemap.unkb0,
                    unk9,

                    unk13: Vec4::new(0.08333, 0.08333, 0.16667, 1.00), // cb11[14] in tfs
                    unk14, // Vec4::new(25.00, 0.50, 0.00, 0.00),         // cb11[15] in tfs
                    unk15: Vec4::new(0.00, 0.00, 0.00, 0.00), // cb11[16] in tfs
                    unk16, // Vec4::new(0.025, 10000.00, -9999.00, 0.00), // Seems universal
                    unk17: Vec4::new(1., 1., 1., 0.), // cb11[18] in tfs
                    unk18: Vec4::new(0.00, 0.00, 0.00, 0.00), // cb11[19] in tfs
                    unk19: Vec4::new(44.72517, 23.25272, 18.41913, 0.00), // cb11[20] in tfs
                },
            )
            .unwrap();
    }

    fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        if stage != RenderStage::Cubemaps {
            return;
        }

        let tech = Renderer::instance()
            .globals
            .pipelines
            .get_specialized_cubemap_pipeline(
                self.cubemap.shape(),
                self.cubemap.use_alpha(),
                self.cubemap.use_probes(),
                self.cubemap.use_relighting(),
                self.cubemap.use_parallax(),
            );

        self.cb_vs.bind(cmd, crate::gpu::ShaderStage::Vertex, 11);
        self.cb_ps.bind(cmd, crate::gpu::ShaderStage::Pixel, 11);

        tech.bind(cmd);

        cmd.set_input_topology(PrimitiveType::Triangles);
        cmd.set_input_layout(1); // float3 v0 : POSITION0, // Format DXGI_FORMAT_R32G32B32_FLOAT size 12

        cmd.input_assembler_set_index_buffer(&self.ib, dxgi::Format::R16Uint, 0);
        cmd.input_assembler_set_vertex_buffers(
            0,
            &[Some(self.vb.clone())],
            Some(&[12]),
            Some(&[0]),
        );

        cmd.pixel_set_shader_resources(1, &[None]); // Unbind t1
        self.texture_cubemap_specular
            .get()
            .inspect(|t| t.bind(cmd, 0, deimos_data::tfx::ShaderStage::Pixel));
        self.texture_cubemap_alpha
            .get()
            .inspect(|t| t.bind(cmd, 1, deimos_data::tfx::ShaderStage::Pixel));
        self.texture_voxel_diffuse
            .get()
            .inspect(|t| t.bind(cmd, 2, deimos_data::tfx::ShaderStage::Pixel));

        cmd.draw_indexed(geometry::CUBE_INDICES.len() as u32, 0, 0);
        cmd.flush_states();
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        RenderStageSubscription::CUBEMAPS
    }
}

#[repr(C)]
pub struct CubemapTransform {
    pub translation: Vec4,
    pub rotation: Quat,
    pub scale: Vec4,
}

#[repr(C)]
pub struct CubemapPixelConstants {
    pub unk0: Vec4,
    pub unk1: Vec4,
    pub unk2: Vec4,
    pub unk3: Vec4,
    pub unk4: Vec4,
    pub unk5: Mat4,
    pub unk9: Mat4,
    pub unk13: Vec4,
    pub unk14: Vec4,
    pub unk15: Vec4,
    pub unk16: Vec4,
    pub unk17: Vec4,
    pub unk18: Vec4,
    pub unk19: Vec4,
}
