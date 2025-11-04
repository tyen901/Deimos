use std::io::{Cursor, Seek, SeekFrom};

use crate::world::{
    UnimplementedTigerComponent, UnimplementedTigerComponents,
    render_objects::{DynamicRenderObject, StaticRenderObject},
    transform::Transform,
};
use anyhow::Context;
use deimos_data::{
    map::ComponentData,
    pattern::SPattern,
    tfx::{
        TfxFeatureRenderer,
        common::AxisAlignedBBox,
        features::{dynamic::SDynamicMeshMaterialVariants, statics::SUnk808082D5},
    },
};
use deimos_render::{
    Renderer,
    feature::{
        decals::DecalCollectionRenderer,
        decorators::DecoratorRenderer,
        rigid_model::DynamicModel,
        static_geometry::{StaticInstancesRenderer, StaticModelRenderer},
        terrain_patches::TerrainPatchesRenderer,
    },
    object::RenderObject,
    tfx::packet::CompactTransform,
};
use glam::Vec4Swizzles;
use tiger_parse::{Endian, PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package_manager};

pub fn spawn_pattern(
    world: &mut hecs::World,
    pattern_tag: TagHash,
    map_data: Option<&ComponentData>,
) -> anyhow::Result<hecs::Entity> {
    let header = package_manager()
        .read_tag_struct::<SPattern>(pattern_tag)
        .context("Failed to read SEntity")?;
    spawn_pattern_from_header(world, &header, map_data)
}

