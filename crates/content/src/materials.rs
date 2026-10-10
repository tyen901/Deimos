//! Per-draw source material bindings. Geometry identity is deliberately separate.
use crate::{Installation, MaterialBinding, read_at};
use anyhow::{Context, Result, ensure};
use deimos_data::{
    pattern::{ComponentKind, SComponent},
    tfx::{
        RenderStage,
        features::{
            dynamic::{SDynamicModel, SDynamicModelComponent},
            statics::SStaticMesh,
            terrain::STerrain,
        },
    },
};
use tiger_pkg::TagHash;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawSlot {
    StaticGroup(u16),
    StaticSpecial(u16),
    DynamicPart { mesh: u16, part: u16 },
    TerrainPart(u16),
}
pub struct DrawMaterial {
    pub slot: DrawSlot,
    pub stage: RenderStage,
    pub base: Option<u32>,
    /// Alternatives in the source component's permutation order. The caller
    /// selects a permutation; resolution does not substitute a default material.
    pub variants: Vec<Option<u32>>,
    pub variant_table: Option<u16>,
}
pub struct MaterialSet {
    pub owner: MaterialBinding,
    pub draws: Vec<DrawMaterial>,
}
fn optional_tag(tag: TagHash) -> Option<u32> {
    tag.is_some().then_some(tag.0)
}

/// Resolve authored per-part option tables, including inline singleton lists.
pub(crate) fn permutation(
    model: &deimos_data::tfx::features::dynamic::SDynamicModelComponent,
    overrides: &[(u32, u32)],
    default_order: &[&deimos_data::map::S808085E3],
) -> Result<Vec<usize>> {
    use deimos_data::tfx::features::dynamic::MATERIAL_OPTION_INVALID;
    use std::collections::BTreeMap;
    let mut configuration = BTreeMap::new();
    for group in &model.unk38 {
        for value in &group.unk8 {
            if value.value != MATERIAL_OPTION_INVALID {
                configuration.insert(value.switch_key, value.value);
            }
        }
    }
    for &(key, mut value) in overrides {
        if !model.unk38.iter().flat_map(|group| &group.unk8).any(|option| option.switch_key == key) {
            continue; // Pattern options can target other model components.
        }
        if value == deimos_data::hash::FNV1_BASE {
            let defaults = default_order.iter().find(|entry| entry.key == key)
                .with_context(|| format!("Missing authored defaults for material key {key:08X}"))?;
            value = *defaults.values.iter().find(|&&candidate| model.unk38.iter()
                .flat_map(|group| &group.unk8)
                .any(|option| option.switch_key == key && option.value == candidate))
                .with_context(|| format!("No authored default for material key {key:08X}"))?;
        }
        configuration.insert(key, value);
    }
    let mut permutations = Vec::with_capacity(model.technique_map.len());
    for mapping in &model.technique_map {
        let start = mapping.unk8 as usize;
        let entries = model.unk418.get(start..start + mapping.technique_count as usize)
            .context("Part material permutation table range")?;
        let mut candidates = Vec::with_capacity(entries.len());
        let mut owned_options = std::collections::BTreeSet::new();
        for entry in entries {
            // The compact list embeds its sole group index. Only multi-group
            // conditions use an offset into unk408; treating a singleton as an
            // offset aliases unrelated keys in the final per-part tables.
            let singleton = [entry.unk2 as u16];
            let groups: &[u16] = match entry.unk0 {
                0 => &[],
                1 => {
                    ensure!(entry.unk2 >= 0, "Negative singleton material option");
                    &singleton
                },
                count => {
                    ensure!(entry.unk2 >= 0, "Negative material condition list offset");
                    let start = entry.unk2 as usize;
                    model.unk408.get(start..start + count as usize)
                        .context("Part material permutation key range")?
                }
            };
            let mut keys = BTreeMap::new();
            for &group in groups {
                for option in &model.unk38.get(group as usize).context("Part material key")?.unk8 {
                    keys.insert(option.switch_key, option.value);
                    owned_options.insert((option.switch_key, option.value));
                }
            }
            candidates.push(keys);
        }
        // A model option can affect another part only. Project onto this
        // table's actual option pairs, including its authored empty condition.
        let selected: BTreeMap<_, _> = configuration.iter()
            .filter(|(key, value)| owned_options.contains(&(**key, **value)))
            .map(|(&k, &v)| (k, v)).collect();
        // As in the source lookup, a repeated condition is owned by its last
        // table entry. No partial-condition ranking or inferred exclusions.
        let index = candidates.iter().rposition(|keys| *keys == selected)
            .with_context(|| format!("Part material configuration has no exact condition: {selected:?}"))?;
        permutations.push(index);
    }
    Ok(permutations)
}

