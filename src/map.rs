use deimos_data::{
    map::{MapNodeResource, SBubbleParent, SMapNodeTable},
    tfx::features::statics::SUnk808082D5,
};
use deimos_render::{
    feature::{static_geometry::StaticInstancesRenderer, terrain_patches::TerrainPatchesRenderer},
    object::RenderObjectHandle,
    Renderer,
};
use glam::{Quat, Vec3};
use tiger_parse::PackageManagerExt;
use tiger_pkg::{package_manager, TagHash};

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
