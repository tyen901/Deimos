use std::{f32, io::Write, ops::Deref};

use ahash::HashSet;
use anyhow::Context;
use bit_field::BitField;
use bytemuck::{Pod, Zeroable};
use deimos_data::tfx::{
    RenderStage, ShaderStage,
    features::{
        dynamic::RenderStageSubscription,
        statics::{
            SStaticInstanceTransform, SStaticMesh, SStaticMeshData, SStaticMeshInstances,
            SStaticSpecialMesh,
        },
    },
    geometry::AxisAlignedBBox,
};
use glam::{Mat4, Vec3, Vec4};
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};
use tiger_parse::PackageManagerExt;
use tiger_pkg::TagHash;
use tiger_pkg::package_manager;

use crate::{
    asset::{Handle, vertex_buffer::VertexBuffer},
    features::shared::ModelBuffers,
    gpu::{buffer::ImmutableBuffer, command_list::CommandList},
    renderer::{
        Renderer,
        packet::{RenderPerFrameNode, RenderPerViewNode, SubmitNode, SubmitNodeContainer},
    },
    tfx::technique::Technique,
    visibility::{ViewVisibility, bvh::Bvh},
};

use super::FeatureRenderer;

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
    pub materials_by_group: Vec<Handle<Technique>>,
    pub hash: TagHash,
    pub subscribed_stages: RenderStageSubscription,
    buffers: Vec<ModelBuffers>,
    special_meshes: Vec<SpecialMesh>,
}

