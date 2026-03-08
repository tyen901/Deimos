use std::u64;

use deimos_data::tfx::{
    RenderStage, ShaderStage, TfxScopeBits,
    features::dynamic::{
        RenderStageSubscription, SDynamicMesh, SDynamicMeshMaterialVariants, SDynamicMeshPart,
        SDynamicModel,
    },
};
use glam::{Mat4, Vec4};
use itertools::{Itertools, multizip};
use tiger_parse::PackageManagerExt;
use tiger_pkg::TagHash;
use tiger_pkg::package_manager;

use crate::{
    asset::Handle,
    features::FeatureRenderer,
    gpu::command_list::CommandList,
    renderer::{
        Renderer,
        packet::{RenderPerFrameNode, RenderPerViewNode, SubmitNode, SubmitNodeContainer},
    },
    tfx::{expression_vm::interpreter::TempObjectChannels, technique::Technique},
};

use super::shared::ModelBuffers;

pub struct DynamicModel {
    mesh_buffers: Vec<ModelBuffers>,

    technique_map: Vec<SDynamicMeshMaterialVariants>,
    techniques: Vec<Handle<Technique>>,

    pub model: SDynamicModel,
    pub mesh_stages: Vec<RenderStageSubscription>,
    pub subscribed_stages: RenderStageSubscription,
    part_techniques: Vec<Vec<Handle<Technique>>>,

    // pub selected_mesh: usize,
    pub permutation: usize,
    permutation_count: usize,

    identifier_count: usize,

    pub hash: TagHash,

    pub channels: TempObjectChannels,
    pub transform: Mat4,
}

