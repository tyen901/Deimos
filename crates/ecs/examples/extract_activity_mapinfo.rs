use std::{
    io::{Cursor, Seek},
    path::PathBuf,
    str::FromStr,
};

use anyhow::Context;
use deimos_data::{activity::SActivity, tag::WideHash, tfx::STechnique, wwise::SWwiseEvent};
use deimos_ecs::{
    interactibles::Pingable,
    transform::Transform,
    world::map::{
        load_activity_for_map_into_world, load_activity_phase_into_world, load_map_into_world,
    },
};
use itertools::Itertools;
use serde::Serialize;
use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{TagHash, package, package_manager};
use tracing::error;

fn main() -> anyhow::Result<()> {
    deimos_core::initialize_package_manager(None)?;
    let activity: SActivity = package_manager().read_tag_struct(0x80A8FF36)?;
    let mut world = hecs::World::new();
    let activity_map = &activity.unk50[0];
    let map_hash = activity_map.map_references[0];

    eprintln!("Loading map {map_hash:?}");
    load_map_into_world(
        map_hash.hash32(),
        &mut world,
        |world, entity, pattern, data, component| {
            Ok(deimos_ecs::world::map::ComponentLoadResult::Skipped)
        },
    )
    .expect("Failed to load map");

    if activity.ambient_activity.is_some()
        && let Err(e) = load_activity_for_map_into_world(
            activity.ambient_activity,
            activity_map.bubble_name,
            &mut world,
            |world, entity, pattern, data, component| {
                Ok(deimos_ecs::world::map::ComponentLoadResult::Skipped)
            },
        )
    {
        eprintln!("Failed to load ambient activity: {e}");
    }

    for unk in &activity_map.unk18 {
        if let Err(e) = load_activity_phase_into_world(
            unk,
            &mut world,
            |world, entity, pattern, data, component| {
                Ok(deimos_ecs::world::map::ComponentLoadResult::Skipped)
            },
        ) {
            eprintln!(
                "Activity phase load for {} failed: {e}",
                unk.unk_entity_reference.taghash()
            );
        }
    }
    eprintln!("Map loaded successfully");

    let mut objects = vec![];
    for (entity, (transform, pingable)) in world.query::<(&Transform, &Pingable)>().iter() {
        let Some(name) = pingable.name.clone() else {
            continue;
        };
        objects.push(NamedObject {
            name,
            transform: *transform,
        });
    }

    let json = serde_json::to_string_pretty(&objects)?;
    std::fs::write("objects.json", json)?;

    Ok(())
}

#[derive(Serialize)]
struct NamedObject {
    pub name: String,
    pub transform: Transform,
}