impl StaticModel {
    #[profiling::function]
    pub fn load(renderer: &Renderer, hash: TagHash) -> anyhow::Result<Self> {
        let model = package_manager().read_tag_struct::<SStaticMesh>(hash)?;
        let materials_by_group = model
            .techniques
            .iter()
            .map(|&tag| renderer.asset_manager.load(tag))
            .collect::<Vec<_>>();

        let buffers = model
            .opaque_meshes
            .buffers
            .iter()
            .map(
                |&(index_buffer, vertex0_buffer, vertex1_buffer, _unk_buffer)| {
                    ModelBuffers::load(renderer, vertex0_buffer, vertex1_buffer, index_buffer)
                },
            )
            .collect::<anyhow::Result<Vec<_>>>()
            .context("loading opaque mesh buffers")?;

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
                Ok(SpecialMesh {
                    mesh: mesh.clone(),
                    buffers: ModelBuffers::load(
                        renderer,
                        mesh.vertex0_buffer,
                        mesh.vertex1_buffer,
                        mesh.index_buffer,
                    )
                    .expect("Failed to load special mesh buffers"),
                    technique: renderer.asset_manager.load(mesh.technique),
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()
            .context("loading special mesh buffers")?;

        Ok(Self {
            hash,
            model,
            materials_by_group,
            buffers,
            special_meshes,
            subscribed_stages,
        })
    }
}

pub struct StaticModelRenderer {
    // unk_cb1: ConstantBuffer<Vec4>,
    instance_buffer: ImmutableBuffer,
    instance_id_buffer: VertexBuffer,
    model: StaticModel,
    visible_instance_ids: Vec<u32>,
    transforms: Vec<(SStaticInstanceTransform, AxisAlignedBBox)>,
    pub bounds: AxisAlignedBBox,
    identifier: u64,

    constants_dirty: bool,

    bvh: Bvh,
    precomputed_submit_nodes: Vec<(RenderStage, SubmitNode)>,
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
pub struct InstanceTransformBlock {
    pub transform: [Vec4; 3], // 0-4, 4-8, 8-12
    pub params0: Vec4,        // 12-16,
    pub params1: Vec4,        // 16-20
}

impl StaticModelRenderer {
    pub fn new(
        renderer: &Renderer,
        transforms: Vec<(SStaticInstanceTransform, AxisAlignedBBox)>,
        model_hash: TagHash,
        identifier: u64,
    ) -> anyhow::Result<Self> {
        let model = StaticModel::load(renderer, model_hash)?;
        let transforms_tmp = transforms.iter().map(|(t, _)| t.clone()).collect_vec();
        let instance_data = Self::generate_constants(&model.model.opaque_meshes, &transforms_tmp);
        let instance_buffer = ImmutableBuffer::new(
            &renderer.gpu,
            "static_geometry::instance_buffer",
            d3d12::Format::R32Uint,
            &instance_data,
        )?;

        // Instance IDs dictate from where in the instance buffer to read the transform data. This is calculated as the ID * 0x50 (in bytes).
        // In the past, the engine would skip the 32 bytes where the quantization information was stored, but the offset must now be an exact multiple of 0x40 bytes.
        // cb0[0].x dictates where the quantization information is stored. For now I've opted to just skip the first instance and use that slot for the quantization information.
        let visible_instance_ids = (0..transforms.len() as u32).map(|i| i + 1).collect_vec();

        let instance_id_buffer = VertexBuffer::load_data_ex(
            &renderer.gpu,
            bytemuck::cast_slice(&visible_instance_ids),
            4,
        )?;

        let bounds = transforms.iter().map(|(_, b)| b.clone()).collect_vec();
        let group_bounds = bounds.iter().cloned().sum();
        let bvh = Bvh::build(&bounds);

        let mut precomputed_submit_nodes = vec![];

        let opaque_meshes = &model.model.opaque_meshes;
        for (group_index, group, _part) in opaque_meshes
            .mesh_groups
            .iter()
            .enumerate()
            .map(|(i, g)| (i, g, &opaque_meshes.parts[g.part_index as usize]))
            .filter(|(_, _, p)| p.lod_category.is_highest_detail())
        {
            if let Some(technique) = model.materials_by_group.get(group_index) {
                let key = StaticSubmitKey {
                    technique: technique.hash().0,
                    model_index: 0, // fixed up in the StaticInstancesRenderer
                    group_index: group_index as u16,
                };
                precomputed_submit_nodes.push((
                    group.render_stage,
                    SubmitNode {
                        key: key.to_u64(),
                        view_node: 0, // fixed up at submit time
                    },
                ));
            }
        }

        trace!(instances = transforms.len(), model_hash=%model_hash, "Loading model");
        Ok(Self {
            // unk_cb1: ConstantBuffer::create(gpu, Some(&Vec4::ZERO))?, // Offsets instance buffer data
            instance_buffer,
            instance_id_buffer,
            model,
            bounds: group_bounds,
            bvh,
            visible_instance_ids,
            transforms,
            identifier,
            constants_dirty: true,
            precomputed_submit_nodes,
        })
    }

    #[profiling::function]
    pub fn render_all(&self, cmd: &mut CommandList, stage: RenderStage) {
        cmd.enable_smart_technique_binding();
        // self.unk_cb1.bind(cmd, ShaderStage::Vertex, 1);
        self.instance_id_buffer.bind_single(cmd, 2);
        self.instance_buffer.bind_srv(cmd, ShaderStage::Vertex, 2);

        let is_opaque = matches!(
            stage,
            RenderStage::ShadowGenerate | RenderStage::DepthPrepass | RenderStage::GenerateGbuffer
        );

        let mut bound_buffer_index = None;
        if is_opaque {
            let opaque_meshes = &self.model.model.opaque_meshes;
            for (i, group, part) in opaque_meshes
                .mesh_groups
                .iter()
                .enumerate()
                .map(|(i, g)| (i, g, &opaque_meshes.parts[g.part_index as usize]))
                .filter(|(_, g, p)| g.render_stage == stage && p.lod_category.is_highest_detail())
            {
                if bound_buffer_index != Some(part.buffer_index) {
                    let buffers = &self.model.buffers[part.buffer_index as usize];
                    if buffers.bind(cmd).is_none() {
                        continue;
                    }
                }
                bound_buffer_index = Some(part.buffer_index);

                cmd.set_input_layout(group.input_layout_index as usize);
                cmd.set_input_topology(part.primitive_type);

                if let Some(technique) = &self.model.materials_by_group.get(i).and_then(|h| h.get())
                {
                    technique.bind(cmd);
                } else {
                    continue;
                }

                cmd.draw_indexed_instanced(
                    part.index_range(),
                    0..self.visible_instance_ids.len() as u32,
                    0,
                );
            }
        }

        if !is_opaque {
            for mesh in self
                .model
                .special_meshes
                .iter()
                .filter(|m| m.mesh.render_stage == stage && m.mesh.lod.is_highest_detail())
            {
                if mesh.buffers.bind(cmd).is_none() {
                    continue;
                }

                cmd.set_input_layout(mesh.input_layout_index as usize);
                cmd.set_input_topology(mesh.primitive_type);
                if let Some(technique) = mesh.technique.get() {
                    technique.bind(cmd);
                } else {
                    continue;
                }

                cmd.draw_indexed_instanced(
                    mesh.index_range(),
                    0..self.visible_instance_ids.len() as u32,
                    0,
                );
            }
        }
    }

    #[profiling::function]
    pub fn render_group(&self, cmd: &mut CommandList, stage: RenderStage, group_index: usize) {
        cmd.enable_smart_technique_binding();
        // self.unk_cb1.bind(cmd, ShaderStage::Vertex, 1);
        self.instance_id_buffer.bind_single(cmd, 2);
        self.instance_buffer.bind_srv(cmd, ShaderStage::Vertex, 2);

        let is_opaque = matches!(
            stage,
            RenderStage::ShadowGenerate | RenderStage::DepthPrepass | RenderStage::GenerateGbuffer
        );

        if is_opaque {
            let opaque_meshes = &self.model.model.opaque_meshes;
            let group = &opaque_meshes.mesh_groups[group_index];
            let part = &opaque_meshes.parts[group.part_index as usize];
            let buffers = &self.model.buffers[part.buffer_index as usize];
            if buffers.bind(cmd).is_none() {
                return;
            }

            cmd.set_input_layout(group.input_layout_index as usize);
            cmd.set_input_topology(part.primitive_type);

            if let Some(technique) = &self
                .model
                .materials_by_group
                .get(group_index)
                .and_then(|h| h.get())
            {
                technique.bind(cmd);
            } else {
                return;
            }

            cmd.draw_indexed_instanced(
                part.index_range(),
                0..self.visible_instance_ids.len() as u32,
                0,
            );
        }
    }

    #[profiling::function]
    fn generate_constants(
        model: &SStaticMeshData,
        transforms: &[SStaticInstanceTransform],
        // ao: Option<&SStaticAmbientOcclusion>,
    ) -> Vec<u8> {
        let mut buffer = vec![];

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
        for transform in transforms {
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
                        f32::from_bits(0), // Quantization block offset
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

        // unsafe {
        //     self.instance_buffer.write_array(ctx, &buffer).unwrap();
        // }
        buffer
    }

    // fn visibility_test(&mut self, camera: &Camera) -> bool {
    //     if !camera.culling_frustum.aabb_intersecting(&self.bounds) {
    //         return false;
    //     }

    //     self.visible_instance_ids.clear();
    //     for (i, (_, b)) in self.transforms.iter().enumerate() {
    //         if camera.is_visible(b) {
    //             self.visible_instance_ids.push(1 + i as u32);
    //         }
    //     }

    //     !self.visible_instance_ids.is_empty()
    // }

    // fn prepare_write_instance_ids(&self, cmd: &DeviceContext) {
    //     // Safety: there's never more instances than we allocated space for (hopefully)
    //     unsafe {
    //         self.instance_id_buffer
    //             .write(cmd, bytemuck::cast_slice(&self.visible_instance_ids))
    //             .unwrap();
    //     }
    // }

    #[profiling::function]
    pub fn is_visible(&self, visibility: &ViewVisibility) -> bool {
        if !visibility.is_visible(&self.bounds) {
            return false;
        }

        self.bvh.has_visible_leaves(visibility)
    }
}

pub struct StaticInstancesRenderer {
    subscribed_stages: RenderStageSubscription,
    /// (model, visible)
    models: Vec<StaticModelRenderer>,
    // (technique_hash, model_index, group_index) sorted by the group's technique hash
    // groups_by_stage_sorted_by_technique: HashMap<RenderStage, Arc<Vec<(TagHash, usize, usize)>>>,
}

impl StaticInstancesRenderer {
    pub fn load_from_tag(renderer: &Renderer, instances_hash: TagHash) -> anyhow::Result<Self> {
        let instances: SStaticMeshInstances = package_manager().read_tag_struct(instances_hash)?;
        println!(
            "Instance collection {instances_hash} has {} occlusion bounds",
            instances.occlusion_bounds.bounds.len()
        );
        let mut models = instances
            .instance_groups
            .par_iter()
            .map(|group| {
                let model = instances.statics[group.static_index as usize];
                let range = (group.instance_start as usize)
                    ..(group.instance_start + group.instance_count) as usize;

                let renderer = StaticModelRenderer::new(
                    renderer,
                    instances.transforms[range.clone()]
                        .iter()
                        .cloned()
                        .zip(
                            instances.occlusion_bounds.bounds[range]
                                .iter()
                                .map(|b| &b.bb)
                                .cloned(),
                        )
                        .collect(),
                    model,
                    instances.vertex_ao_identifier,
                )?;

                Ok(renderer)
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        models
            .iter_mut()
            .enumerate()
            .for_each(|(model_index, model)| {
                for (_, node) in &mut model.precomputed_submit_nodes {
                    let mut key = StaticSubmitKey::from_u64(node.key);
                    key.model_index = model_index as u16;
                    node.key = key.to_u64();
                }
            });

        Ok(Self::new(models))
    }

    pub fn new(models: Vec<StaticModelRenderer>) -> Self {
        Self {
            subscribed_stages: models
                .iter()
                .fold(RenderStageSubscription::empty(), |acc, m| {
                    acc | m.model.subscribed_stages
                }),
            models,
        }
    }
}

impl FeatureRenderer for StaticInstancesRenderer {
    // fn visibility_test(&mut self, camera: &Camera) -> bool {
    //     self.models.par_iter_mut().for_each(|(model, visible)| {
    //         *visible = model.visibility_test(camera);
    //     });
    //     true
    // }

    #[profiling::function]
    fn populate_submit_node_blocks(
        &self,
        _renderer: &Renderer,
        view_node: usize,
        visibility: &ViewVisibility,
        submit_node_blocks: &mut SubmitNodeContainer,
    ) {
        let visible_indices: Vec<usize> = (0..self.models.len())
            .into_par_iter()
            .filter(|&i| self.models[i].is_visible(visibility))
            .collect();

        for model_index in &visible_indices {
            let model = &self.models[*model_index];
            for &(stage, mut node) in &model.precomputed_submit_nodes {
                node.view_node = view_node;
                submit_node_blocks.push(stage, node);
            }
        }
    }

    fn submit(
        &self,
        cmd: &mut CommandList,
        stage: RenderStage,
        _frame_node: &RenderPerFrameNode,
        _view_node: &RenderPerViewNode,
        submit_key: u64,
    ) {
        let key = StaticSubmitKey::from_u64(submit_key);

        self.models[key.model_index as usize].render_group(cmd, stage, key.group_index as usize);
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        self.subscribed_stages
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct StaticSubmitKey {
    pub technique: u32,
    pub model_index: u16,
    pub group_index: u16,
}

impl StaticSubmitKey {
    pub fn from_u64(k: u64) -> Self {
        Self {
            technique: k.get_bits(32..64) as u32,
            model_index: k.get_bits(16..32) as u16,
            group_index: k.get_bits(0..16) as u16,
        }
    }

    pub fn to_u64(self) -> u64 {
        let mut k = 0u64;

        k.set_bits(0..16, self.group_index as u64);
        k.set_bits(16..32, self.model_index as u64);
        k.set_bits(32..64, self.technique as u64);

        k
    }
}
