use std::{f32, io::Write, ops::Deref};

use anyhow::Context;
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
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tiger_parse::PackageManagerExt;
use tiger_pkg::TagHash;
use tiger_pkg::package_manager;

use crate::{
    asset::{Handle, vertex_buffer::VertexBuffer},
    features::shared::ModelBuffers,
    gpu::{buffer::ImmutableBuffer, command_list::CommandList},
    renderer::Renderer,
    tfx::technique::Technique,
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
    pub materials: Vec<Handle<Technique>>,
    pub hash: TagHash,
    pub subscribed_stages: RenderStageSubscription,
    buffers: Vec<ModelBuffers>,
    special_meshes: Vec<SpecialMesh>,
}

impl StaticModel {
    #[profiling::function]
    pub fn load(renderer: &Renderer, hash: TagHash) -> anyhow::Result<Self> {
        let model = package_manager().read_tag_struct::<SStaticMesh>(hash)?;
        let materials = model
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
            materials,
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
    bounds: AxisAlignedBBox,
    identifier: u64,

    constants_dirty: bool,
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

        trace!(instances = transforms.len(), model_hash=%model_hash, "Loading model");
        Ok(Self {
            // unk_cb1: ConstantBuffer::create(gpu, Some(&Vec4::ZERO))?, // Offsets instance buffer data
            instance_buffer,
            instance_id_buffer,
            model,
            bounds: transforms.iter().map(|(_, b)| b.clone()).sum(),
            visible_instance_ids,
            transforms,
            identifier,
            constants_dirty: true,
        })
    }