pub fn spawn_pattern_from_header(
    world: &mut hecs::World,
    header: &SPattern,
    map_data: Option<&ComponentData>,
) -> anyhow::Result<hecs::Entity> {
    let renderer = Renderer::instance();

    let entity = world.spawn(());

    for e in &header.components {
        let component = &e.unk0;

        macro_rules! add_unknown_component {
            ($name:expr) => {
                let component = UnimplementedTigerComponent {
                    class_id: component.unk10.resource_type,
                    hash: component.taghash(),
                    name: None,
                };
                if let Ok(mut components) = world.get::<&mut UnimplementedTigerComponents>(entity) {
                    components.0.push(component);
                } else {
                    world.insert_one(entity, UnimplementedTigerComponents(vec![component]))?;
                }
            };
        }

        let data = if let Some(data) = map_data
            && data.class_id() == component.dynamic_data.class_id()
        {
            data
        } else {
            &*component.dynamic_data
        };

        macro_rules! get_component_data {
            ($type:ident) => {
                if let ComponentData::$type(c) = data {
                    c
                } else {
                    error!(
                        "Expected component data type {} for component type 0x{:08X}, found {}/0x{:08X}",
                        stringify!($type),
                        component.unk10.resource_type,
                        data.class_name(),
                        data.class_id()
                    );
                    continue;
                }
            };
        }

        match component.unk10.resource_type {
            0x80808673 => {
                let mut cur = Cursor::new(package_manager().read_tag(component.taghash())?);
                cur.seek(SeekFrom::Start(component.unk18.offset + 0x244))?;
                let model_hash: TagHash = TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                cur.seek(SeekFrom::Start(component.unk18.offset + 0x3e8))?;
                let technique_map: Vec<SDynamicMeshMaterialVariants> =
                    TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                // cur.seek(SeekFrom::Start(entref.unk18.offset + 0x3f0))?;
                // let entity_material_map_pre: Vec<(u16, u16)> =
                //     TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                cur.seek(SeekFrom::Start(component.unk18.offset + 0x428))?;
                let techniques: Vec<TagHash> =
                    TigerReadable::read_ds_endian(&mut cur, Endian::Little)?;

                let model = DynamicModel::load(model_hash, technique_map, techniques)?;
                world.insert_one(
                    entity,
                    AxisAlignedBBox::from_center_extents(
                        model.model.model_offset.xyz(),
                        model.model.model_scale.xyz(),
                    ),
                )?;

                let obj = Renderer::instance().add_object(RenderObject::new(
                    TfxFeatureRenderer::RigidObject,
                    model,
                    Box::new(CompactTransform::IDENTITY),
                ));
                world.insert_one(entity, DynamicRenderObject::new(obj))?;
            }
            0x80808562 => {
                let data = get_component_data!(SStaticTerrainPatchesComponent);

                let renderer =
                    TerrainPatchesRenderer::load(&renderer.gpu, data.terrain, data.identifier)?;

                world.insert_one(
                    entity,
                    StaticRenderObject::new(Renderer::instance().add_object(RenderObject::new(
                        deimos_data::tfx::TfxFeatureRenderer::TerrainPatch,
                        renderer,
                        Box::new(()),
                    ))),
                )?;
            }
            0x808085AF => {
                let data = get_component_data!(SStaticInstancesCollectionComponent);
                let instances: SUnk808082D5 = package_manager().read_tag_struct(data.instances)?;
                world.insert_one(
                    entity,
                    StaticRenderObject::new(renderer.add_object(RenderObject::new(
                        TfxFeatureRenderer::ChunkedInstanceObjects,
                        Box::new(StaticInstancesRenderer::load(
                            &renderer.gpu,
                            instances.instances,
                        )?),
                        Box::new(()),
                    ))),
                );

                // for group in &instances.instances.instance_groups {
                //     let model = instances.instances.statics[group.static_index as usize];
                //     let range = (group.instance_start as usize)
                //         ..(group.instance_start + group.instance_count) as usize;

                //     let renderer = StaticModelRenderer::new(
                //         &renderer.gpu,
                //         instances.instances.transforms[range.clone()]
                //             .iter()
                //             .cloned()
                //             .zip(
                //                 instances.instances.occlusion_bounds.bounds[range]
                //                     .iter()
                //                     .map(|b| &b.bb)
                //                     .cloned(),
                //             )
                //             .collect(),
                //         model,
                //         instances.instances.vertex_ao_identifier,
                //     )?;

                //     // TODO(cohae): It's pretty stupid that we have to spawn a new entity for each group, since it breaks up the pattern structure
                //     world.spawn((StaticRenderObject::new(Renderer::instance().add_object(
                //         RenderObject::new(
                //             deimos_data::tfx::TfxFeatureRenderer::ChunkedInstanceObjects,
                //             Box::new(renderer),
                //             Box::new(()),
                //         ),
                //     )),));
                // }

                // world.insert_one(
                //     entity,
                //     StaticRenderObject::new(Renderer::instance().add_object(RenderObject::new(
                //         deimos_data::tfx::TfxFeatureRenderer::TerrainPatch,
                //         renderer,
                //         Box::new(()),
                //     ))),
                // )?;
            }
            0x80808220 => {
                let data = get_component_data!(SDecalCollectionComponent);
                if let Some(collection) = &*data.decals {
                    let renderer = DecalCollectionRenderer::load(collection.clone())?;
                    world.insert_one(
                        entity,
                        StaticRenderObject::new(Renderer::instance().add_object(
                            RenderObject::new(
                                deimos_data::tfx::TfxFeatureRenderer::DynamicDecals,
                                renderer,
                                Box::new(()),
                            ),
                        )),
                    )?;
                }
            }
            0x808085A9 => {
                let data = get_component_data!(SDecoratorsComponent);
                if let Some(decorators) = data.decorators.0.as_ref() {
                    let renderer = DecoratorRenderer::load(
                        Renderer::instance(),
                        data.decorators.taghash(),
                        decorators.clone(),
                    )?;
                    world.insert_one(
                        entity,
                        StaticRenderObject::new(Renderer::instance().add_object(
                            RenderObject::new(
                                deimos_data::tfx::TfxFeatureRenderer::SpeedtreeTrees,
                                Box::new(renderer),
                                Box::new(()),
                            ),
                        )),
                    )?;
                }
            }
            0x80808377 => {
                let data = get_component_data!(SSkyObjectCollectionComponent);
                let Some(objects) = &*data.objects else {
                    continue;
                };
                for obj in &objects.unk8 {
                    let (scale, rotation, translation) =
                        obj.transform.to_scale_rotation_translation();

                    let render_obj = RenderObject::new(
                        TfxFeatureRenderer::SkyTransparent,
                        DynamicModel::load(obj.model_ref.entity_model, vec![], vec![])?,
                        Box::new(CompactTransform::IDENTITY),
                    );

                    // TODO(cohae): Again, spawning new entities for each object is kinda dumb
                    world.spawn((
                        Transform::new(translation, rotation, scale),
                        DynamicRenderObject::new(Renderer::instance().add_object(render_obj)),
                    ));
                }
            }
            u => {
                debug!(
                    "\t- Unknown entity component type {:08X}, tag {:08X}, data type {:08X}/{} (table {})",
                    u,
                    component.unk10.resource_type,
                    data.class_id(),
                    data.class_name(),
                    component.taghash()
                );
                if let Some(map_data) = map_data {
                    debug!(
                        "\t\t- Has map data ({:08X} / {})",
                        map_data.class_id(),
                        map_data.class_name()
                    );
                }

                if let ComponentData::Unknown { .. } = data {
                } else {
                    error!(
                        "Defined component data type 0x{:X} ({}) was not used while instancing components! (component class 0x{u:X})",
                        data.class_id(),
                        data.class_name()
                    );
                }

                let component = UnimplementedTigerComponent {
                    class_id: component.unk10.resource_type,
                    hash: component.taghash(),
                    name: None,
                };
                if let Ok(mut components) = world.get::<&mut UnimplementedTigerComponents>(entity) {
                    components.0.push(component);
                } else {
                    world.insert_one(entity, UnimplementedTigerComponents(vec![component]))?;
                }
            }
        }
    }

    Ok(entity)
}
