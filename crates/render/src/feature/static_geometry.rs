use std::{f32, io::Write, ops::Deref, sync::Arc};

use bytemuck::{Pod, Zeroable};
use deimos_core::ConVars;
use deimos_data::tfx::{
    common::AxisAlignedBBox,
    features::{
        dynamic::RenderStageSubscription,
        statics::{SStaticInstanceTransform, SStaticMesh, SStaticSpecialMesh},
    },
    RenderStage, ShaderStage,
};
use glam::{Mat4, Vec3, Vec4};
use itertools::Itertools;
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;
use tiger_pkg::TagHash;

use crate::{
    asset::{vertex_buffer::VertexBuffer, Handle},
    camera::Camera,
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList},
    tfx::technique::Technique,
    Gpu, Renderer,
};

use super::{shared::ModelBuffers, FeatureRenderer};

struct SpecialMesh {
    mesh: SStaticSpecialMesh,
    buffers: ModelBuffers,
    technique: Handle<Technique>,
}

impl Deref for SpecialMesh {
    type Target = SStaticSpecialMesh;

    fn deref(&self) -> &Self::Target {
        &self.mesh
    }
}

pub struct StaticModel {
    pub model: SStaticMesh,
    pub materials: Vec<Handle<Technique>>,
    pub hash: TagHash,
    pub subscribed_stages: RenderStageSubscription,
    buffers: Vec<ModelBuffers>,
    special_meshes: Vec<SpecialMesh>,
}

impl StaticModel {
    #[profiling::function]
    pub fn load(hash: TagHash) -> anyhow::Result<Self> {
        let model = package_manager().read_tag_struct::<SStaticMesh>(hash)?;
        let materials = model
            .techniques
            .iter()
            .map(|&tag| Renderer::instance().asset_manager.load::<Technique>(tag))
            .collect();

        let buffers = model
            .opaque_meshes
            .buffers
            .iter()
            .map(
                |&(index_buffer, vertex0_buffer, vertex1_buffer, _unk_buffer)| {
                    ModelBuffers::load(vertex0_buffer, vertex1_buffer, index_buffer)
                },
            )
            .collect();

        let mut subscribed_stages = model
            .opaque_meshes
            .mesh_groups
            .iter()
            .fold(RenderStageSubscription::empty(), |acc, group| {
                acc | group.render_stage
            });

        let special_meshes = model
            .special_meshes
            .iter()
            .map(|mesh| {
                subscribed_stages |= mesh.render_stage;
                SpecialMesh {
                    mesh: mesh.clone(),
                    buffers: ModelBuffers::load(
                        mesh.vertex0_buffer,
                        mesh.vertex1_buffer,
                        mesh.index_buffer,
                    ),
                    technique: Renderer::instance().asset_manager.load(mesh.technique),
                }
            })
            .collect();

        Ok(Self {
            hash,
            model,
            materials,
            buffers,
            special_meshes,
            subscribed_stages,
        })
    }
}

pub struct StaticInstancesRenderer {
    unk_cb1: ConstantBuffer<Vec4>,
    instance_buffer: ConstantBuffer<u8>,
    instance_id_buffer: VertexBuffer,
    model: StaticModel,
    visible_instance_ids: Vec<u32>,
    transforms: Vec<(SStaticInstanceTransform, AxisAlignedBBox)>,
    bounds: AxisAlignedBBox,
    identifier: u64,

    constants_dirty: bool,
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
pub struct InstanceTransformBlock {
    pub transform: [Vec4; 3],
    pub params0: Vec4,
    pub params1: Vec4,
}

impl StaticInstancesRenderer {
    pub fn new(
        gpu: &Arc<Gpu>,
        transforms: Vec<(SStaticInstanceTransform, AxisAlignedBBox)>,
        model_hash: TagHash,
        identifier: u64,
    ) -> anyhow::Result<Self> {
        let cbuffer = ConstantBuffer::create_raw(
            gpu,
            size_of::<InstanceTransformBlock>() // header + padding
            + transforms.len() * size_of::<InstanceTransformBlock>(), // per-transform data
        )?;

        // Instance IDs dictate from where in the instance buffer to read the transform data. This is calculated as the ID * 0x50 (in bytes).
        // In the past, the engine would skip the 32 bytes where the quantization information was stored, but the offset must now be an exact multiple of 0x40 bytes.
        // cb0[0].x dictates where the quantization information is stored. For now I've opted to just skip the first instance and use that slot for the quantization information.
        let visible_instance_ids = (0..transforms.len() as u32).map(|i| i + 1).collect_vec();

        let instance_id_buffer =
            VertexBuffer::load_data_ex(gpu, bytemuck::cast_slice(&visible_instance_ids), 4, true)?;

        trace!(instances = transforms.len(), model_hash=%model_hash, "Loading model");
        Ok(Self {
            unk_cb1: ConstantBuffer::create(gpu, Some(&Vec4::ZERO))?, // Offsets instance buffer data
            instance_buffer: cbuffer,
            instance_id_buffer,
            model: StaticModel::load(model_hash)?,
            bounds: transforms.iter().map(|(_, b)| b.clone()).sum(),
            visible_instance_ids,
            transforms,
            identifier,
            constants_dirty: true,
        })
    }