    #[profiling::function]
    pub fn render_all(&self, cmd: &mut CommandList, stage: RenderStage) {
        // self.unk_cb1.bind(cmd, ShaderStage::Vertex, 1);
        self.instance_id_buffer.bind_single(cmd, 2);
        self.instance_buffer.bind_srv(cmd, ShaderStage::Vertex, 2);

        let is_opaque = matches!(
            stage,
            RenderStage::ShadowGenerate | RenderStage::DepthPrepass | RenderStage::GenerateGbuffer
        );

        if is_opaque {
            let opaque_meshes = &self.model.model.opaque_meshes;
            for (i, group, part) in opaque_meshes
                .mesh_groups
                .iter()
                .enumerate()
                .map(|(i, g)| (i, g, &opaque_meshes.parts[g.part_index as usize]))
                .filter(|(_, g, p)| g.render_stage == stage && p.lod_category.is_highest_detail())
            {
                let buffers = &self.model.buffers[part.buffer_index as usize];
                if buffers.bind(cmd).is_none() {
                    continue;
                }

                cmd.set_input_layout(group.input_layout_index as usize);
                cmd.set_input_topology(part.primitive_type);

                if let Some(technique) = &self.model.materials.get(i).and_then(|h| h.get()) {
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

    // #[profiling::function]
    // pub fn render_group(&self, cmd: &mut CommandList, stage: RenderStage, group: usize) {
    //     // self.unk_cb1.bind(cmd, ShaderStage::Vertex, 1);
    //     // self.instance_buffer.bind(cmd, ShaderStage::Vertex, 2);
    //     self.instance_id_buffer.bind_single(cmd, 2);

    //     let i = group;
    //     let group = &self.model.model.opaque_meshes.mesh_groups[i];
    //     if group.render_stage != stage {
    //         return;
    //     }
    //     let part = &self.model.model.opaque_meshes.parts[group.part_index as usize];
    //     if !part.lod_category.is_highest_detail() {
    //         return;
    //     }

    //     let buffers = &self.model.buffers[part.buffer_index as usize];
    //     if buffers.bind(cmd).is_none() {
    //         return;
    //     }

    //     cmd.set_input_layout(group.input_layout_index as usize);
    //     cmd.set_input_topology(part.primitive_type);

    //     if let Some(technique) = &self.model.materials.get(i).and_then(|h| h.get()) {
    //         technique.bind(cmd);
    //     } else {
    //         return;
    //     }

    //     cmd.draw_indexed_instanced(
    //         part.index_range(),
    //         0..self.visible_instance_ids.len() as u32,
    //         0,
    //     );
    // }

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
}

pub struct StaticInstancesRenderer {
    subscribed_stages: RenderStageSubscription,
    /// (model, visible)
    models: Vec<StaticModelRenderer>,
    // (technique_hash, model_index, group_index) sorted by the group's technique hash
    // groups_by_stage_sorted_by_technique: HashMap<RenderStage, Arc<Vec<(TagHash, usize, usize)>>>,
}

impl StaticInstancesRenderer {
    pub fn load(renderer: &Renderer, instances_hash: TagHash) -> anyhow::Result<Self> {
        let instances: SStaticMeshInstances = package_manager().read_tag_struct(instances_hash)?;
        let models = instances
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

        // let mut groups_by_stage_sorted_by_technique: HashMap<
        //     RenderStage,
        //     Vec<(TagHash, usize, usize)>,
        // > = HashMap::default();
        // for (model_index, (model, _visible)) in models.iter().enumerate() {
        //     for (group_index, (group, technique)) in model
        //         .model
        //         .model
        //         .opaque_meshes
        //         .mesh_groups
        //         .iter()
        //         .zip(model.model.materials.iter())
        //         .enumerate()
        //     {
        //         let part = &model.model.model.opaque_meshes.parts[group.part_index as usize];
        //         if part.lod_category.is_highest_detail() {
        //             groups_by_stage_sorted_by_technique
        //                 .entry(group.render_stage)
        //                 .or_default()
        //                 .push((technique.hash(), model_index, group_index));
        //         }
        //     }
        // }

        // for (_stage, groups_sorted_by_technique) in groups_by_stage_sorted_by_technique.iter_mut() {
        //     groups_sorted_by_technique.sort_unstable_by_key(|k| k.0);
        // }

        // let groups_by_stage_sorted_by_technique = groups_by_stage_sorted_by_technique
        //     .into_iter()
        //     .map(|(k, v)| (k, Arc::new(v)))
        //     .collect();

        Ok(Self {
            subscribed_stages: models
                .iter()
                .fold(RenderStageSubscription::empty(), |acc, m| {
                    acc | m.model.subscribed_stages
                }),
            models,
            // groups_by_stage_sorted_by_technique,
        })
    }
}

impl FeatureRenderer for StaticInstancesRenderer {
    // fn visibility_test(&mut self, camera: &Camera) -> bool {
    //     self.models.par_iter_mut().for_each(|(model, visible)| {
    //         *visible = model.visibility_test(camera);
    //     });
    //     true
    // }

    fn extract(&mut self, _renderer: &Renderer, _data: &dyn std::any::Any) {}

    fn prepare(&mut self, _renderer: &Renderer) {
        // let ctx = renderer.gpu.context();
        // for (model, _visible) in self.models.iter_mut().filter(|(_, visible)| *visible) {
        //     model.prepare_write_instance_ids(&ctx);
        //     if model.constants_dirty {
        //         model.update_constants(
        //             &renderer.gpu.context(), /*, renderer.ao.read().as_ref() */
        //         );
        //         model.constants_dirty = false;
        //     }
        // }
    }

    fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        for model in self.models.iter() {
            model.render_all(cmd, stage);
        }

        // let Some(groups_sorted_by_technique) = self.groups_by_stage_sorted_by_technique.get(&stage)
        // else {
        //     // Special meshes are rendered single-threaded for now
        //     for (model, _visible) in self.models.iter().filter(|(_, v)| *v) {
        //         model.render_all(cmd, stage);
        //     }
        //     return;
        // };

        // let initial_state = Arc::new(GpuState::backup(cmd));

        // // Equally divide groups_sorted_by_technique into X ranges for parallel processing
        // // let mut job_ranges = vec![];
        // let job_count = 6;
        // let node_count = groups_sorted_by_technique.len();
        // let nodes_per_job = node_count / job_count;
        // let mut last_end = 0;
        // let mut jobs_scheduled = 0;
        // for _i in 0..job_count {
        //     let node_start = last_end;
        //     let mut node_end = (node_start + nodes_per_job).min(node_count);

        //     if node_start >= node_count || node_end == 0 {
        //         break;
        //     }

        //     let last_technique = groups_sorted_by_technique[node_end - 1].0;
        //     // Extend node_end to include all groups with the same technique hash
        //     loop {
        //         if node_end < node_count && groups_sorted_by_technique[node_end].0 == last_technique
        //         {
        //             node_end += 1;
        //         } else {
        //             break;
        //         }
        //     }

        //     last_end = node_end;
        //     let range = node_start..node_end;
        //     // job_ranges.push(node_start..node_end);

        //     let groups_sorted_by_technique = groups_sorted_by_technique.clone();
        //     let initial_state = initial_state.clone();
        //     let p_models = &self.models as *const _ as u64;
        //     Renderer::instance()
        //         .cmd_pool
        //         .queue_job(Box::new(move |job_cmd: &mut CommandList| {
        //             // Safety: p_models is valid for the lifetime of this closure
        //             // TODO(cohae): need a better way to pass self.models to the job
        //             let p_models = p_models as *const Vec<(StaticModelRenderer, bool)>;
        //             let models = unsafe { &*p_models };
        //             initial_state.restore(job_cmd);
        //             for (_technique_hash, model_index, group_index) in
        //                 &groups_sorted_by_technique[range.clone()]
        //             {
        //                 let (model, visible) = &models[*model_index];
        //                 if *visible {
        //                     model.render_group(job_cmd, stage, *group_index);
        //                 }
        //             }
        //         }));
        //     jobs_scheduled += 1;
        // }

        // for cmd_result in Renderer::instance()
        //     .cmd_pool
        //     .collect_results(jobs_scheduled)
        // {
        //     cmd.execute_command_list(&cmd_result, true);
        // }

        // let initial_state = Arc::new(GpuState::backup(cmd));
        // let command_lists = job_ranges
        //     .par_iter()
        //     .map(|range| {
        //         let mut cmd = cmd.new_sublist();
        //         initial_state.restore(&mut cmd);
        //         for (_technique_hash, model_index, group_index) in
        //             &groups_sorted_by_technique[range.clone()]
        //         {
        //             let (model, visible) = &self.models[*model_index];
        //             if *visible {
        //                 model.render_group(&mut cmd, stage, *group_index);
        //             }
        //         }

        //         cmd
        //     })
        //     .collect::<Vec<_>>();
        // for command_list in command_lists {
        //     cmd.execute_command_list(&command_list.finish_command_list(false).unwrap(), true);
        // }
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        self.subscribed_stages
    }
}
