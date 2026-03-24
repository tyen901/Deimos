use std::io::{Cursor, Seek, SeekFrom};

use anyhow::Context;
use deimos_data::{
    map::{ComponentData, SComponentDataListPtr},
    pattern::{SComponent, SPattern},
    strings::StringContainer,
    tag::{TagRef, WideHash},
};

use itertools::Itertools;
use tiger_parse::{FnvHash, PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package_manager};
use tracing::{debug, error};

use crate::{
    UnimplementedTigerComponent, UnimplementedTigerComponents, interactibles::Pingable,
    transform::Transform, world::map::ComponentLoadResult,
};

#[macro_export]
macro_rules! once {
    () => {{
        static RAN_ONCE: AtomicBool = AtomicBool::new(false);
        !RAN_ONCE.swap(true, Ordering::SeqCst)
    }};
}

pub fn spawn_pattern<F>(
    world: &mut hecs::World,
    pattern_tag: TagHash,
    map_data_list: Option<&SComponentDataListPtr>,
    transform: Option<Transform>,
    callback: F,
) -> anyhow::Result<hecs::Entity>
where
    F: Fn(
        &mut hecs::World,
        hecs::Entity,
        &SPattern,
        &ComponentData,
        &TagRef<SComponent>,
    ) -> anyhow::Result<ComponentLoadResult>,
{
    let header = package_manager()
        .read_tag_struct::<SPattern>(pattern_tag)
        .context("Failed to read SEntity")?;
    spawn_pattern_from_header(world, &header, map_data_list, transform, callback)
}

pub fn spawn_pattern_from_header<F>(
    world: &mut hecs::World,
    header: &SPattern,
    map_data_list: Option<&SComponentDataListPtr>,
    transform: Option<Transform>,
    callback: F,
) -> anyhow::Result<hecs::Entity>
where
    F: Fn(
        &mut hecs::World,
        hecs::Entity,
        &SPattern,
        &ComponentData,
        &TagRef<SComponent>,
    ) -> anyhow::Result<ComponentLoadResult>,
{
    let entity = world.spawn(());
    if let Some(transform) = transform {
        world.insert_one(entity, transform)?;
    }

    for e in &header.components {
        let mut cur = Cursor::new(package_manager().read_tag(e.component)?);
        let component = TagRef::new(SComponent::read_ds(&mut cur)?, e.component);
        cur.seek(SeekFrom::Start(component.unk18.offset))?;

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

        let Some(dynamic_data) = component.dynamic_data.first() else {
            continue;
        };

        let data = if let Some(data) =
            map_data_list.and_then(|l| l.get_by_class(dynamic_data.data().class_id()))
        {
            data
        } else {
            dynamic_data.data()
        };

        macro_rules! get_component_data {
            ($type:ident) => {
                if let ComponentData::$type(c) = data {
                    c
                } else {
                    error!(
                        "Expected component data type {} for component type 0x{:08X}, found \
                         {}/0x{:08X}",
                        stringify!($type),
                        component.unk10.resource_type,
                        data.class_name(),
                        data.class_id()
                    );
                    continue;
                }
            };
        }

        match callback(world, entity, header, data, &component)
            .context("component load callback")?
        {
            ComponentLoadResult::Loaded => {
                continue;
            }
            ComponentLoadResult::Skipped => {}
        }

        match component.unk10.resource_type {
            0x80802976 => {
                cur.seek(SeekFrom::Start(component.unk18.offset + 0x68))?;
                let hash = FnvHash::read_ds(&mut cur)?;
                cur.seek(SeekFrom::Start(component.unk18.offset + 0x78))?;
                let string_table = WideHash::read_ds(&mut cur)?;

                let container = StringContainer::load(string_table)?;
                let name = container.0.get(&hash).cloned();
                world.insert_one(entity, Pingable { name })?;
            }
            u => {
                debug!(
                    "\t- Unknown entity component type {:08X}, tag {:08X}, data type {:08X}/{} \
                     (table {})",
                    u,
                    component.unk10.resource_type,
                    data.class_id(),
                    data.class_name(),
                    component.taghash()
                );
                if let Some(map_data) = map_data_list {
                    debug!(
                        "\t\t- Has map data ({})",
                        map_data
                            .iter()
                            .enumerate()
                            .map(|(i, c)| format!("[{i}]={}({:08X})", c.class_name(), c.class_id()))
                            .join(", ")
                    );
                }

                if let ComponentData::Unknown { .. } = data {
                } else {
                    error!(
                        "Defined component data type 0x{:X} ({}) was not used while instancing \
                         components! (component class 0x{u:X})",
                        data.class_id(),
                        data.class_name()
                    );
                }

                add_unknown_component!(None);
            }
        }
    }

    Ok(entity)
}