pub(crate) fn load(installation: &Installation, owner: MaterialBinding) -> Result<MaterialSet> {
    let mut draws = Vec::new();
    match owner {
        MaterialBinding::StaticModel(tag) => {
            let model: SStaticMesh = installation.read_type(tag)?;
            let geometry = installation.follow(&model.opaque_meshes)?;
            for (index, group) in geometry.mesh_groups.iter().enumerate() {
                let part = geometry
                    .parts
                    .get(group.part_index as usize)
                    .context("Static material part index")?;
                if !part.lod_category.is_highest_detail() {
                    continue;
                }
                let technique = model
                    .techniques
                    .get(index)
                    .context("Static material group index")?;
                draws.push(DrawMaterial {
                    slot: DrawSlot::StaticGroup(u16::try_from(index)?),
                    stage: group.render_stage,
                    base: optional_tag(*technique),
                    variants: Vec::new(),
                    variant_table: None,
                });
            }
            for (index, special) in model
                .special_meshes
                .iter()
                .enumerate()
                .filter(|(_, p)| p.lod.is_highest_detail())
            {
                draws.push(DrawMaterial {
                    slot: DrawSlot::StaticSpecial(u16::try_from(index)?),
                    stage: special.render_stage,
                    base: optional_tag(special.technique),
                    variants: Vec::new(),
                    variant_table: None,
                });
            }
        }
        MaterialBinding::RigidComponent(tag) => {
            let bytes = installation.read(tag)?;
            let header: SComponent = read_at(&bytes, 0)?;
            ensure!(
                header.default_instance.resource_type == ComponentKind::RigidModel as u32,
                "Material owner is not a rigid component"
            );
            let component: SDynamicModelComponent = read_at(&bytes, header.definition.offset)?;
            dynamic(
                installation,
                component.model_hash.0,
                Some(&component),
                &mut draws,
            )?;
        }
        MaterialBinding::Model(tag) => dynamic(installation, tag, None, &mut draws)?,
        MaterialBinding::Terrain(tag) => {
            let terrain: STerrain = installation.read_type(tag)?;
            for (index, part) in terrain.mesh_parts.iter().enumerate().filter(|(_, p)| {
                p.detail_level == deimos_data::tfx::features::terrain::TerrainDetailLevel::High
            }) {
                draws.push(DrawMaterial {
                    slot: DrawSlot::TerrainPart(u16::try_from(index)?),
                    stage: RenderStage::GenerateGbuffer,
                    base: optional_tag(part.technique),
                    variants: Vec::new(),
                    variant_table: None,
                });
            }
        }
    }
    Ok(MaterialSet { owner, draws })
}
fn dynamic(
    installation: &Installation,
    tag: u32,
    component: Option<&SDynamicModelComponent>,
    draws: &mut Vec<DrawMaterial>,
) -> Result<()> {
    let model: SDynamicModel = installation.read_type(tag)?;
    for (mesh_index, mesh) in model.meshes.iter().enumerate() {
        for stage in RenderStage::iter() {
            let range = mesh.get_range_for_stage(stage);
            let parts = mesh
                .parts
                .get(range.clone())
                .context("Dynamic material part range")?;
            for (offset, part) in parts
                .iter()
                .enumerate()
                .filter(|(_, p)| p.lod_category.is_highest_detail())
            {
                let mut variants = Vec::new();
                if let Some(component) = component.filter(|_| part.variant_shader_index != u16::MAX)
                {
                    let mapping = component
                        .technique_map
                        .get(part.variant_shader_index as usize)
                        .context("Dynamic material variant mapping")?;
                    ensure!(
                        mapping.technique_count > 0,
                        "Empty material permutation range"
                    );
                    let start = usize::try_from(mapping.technique_start)?;
                    let end = start
                        .checked_add(usize::try_from(mapping.technique_count)?)
                        .context("Material permutation overflow")?;
                    variants.extend(
                        component
                            .techniques
                            .get(start..end)
                            .context("Dynamic material permutation range")?
                            .iter()
                            .map(|tag| optional_tag(*tag)),
                    );
                }
                draws.push(DrawMaterial {
                    slot: DrawSlot::DynamicPart {
                        mesh: u16::try_from(mesh_index)?,
                        part: u16::try_from(range.start + offset)?,
                    },
                    stage,
                    base: optional_tag(part.technique),
                    variants,
                    variant_table: component.filter(|_| part.variant_shader_index != u16::MAX).map(|_| part.variant_shader_index),
                });
            }
        }
    }
    Ok(())
}