impl DynamicModel {
    #[profiling::function]
    pub fn load(
        renderer: &Renderer,
        hash: TagHash,
        technique_map: Vec<SDynamicMeshMaterialVariants>,
        techniques: Vec<TagHash>,
    ) -> anyhow::Result<Box<Self>> {
        let model = package_manager().read_tag_struct::<SDynamicModel>(hash)?;

        let techniques = techniques
            .iter()
            .map(|&tag| renderer.asset_manager.load(tag))
            .collect_vec();

        let mesh_buffers = model
            .meshes
            .iter()
            .map(|m| {
                ModelBuffers::load(renderer, m.vertex0_buffer, m.vertex1_buffer, m.index_buffer)
                    .expect("Failed to load model buffers for dynamic model")
            })
            .collect_vec();

        let mesh_stages = model
            .meshes
            .iter()
            .map(|m| RenderStageSubscription::from_partrange_list(&m.part_range_per_render_stage))
            .collect_vec();

        let part_techniques = model
            .meshes
            .iter()
            .map(|m| {
                m.parts
                    .iter()
                    .map(|p| renderer.asset_manager.load(p.technique))
                    .collect_vec()
            })
            .collect_vec();

        let permutation_count = technique_map
            .iter()
            .filter(|m| m.unk8 == 0)
            .map(|m| m.technique_count as usize)
            .next()
            .unwrap_or(1);

        let identifier_count = model
            .meshes
            .iter()
            .map(|m| {
                m.parts
                    .iter()
                    .map(|p| p.external_identifier)
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0) as usize
            + 1;

        Ok(Box::new(Self {
            permutation: permutation_count - 1,
            permutation_count,
            // selected_mesh: 0,
            identifier_count,
            mesh_buffers,
            technique_map,
            techniques,
            model,
            subscribed_stages: mesh_stages
                .iter()
                .fold(RenderStageSubscription::empty(), |acc, &x| acc | x),
            mesh_stages,
            part_techniques,
            hash,
            channels: TempObjectChannels::default(),
            transform: Mat4::IDENTITY,
        }))
    }

    pub const fn mesh_count(&self) -> usize {
        self.model.meshes.len()
    }

    pub const fn variant_count(&self) -> usize {
        self.permutation_count
    }

    pub const fn identifier_count(&self) -> usize {
        self.identifier_count
    }

    fn get_permutation_technique(
        &self,
        index: u16,
        permutation_count: usize,
    ) -> Option<Handle<Technique>> {
        if index == u16::MAX {
            None
        } else {
            self.technique_map
                .get(index as usize)
                .as_ref()
                .map(|permutation_range| {
                    self.techniques[permutation_range.technique_start as usize
                        + (permutation_count % permutation_range.technique_count as usize)]
                        .clone()
                })
        }
    }

    // /// ⚠ Expects the `rigid_model` scope to be bound
    // pub fn draw(
    //     &self,
    //     renderer: &Renderer,
    //     render_stage: TfxRenderStage,
    //     identifier: u16,
    //     object_channels: Option<&ObjectChannels>,
    // ) -> anyhow::Result<()> {
    //     self.draw_wrapped(
    //         renderer,
    //         render_stage,
    //         identifier,
    //         object_channels,
    //         |_, renderer, _mesh, part| unsafe {
    //             renderer
    //                 .gpu
    //                 .lock_context()
    //                 .DrawIndexed(part.index_count, part.index_start, 0);
    //         },
    //     )
    // }

    pub fn draw_wrapped<F>(
        &self,
        cmd: &mut CommandList,
        stage: RenderStage,
        identifier: u16,
        mut f: F,
    ) where
        F: FnMut(&Self, &mut CommandList, &SDynamicMesh, &SDynamicMeshPart),
    {
        for (mesh, subscribed_stages, mesh_buffers, mesh_techniques) in multizip((
            self.model.meshes.iter(),
            self.mesh_stages.iter(),
            self.mesh_buffers.iter(),
            self.part_techniques.iter(),
        )) {
            if !subscribed_stages.is_subscribed(stage) {
                continue;
            }

            // self.cb.bind(cmd, ShaderStage::Vertex, 1);
            // self.cb.bind(cmd, ShaderStage::Pixel, 1);

            cmd.set_input_layout(mesh.get_input_layout_for_stage(stage) as usize);
            mesh_buffers.bind(cmd);
            for part_index in mesh.get_range_for_stage(stage) {
                let part = &mesh.parts[part_index];
                if identifier != u16::MAX && part.external_identifier != identifier {
                    continue;
                }

                if !part.lod_category.is_highest_detail() {
                    continue;
                }

                let variant_material =
                    self.get_permutation_technique(part.variant_shader_index, self.permutation);

                let mut all_scopes = TfxScopeBits::empty();
                if let Some(technique) = mesh_techniques[part_index].get() {
                    technique.bind(cmd);
                    // technique
                    //     .bind_with_channels(cmd, Some(&self.channels))
                    //     .expect("Failed to bind technique");
                    all_scopes |= technique.data.used_scopes;
                }

                if let Some(technique) = &variant_material
                    && let Some(technique) = technique.get()
                {
                    technique.bind(cmd);
                    // tech.bind_with_channels(cmd, Some(&self.channels))
                    //     .expect("Failed to bind variant technique");
                    all_scopes |= technique.data.used_scopes;
                }

                // No technique, no scopes, no draw
                if all_scopes.is_empty() {
                    continue;
                }

                // TODO(cohae): Reimplement in pipeline cache
                // if all_scopes.contains(TfxScopeBits::SKINNING) {
                //     cmd.vertex_set_shader(&Renderer::instance().common.disable_skinning_vs);
                // }

                cmd.set_input_topology(part.primitive_type);

                f(self, cmd, mesh, part);
            }
        }
    }
}

impl FeatureRenderer for DynamicModel {
    // fn extract(&mut self, _renderer: &Renderer, data: &dyn std::any::Any) {
    //     let (obj_local_to_world, permutation) = *data
    //         .downcast_ref::<(Mat4, usize)>()
    //         .expect("Invalid extracted data type");
    //     self.transform = obj_local_to_world;
    //     self.permutation = permutation;

    //     self.constants = RigidModelConstants {
    //         mesh_to_world: obj_local_to_world,
    //         position_scale: self.model.model_scale,
    //         position_offset: self.model.model_offset,
    //         texcoord0_scale_offset: Vec4::new(
    //             self.model.texcoord_scale.x,
    //             self.model.texcoord_scale.y,
    //             self.model.texcoord_offset.x,
    //             self.model.texcoord_offset.y,
    //         ),
    //         dynamic_sh_ao_values: Vec4::new(0.0, 0.0, 0.0, 0.8),
    //     };
    // }

    // fn prepare(&mut self, _renderer: &Renderer) {}

    // fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
    //     profiling::scope!("DynamicModel::draw");

    //     self.draw_wrapped(cmd, stage, u16::MAX, |_model, cmd, _mesh, part| {
    //         cmd.draw_indexed_instanced(part.index_range(), 0..1, 0);
    //     });
    // }

