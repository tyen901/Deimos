use anyhow::Context;
use deimos_data::map::{ComponentData, SBubbleParent, SMapNodeTable};
use glam::Vec4Swizzles;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::world::{pattern::spawn_pattern, transform::Transform};

pub fn load_map_into_world(taghash: TagHash, world: &mut hecs::World) -> anyhow::Result<()> {
    let parent = package_manager()
        .read_tag_struct::<SBubbleParent>(taghash)
        .context("Failed to read SBubbleParent")?;
    for resources in &parent.definition.containers {
        for datatable_hash in &resources.data_tables {
            let datatable = package_manager()
                .read_tag_struct::<SMapNodeTable>(*datatable_hash)
                .context("Failed to read SMapNodeTable")?;
            for node in datatable.nodes {
                let transform = Transform::new(
                    node.translation.xyz(),
                    node.rotation,
                    node.translation.www(),
                );

                if let Some(ComponentData::Unknown { class, offset, .. }) =
                    *node.primary_component_data
                {
                    debug!(
                        "Unknown dynamic component data class: {:08X} in {datatable_hash} at offset: {:#X}",
                        class, offset
                    );
                }

                if node.entity.is_none() {
                    anyhow::bail!(
                        "Map data table node with world id {} has no entity. This shouldn't be possible!",
                        node.world_id
                    );
                }

                match spawn_pattern(
                    world,
                    node.entity.hash32(),
                    node.primary_component_data.as_ref(),
                ) {
                    Ok(entity) => {
                        world.insert_one(entity, transform)?;
                    }
                    Err(e) => {
                        error!("Failed to load entity: {:?}", e);
                    }
                }
            }
        }
    }

    Ok(())
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