    #[profiling::function]
    pub fn render(&self, cmd: &mut CommandList, stage: RenderStage) {
        self.unk_cb1.bind(cmd, ShaderStage::Vertex, 1);
        self.instance_buffer.bind(cmd, ShaderStage::Vertex, 2);
        self.instance_id_buffer.bind_single(cmd, 2);

        let opaque_meshes = &self.model.model.opaque_meshes;
        for (i, group, part) in opaque_meshes
            .mesh_groups
            .iter()
            .enumerate()
            .map(|(i, g)| (i, g, &opaque_meshes.parts[g.part_index as usize]))
            .filter(|(_, g, p)| {
                g.render_stage == stage && p.lod_category.is_second_highest_detail()
            })
        {
            let buffers = &self.model.buffers[part.buffer_index as usize];
            if buffers.bind(cmd).is_none() {
                continue;
            }

            if let Some(technique) = &self.model.materials.get(i).and_then(Handle::get) {
                technique.bind(cmd);
            } else {
                continue;
            }

            cmd.set_input_layout(group.input_layout_index as usize);
            cmd.set_input_topology(part.primitive_type);

            cmd.draw_indexed_instanced(
                part.index_count,
                self.visible_instance_ids.len() as u32,
                part.index_start,
                0,
                0,
            );
        }

        for mesh in self
            .model
            .special_meshes
            .iter()
            .filter(|m| m.mesh.render_stage == stage && m.mesh.lod.is_second_highest_detail())
        {
            if mesh.buffers.bind(cmd).is_none() {
                continue;
            }

            if let Some(technique) = &mesh.technique.get() {
                technique.bind(cmd);
            } else {
                continue;
            }
            cmd.set_input_layout(mesh.input_layout_index as usize);
            cmd.set_input_topology(mesh.primitive_type);

            cmd.draw_indexed_instanced(
                mesh.index_count,
                self.visible_instance_ids.len() as u32,
                mesh.index_start,
                0,
                0,
            );
        }
    }

    #[profiling::function]
    pub fn update_constants(
        &self,
        ctx: &d3d11::DeviceContext,
        // ao: Option<&SStaticAmbientOcclusion>,
    ) {
        let mut buffer = vec![];
        let model = &self.model.model.opaque_meshes;

        buffer
            .write_all(bytemuck::cast_slice(&[
                model.mesh_offset.x,
                model.mesh_offset.y,
                model.mesh_offset.z,
                model.mesh_scale,
                model.texture_coordinate_scale,
                model.texture_coordinate_offset.x,
                model.texture_coordinate_offset.y,
                f32::from_bits(model.max_color_index),
            ]))
            .unwrap();

        while buffer.len() < size_of::<InstanceTransformBlock>() {
            buffer.write_all(&[0u8]).unwrap();
        }

        // let model_transform = Mat4::from_cols_array_2d(&[
        //     [model.mesh_scale, 0.0, 0.0, model.mesh_offset.x],
        //     [0.0, model.mesh_scale, 0.0, model.mesh_offset.y],
        //     [0.0, 0.0, model.mesh_scale, model.mesh_offset.z],
        //     [0.0, 0.0, 0.0, 1.0],
        // ]);
        for (transform, _) in &self.transforms {
            let instance_transform = Mat4::from_scale_rotation_translation(
                Vec3::splat(transform.scale),
                transform.rotation,
                transform.translation,
            )
            .transpose();
            // let instance_transform = Mat4::IDENTITY;

            // let matrix = instance_transform;
            // let vertex_ao_offset = if let Some(vao_base) = vao_base {
            //     transform.vertex_ao_offset + vao_base
            // } else {
            //     transform.vertex_ao_offset
            // };

            buffer
                .write_all(bytemuck::cast_slice(&[InstanceTransformBlock {
                    transform: [
                        instance_transform.x_axis,
                        instance_transform.y_axis,
                        instance_transform.z_axis,
                    ],
                    params0: Vec4::new(
                        1.0,
                        1.0,
                        1.0,
                        f32::from_bits(0x02000000),
                        // f32::from_bits(
                        //     ao_offsets
                        //         .get(i)
                        //         .copied()
                        //         .map(|v| v.shr(2))
                        //         .unwrap_or(0x02000000),
                        // ),
                    ),
                    params1: Vec4::ZERO,
                }]))
                .unwrap();
        }

        unsafe {
            self.instance_buffer.write_array(ctx, &buffer).unwrap();
        }
    }
}

impl FeatureRenderer for StaticInstancesRenderer {
    fn visibility_test(&mut self, camera: &Camera) -> bool {
        if !camera.frustum.aabb_intersecting(&self.bounds) {
            return false;
        }

        let max_dist = ConVars::get("render.max_distance").unwrap_or(f32::INFINITY);

        self.visible_instance_ids.clear();
        for (i, (_, b)) in self.transforms.iter().enumerate() {
            let distance = (camera.position.distance(b.center()) - b.radius()).max(0.0);
            if distance < max_dist && camera.frustum.sphere_intersecting(b.center(), b.radius()) {
                self.visible_instance_ids.push(1 + i as u32);
            }
        }

        !self.visible_instance_ids.is_empty()
    }

    fn extract_and_prepare(
        &mut self,
        renderer: &Renderer,
        _data: &mut dyn super::FeatureRendererData,
        _extracted_data: &dyn std::any::Any,
    ) {
        if self.constants_dirty {
            self.update_constants(
                &renderer.gpu.context(), /*, renderer.ao.read().as_ref() */
            );
            self.constants_dirty = false;
        }
    }

    fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        // Safety: there's never more instances than we allocated space for (hopefully)
        unsafe {
            self.instance_id_buffer
                .write(cmd, bytemuck::cast_slice(&self.visible_instance_ids))
                .unwrap();
        }
        self.render(cmd, stage);
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        self.model.subscribed_stages
    }
}
