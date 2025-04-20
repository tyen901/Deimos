use std::{
    io::{Cursor, Seek, SeekFrom, Write},
    ops::{Deref, Shr},
    sync::Arc,
};

use anyhow::Context;
use deimos_data::{
    map::{MapNodeResource, SBubbleParent, SMapNodeTable},
    tfx::{
        common::AxisAlignedBBox,
        features::{
            dynamic::{RenderStageSubscription, SDynamicMeshMaterialVariants},
            statics::{SStaticInstanceTransform, SStaticMesh, SStaticSpecialMesh, SUnk808082D5},
        },
        RenderStage, TfxFeatureRenderer,
    },
};
use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use itertools::Itertools;
use tiger_parse::{Endian, PackageManagerExt, TigerReadable};
use tiger_pkg::package_manager;
use tiger_pkg::TagHash;

use crate::{
    asset::{vertex_buffer::VertexBuffer, Handle},
    gpu::{cbuffer::ConstantBuffer, command_list::CommandList, ShaderStage},
    object::{RenderObject, RenderObjectHandle},
    tfx::{packet::CompactTransform, technique::Technique},
    Gpu, Renderer,
};

use super::{shared::ModelBuffers, terrain_patches::TerrainPatchesRenderer, FeatureRenderer};

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
    instance_buffer: ConstantBuffer<u8>,
    instance_id_buffer: VertexBuffer,
    model: StaticModel,
    visible_instance_ids: Vec<u32>,
    transforms: Vec<(SStaticInstanceTransform, AxisAlignedBBox)>,
    bounds: AxisAlignedBBox,
    identifier: u64,

    constants_dirty: bool,
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
            transforms.len() * size_of::<Mat4>() + 4 * size_of::<Vec4>(),
        )?;

        // Instance IDs dictate from where in the instance buffer to read the transform data. This is calculated as the ID * 0x40 (in bytes).
        // In the past, the engine would skip the 32 bytes where the quantization information was stored, but the offset must now be an exact multiple of 0x40 bytes.
        // cb0[0].x dictates where the quantization information is stored. For now I've opted to just skip the first instance and use that slot for the quantization information.
        let visible_instance_ids = (0..transforms.len() as u32).map(|i| i + 1).collect_vec();

        let instance_id_buffer =
            VertexBuffer::load_data_ex(gpu, bytemuck::cast_slice(&visible_instance_ids), 4, true)?;

        trace!(instances = transforms.len(), model_hash=%model_hash, "Loading model");
        Ok(Self {
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
        self.instance_buffer.bind(cmd, ShaderStage::Vertex, 2);

        let opaque_meshes = &self.model.model.opaque_meshes;
        for (i, group) in opaque_meshes
            .mesh_groups
            .iter()
            .enumerate()
            .filter(|(_, g)| g.render_stage == stage)
        {
            let part = &opaque_meshes.parts[group.part_index as usize];
            if !part.lod_category.is_highest_detail() {
                continue;
            }

            let buffers = &self.model.buffers[part.buffer_index as usize];
            if buffers.bind(cmd).is_none() {
                continue;
            }
            self.instance_id_buffer.bind_single(cmd, 2);

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
            .filter(|m| m.mesh.render_stage == stage && m.mesh.lod.is_highest_detail())
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

        // Quantization block padding
        buffer.write_all(&[0u8; 32]).unwrap();

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
                .write_all(bytemuck::cast_slice(&[
                    instance_transform.x_axis,
                    instance_transform.y_axis,
                    instance_transform.z_axis,
                    Vec4::new(
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
                ]))
                .unwrap();
        }

        unsafe {
            self.instance_buffer.write_array(ctx, &buffer).unwrap();
        }
    }
}

pub struct StaticMapTemp {
    pub models: Vec<StaticInstancesRenderer>,
    pub terrain: Vec<TerrainPatchesRenderer>,
    // pub decals: Vec<Box<DecalCollectionRenderer>>,
    pub cubemaps: Vec<RenderObjectHandle>,
    // pub collision_hkx: Option<SStaticMapCollision>,
    pub entities: Vec<(Vec3, Quat, Vec3, RenderObjectHandle, TagHash, String)>,
}

pub fn load_static_map(taghash: TagHash) -> anyhow::Result<StaticMapTemp> {
    let mut map = StaticMapTemp {
        models: Vec::new(),
        terrain: Vec::new(),
        // decals: Vec::new(),
        cubemaps: Vec::new(),
        // collision_hkx: None,
        entities: Vec::new(),
    };

    let gpu = Renderer::instance().gpu.clone();
    let parent = package_manager().read_tag_struct::<SBubbleParent>(taghash)?;
    for resources in &parent.definition.containers {
        for datatable_hash in &resources.data_tables {
            let datatable = package_manager().read_tag_struct::<SMapNodeTable>(*datatable_hash)?;
            for node in datatable.data_entries {
                if let Some(ref resource) = *node.data_resource {
                    match resource {
                        MapNodeResource::SStaticInstancesCollectionComponent(c) => {
                            let instances: SUnk808082D5 =
                                package_manager().read_tag_struct(c.instances)?;

                            for group in &instances.instances.instance_groups {
                                let model =
                                    instances.instances.statics[group.static_index as usize];
                                let range = (group.instance_start as usize)
                                    ..(group.instance_start + group.instance_count) as usize;

                                let renderer = StaticInstancesRenderer::new(
                                    &gpu,
                                    instances.instances.transforms[range.clone()]
                                        .iter()
                                        .cloned()
                                        .zip(
                                            instances.instances.occlusion_bounds.bounds[range]
                                                .iter()
                                                .map(|b| &b.bb)
                                                .cloned(),
                                        )
                                        .collect(),
                                    model,
                                    instances.instances.vertex_ao_identifier,
                                )?;
                                map.models.push(renderer);
                            }
                        }
                        MapNodeResource::SStaticTerrainPatchesComponent(terrain) => {
                            let renderer = TerrainPatchesRenderer::load(
                                &gpu,
                                terrain.terrain,
                                terrain.identifier,
                            )?;
                            map.terrain.push(renderer);
                        }
                        // MapNodeResource::SStaticAmbientOcclusionComponent(ao) => {
                        //     let renderer = Renderer::instance();
                        //     *renderer.ao.write() = Some((*ao.ao).clone());
                        //     *renderer.ao_buffer.lock() =
                        //         Some(Renderer::instance().asset_manager.load(ao.ao.ao0.buffer));
                        // }
                        // MapNodeResource::SStaticMapCollisionComponent(collision) => {
                        //     map.collision_hkx = Some((*collision.collision).clone());
                        // }
                        // MapNodeResource::SSkyObjectCollectionComponent(s) => {
                        //     for obj in &s.objects.unk8 {
                        //         let (scale, rotation, translation) =
                        //             obj.transform.to_scale_rotation_translation();

                        //         let render_obj =
                        //             Renderer::instance().add_object(RenderObject::new(
                        //                 TfxFeatureRenderer::SkyTransparent,
                        //                 DynamicModel::load(
                        //                     obj.model_ref.entity_model,
                        //                     vec![],
                        //                     vec![],
                        //                 )?,
                        //                 Box::new(CompactTransform::IDENTITY),
                        //             ));

                        //         map.entities.push((
                        //             translation,
                        //             rotation,
                        //             scale,
                        //             render_obj,
                        //             TagHash::NONE,
                        //             format!("Sky Object {}", obj.model_ref.entity_model),
                        //         ));
                        //     }
                        // }
                        // MapNodeResource::SCubemapComponent(c) => {
                        //     let ct = CompactTransform::from_mat4(Mat4::from_rotation_translation(
                        //         node.rotation,
                        //         node.translation.xyz(),
                        //     ));
                        //     let render_obj = Renderer::instance().add_object(RenderObject::new(
                        //         TfxFeatureRenderer::Cubemaps,
                        //         Box::new(CubemapRenderer::load(&gpu, c)?),
                        //         Box::new(ct),
                        //     ));

                        //     map.cubemaps.push(render_obj);
                        // }
                        // MapNodeResource::SLightCollectionComponent(l) => {
                        //     for (i, (light, transform)) in l
                        //         .lights
                        //         .lights
                        //         .iter()
                        //         .zip(l.lights.transforms.iter())
                        //         .enumerate()
                        //     {
                        //         map.entities.push((
                        //             transform.translation.xyz(),
                        //             transform.rotation,
                        //             Vec3::ONE,
                        //             Renderer::instance().add_object(RenderObject::new(
                        //                 TfxFeatureRenderer::ChunkedLights,
                        //                 LightRenderer::new(&Renderer::instance(), light)
                        //                     .context("Failed to load light")?,
                        //                 Box::new(CompactTransform::IDENTITY),
                        //             )),
                        //             TagHash::NONE,
                        //             format!("Light {}+{}", l.lights.taghash(), i),
                        //         ));
                        //     }
                        // }
                        // MapNodeResource::SShadowingLightComponent(l) => {
                        //     map.entities.push((
                        //         node.translation.xyz(),
                        //         node.rotation,
                        //         Vec3::splat(node.translation.w),
                        //         Renderer::instance().add_object(RenderObject::new(
                        //             TfxFeatureRenderer::DeferredLights,
                        //             LightRenderer::new_shadowing(&Renderer::instance(), &l.light)
                        //                 .context("Failed to load shadowing light")?,
                        //             Box::new(CompactTransform::IDENTITY),
                        //         )),
                        //         TagHash::NONE,
                        //         format!("Shadowing Light {}", l.light.taghash()),
                        //     ));
                        // }
                        // MapNodeResource::SWaterPlaneComponent(w) => {
                        //     map.entities.push((
                        //         node.translation.xyz(),
                        //         node.rotation,
                        //         Vec3::splat(node.translation.w),
                        //         Renderer::instance().add_object(RenderObject::new(
                        //             TfxFeatureRenderer::Water,
                        //             DynamicModel::load(w.model, vec![], vec![])
                        //                 .context("Failed to load water plane model")?,
                        //             Box::new(CompactTransform::IDENTITY),
                        //         )),
                        //         TagHash::NONE,
                        //         format!("Water Plane {}", w.model),
                        //     ));
                        // }
                        // MapNodeResource::SDecalCollectionComponent(d) => {
                        //     if d.decals.is_none() {
                        //         continue;
                        //     }
                        //     let renderer = DecalCollectionRenderer::load(d.decals)?;
                        //     map.decals.push(renderer);
                        // }
                        MapNodeResource::Unknown { class, offset } => {
                            warn!(
                                "Unknown resource class: {:08X} in {datatable_hash} at offset: {:#X}",
                                class, offset
                            );
                        }
                    }
                }

                // if node.entity.is_some() {
                //     match get_entity_render_object(node.entity) {
                //         Ok((Some(o), collider)) => {
                //             map.entities.push((
                //                 node.translation.xyz(),
                //                 node.rotation,
                //                 Vec3::splat(node.translation.w),
                //                 o,
                //                 collider,
                //                 format!("Entity {}", node.entity),
                //             ));
                //         }
                //         Ok(_) => {}
                //         Err(e) => {
                //             error!("Failed to load entity render object: {:?}", e);
                //         }
                //     }
                // }
            }
        }
    }
    Ok(map)
}

// fn get_entity_render_object(
//     entity_hash: TagHash,
// ) -> anyhow::Result<(Option<RenderObjectHandle>, TagHash)> {
//     let header = package_manager()
//         .read_tag_struct::<SEntity>(entity_hash)
//         .context("Failed to read SEntity")?;

//     let mut render_obj = None;
//     let mut collider = TagHash::NONE;
//     for e in &header.entity_resources {
//         let entres = &e.unk0;

//         match entres.unk10.resource_type {
//             // s_physics_component
//             0x808092d8 => {
//                 let mut cur = Cursor::new(package_manager().read_tag(entres.taghash())?);
//                 cur.seek(SeekFrom::Start(entres.unk18.offset + 0x380))?;
//                 collider = TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;
//             }
//             0x808072b8 => {
//                 let mut cur = Cursor::new(package_manager().read_tag(entres.taghash())?);
//                 cur.seek(SeekFrom::Start(entres.unk18.offset + 0x1dc))?;
//                 let model_hash: TagHash = TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

//                 cur.seek(SeekFrom::Start(entres.unk18.offset + 0x2d0))?;
//                 let technique_map: Vec<SDynamicMeshMaterialVariants> =
//                     TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

//                 // cur.seek(SeekFrom::Start(entref.unk18.offset + 0x3f0))?;
//                 // let entity_material_map_pre: Vec<(u16, u16)> =
//                 //     TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

//                 cur.seek(SeekFrom::Start(entres.unk18.offset + 0x310))?;
//                 let techniques: Vec<TagHash> =
//                     TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

//                 let obj = Renderer::instance().add_object(RenderObject::new(
//                     TfxFeatureRenderer::RigidObject,
//                     DynamicModel::load(model_hash, technique_map, techniques)?,
//                     Box::new(CompactTransform::IDENTITY),
//                 ));

//                 render_obj = Some(obj);
//                 // scene.insert(
//                 //     scene_entity,
//                 //     (
//                 //         DynamicModelComponent::load(
//                 //             renderer,
//                 //             &transform,
//                 //             model_hash,
//                 //             entity_material_map,
//                 //             materials,
//                 //             TfxFeatureRenderer::DynamicObjects,
//                 //         )?,
//                 //         TfxFeatureRenderer::DynamicObjects,
//                 //     ),
//                 // )?;
//             }
//             u => {
//                 debug!(
//                     "\t- Unknown entity resource type {:08X}/{:08X} (table {})",
//                     u.to_be(),
//                     entres.unk10.resource_type.to_be(),
//                     entres.taghash()
//                 )
//             }
//         }
//     }

//     Ok((render_obj, collider))
// }

impl FeatureRenderer for StaticInstancesRenderer {
    fn visibility_test(&mut self, frustum: &crate::visibility::frustum::Frustum) -> bool {
        if !frustum.aabb_intersecting(&self.bounds) {
            return false;
        }

        self.visible_instance_ids.clear();
        for (i, (_, b)) in self.transforms.iter().enumerate() {
            if frustum.aabb_intersecting(b) {
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