    fn prepare_per_frame(&self, cmd: &mut CommandList, frame_node: &RenderPerFrameNode) {
        let data = unsafe { frame_node.data::<DynamicObjectData>() }
            .expect("DynamicModel was extracted with no data!");

        let rigid_model_cb = match cmd.gpu().frame().upload.alloc::<RigidModelConstants>() {
            Ok(o) => o,
            Err(e) => {
                error!("Failed to allocate rigid model constants: {e:?}");
                return;
            }
        };
        rigid_model_cb.write(&RigidModelConstants {
            mesh_to_world: data.local_to_world,
            position_scale: self.model.model_scale,
            position_offset: self.model.model_offset,
            texcoord0_scale_offset: Vec4::new(
                self.model.texcoord_scale.x,
                self.model.texcoord_scale.y,
                self.model.texcoord_offset.x,
                self.model.texcoord_offset.y,
            ),
            dynamic_sh_ao_values: Vec4::new(0.0, 0.0, 0.0, 0.8),
        });

        data.cbuffer_gpuva = rigid_model_cb.virtual_address();
    }

    #[profiling::function]
    fn submit(
        &self,
        cmd: &mut CommandList,
        stage: RenderStage,
        frame_node: &RenderPerFrameNode,
        _view_node: &RenderPerViewNode,
        _submit_key: u64,
    ) {
        cmd.disable_smart_technique_binding();
        let data = unsafe { frame_node.data::<DynamicObjectData>() }
            .expect("DynamicModel was extracted with no data!");

        cmd.set_shader_constant_buffer_view(ShaderStage::Vertex, 1, Some(data.cbuffer_gpuva));
        cmd.set_shader_constant_buffer_view(ShaderStage::Pixel, 1, Some(data.cbuffer_gpuva));

        self.draw_wrapped(cmd, stage, u16::MAX, |_model, cmd, _mesh, part| {
            cmd.draw_indexed_instanced(part.index_range(), 0..1, 0);
        });
    }

    fn populate_submit_node_blocks(
        &self,
        _renderer: &Renderer,
        view_node: usize,
        _visibility: &crate::visibility::ViewVisibility,
        submit_node_blocks: &mut SubmitNodeContainer,
    ) {
        submit_node_blocks.broadcast(
            self.subscribed_stages,
            SubmitNode {
                view_node,
                key: u64::MAX,
            },
        );
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        self.subscribed_stages
    }
}

// impl FeatureRenderer for DynamicModel {
//     fn visibility_test(&mut self, camera: &crate::camera::Camera) -> bool {
//         let bounds = AxisAlignedBBox::from_center_extents(
//             self.model.model_offset.xyz(),
//             self.model.model_scale.xyz() * 2.0,
//         )
//         .transformed(self.transform);

//         camera.is_visible(&bounds)
//     }

//     fn extract_and_prepare(&mut self, renderer: &Renderer, extracted_data: &dyn Any) {
//         let (obj_local_to_world, permutation) = extracted_data
//             .downcast_ref::<(CompactTransform, usize)>()
//             .expect("Invalid extracted data type")
//             .clone();
//         self.transform = obj_local_to_world.to_mat4();
//         self.permutation = permutation;

//         self.cb
//             .write(
//                 &renderer.gpu.context(),
//                 &RigidModel {
//                     mesh_to_world: obj_local_to_world.to_mat4(),
//                     position_scale: self.model.model_scale,
//                     position_offset: self.model.model_offset,
//                     texcoord0_scale_offset: Vec4::new(
//                         self.model.texcoord_scale.x,
//                         self.model.texcoord_scale.y,
//                         self.model.texcoord_offset.x,
//                         self.model.texcoord_offset.y,
//                     ),
//                     dynamic_sh_ao_values: Vec4::new(0.0, 0.0, 0.0, 0.8),
//                 },
//             )
//             .unwrap();

//         self.channels.position = obj_local_to_world.translation().extend(0.0);
//     }

//     // #[profiling::function]
//     fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
//         profiling::scope!("DynamicModel::draw");

//         self.draw_wrapped(cmd, stage, u16::MAX, |_model, cmd, _mesh, part| {
//             cmd.draw_indexed(part.index_count, part.index_start, 0);
//         });
//     }

//     fn subscribed_stages(&self) -> RenderStageSubscription {
//         self.subscribed_stages
//     }
// }

#[repr(C)]
pub struct RigidModelConstants {
    mesh_to_world: Mat4,          // c0-c3
    position_scale: Vec4,         // c4
    position_offset: Vec4,        // c5
    texcoord0_scale_offset: Vec4, // c6
    dynamic_sh_ao_values: Vec4,   // c7
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DynamicObjectData {
    pub local_to_world: Mat4,
    pub permutation: usize,

    pub cbuffer_gpuva: d3d12::GpuVirtualAddress,
}
