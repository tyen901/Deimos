use crate::Installation;
use anyhow::Result;
use deimos_data::map::{SBubbleParent, SMapNodeTable};
use std::collections::HashSet;

pub struct WorldEntry {
    pub tag: u32,
    pub name: Option<String>,
}

/// Source node tables, without renderer or ECS ownership. Map instance data is
/// retained alongside each node so component overrides survive traversal.
pub struct World {
    pub tag: u32,
    pub map_name: u32,
    pub tables: Vec<(u32, SMapNodeTable)>,
}

pub(crate) fn load(installation: &Installation, tag: u32) -> Result<World> {
    let parent = installation.read_type::<SBubbleParent>(tag)?;
    let definition = installation.follow(&parent.definition)?;
    let mut tables = Vec::new();
    let mut visited = HashSet::new();
    for container in definition.containers {
        let container = installation.read_type::<deimos_data::map::SMapContainer>(
            installation.resolve(container.taghash())?,
        )?;
        for table in container.data_tables {
            if visited.insert(table) {
                tables.push((table.0, installation.read_type(table.0)?));
            }
        }
    }
    Ok(World {
        tag,
        map_name: parent.map_name,
        tables,
    })
}
