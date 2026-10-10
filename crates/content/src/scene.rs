//! Renderer-independent instancing, following the Deimos component dispatch.
use crate::{Installation, MeshKey, World, read_at};
use anyhow::{Context, Result, ensure};
use deimos_data::{
    map::ComponentData,
    pattern::{ComponentKind, S8080A313, SComponent, SPattern},
    tfx::{
        RenderStage, TfxFeatureRenderer,
        features::{
            decorators::DecoratorGpuInstance,
            dynamic::SDynamicModelComponent,
            statics::{SStaticMesh, SStaticMeshInstances, SUnk808082D5},
        },
    },
};
use glam::{Mat4, Vec3};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderObjectKey {
    pub mesh: MeshKey,
    pub feature: TfxFeatureRenderer,
}
pub struct Scene {
    pub groups: BTreeMap<RenderObjectKey, Vec<Mat4>>,
    pub issues: Vec<String>,
}
struct Component {
    header: SComponent,
    bytes: Vec<u8>,
}
struct Builder<'a> {
    installation: &'a Installation,
    scene: Scene,
    // A request-local graph compilation table, discarded after traversal.
    // Engine resources, not this table, own long-lived asset reuse.
    patterns: HashMap<u32, Arc<Vec<Component>>>,
    absolute_collections: HashSet<(u32, u32)>,
    visiting: HashSet<u32>,
    statics: HashMap<u32, Vec<MeshKey>>,
}

pub(crate) fn load(installation: &Installation, world: &World) -> Result<Scene> {
    let mut builder = Builder {
        installation,
        scene: Scene {
            groups: BTreeMap::new(),
            issues: Vec::new(),
        },
        patterns: HashMap::new(),
        absolute_collections: HashSet::new(),
        visiting: HashSet::new(),
        statics: HashMap::new(),
    };
    for (table, source) in &world.tables {
        for node in &source.nodes {
            let tag = installation
                .resolve(node.entity)
                .with_context(|| format!("Table {table:08X} node {}", node.world_id))?;
            let transform = Mat4::from_scale_rotation_translation(
                Vec3::splat(node.translation.w),
                node.rotation,
                node.translation.truncate(),
            );
            if let Err(error) = builder.pattern(tag, Some(&node.component_data), transform) {
                builder.scene.issues.push(format!(
                    "Table {table:08X} node {}: {error:#}",
                    node.world_id
                ));
            }
        }
    }
    Ok(builder.scene)
}

impl Builder<'_> {
    fn model(&mut self, tag: u32, feature: TfxFeatureRenderer, transform: Mat4) {
        for stage in [RenderStage::GenerateGbuffer, RenderStage::Transparents] {
            self.scene
                .groups
                .entry(RenderObjectKey {
                    mesh: MeshKey::Dynamic(tag, stage),
                    feature,
                })
                .or_default()
                .push(transform);
        }
    }
    fn static_meshes(&mut self, tag: u32) -> Result<Vec<MeshKey>> {
        if let Some(keys) = self.statics.get(&tag) {
            return Ok(keys.clone());
        }
        let model: SStaticMesh = self.installation.read_type(tag)?;
        let mut keys = vec![MeshKey::Static(model.opaque_meshes.taghash().0)];
        for (index, part) in model.special_meshes.iter().enumerate() {
            if part.render_stage == RenderStage::Transparents && part.lod.is_highest_detail() {
                keys.push(MeshKey::StaticSpecial(tag, u16::try_from(index)?));
            }
        }
        self.statics.insert(tag, keys.clone());
        Ok(keys)
    }

    fn pattern(
        &mut self,
        tag: u32,
        overrides: Option<&deimos_data::map::SComponentDataListPtr>,
        transform: Mat4,
    ) -> Result<()> {
        ensure!(
            self.visiting.insert(tag),
            "Cyclic attached pattern {tag:08X}"
        );
        let result = self.pattern_contents(tag, overrides, transform);
        self.visiting.remove(&tag);
        result
    }
    fn pattern_contents(
        &mut self,
        tag: u32,
        overrides: Option<&deimos_data::map::SComponentDataListPtr>,
        transform: Mat4,
    ) -> Result<()> {
        let components = if let Some(components) = self.patterns.get(&tag) {
            components.clone()
        } else {
            let pattern: SPattern = self.installation.read_type(tag)?;
            let mut components = Vec::with_capacity(pattern.components.len());
            for reference in pattern.components {
                let bytes = self.installation.read(reference.component.0)?;
                let header = read_at::<SComponent>(&bytes, 0)?;
                components.push(Component { header, bytes });
            }
            let components = Arc::new(components);
            self.patterns.insert(tag, components.clone());
            components
        };
        for component in components.iter() {
            let header = &component.header;
            let Some(default) = header.dynamic_data.first() else {
                continue;
            };
            if header.default_instance.resource_type == ComponentKind::RigidModel as u32 {
                let model: SDynamicModelComponent =
                    read_at(&component.bytes, header.definition.offset)?;
                self.model(
                    model.model_hash.0,
                    TfxFeatureRenderer::RigidObject,
                    transform,
                );
                continue;
            }
            if header.default_instance.resource_type == ComponentKind::AttachedPatterns as u32 {
                let attached: S8080A313 = read_at(&component.bytes, header.definition.offset)?;
                for item in attached.unka8.into_iter().flat_map(|a| a.unk8) {
                    if item.pattern.is_some() {
                        self.pattern(self.installation.resolve(item.pattern)?, None, transform)?;
                    }
                }
                continue;
            }
            let data = match overrides.and_then(|o| o.get_by_class(default.data().class_id())) {
                Some(instance) => instance,
                None => default.data(),
            };
            match data {
                ComponentData::SStaticInstancesCollectionComponent(source) => {
                    if !self
                        .absolute_collections
                        .insert((data.class_id(), source.instances.0))
                    {
                        continue;
                    }
                    let wrapper: SUnk808082D5 = self.installation.read_type(source.instances.0)?;
                    let instances: SStaticMeshInstances =
                        self.installation.read_type(wrapper.instances.0)?;
                    for group in &instances.instance_groups {
                        let model = *instances
                            .statics
                            .get(group.static_index as usize)
                            .context("Static group model index")?;
                        let end = group
                            .instance_start
                            .checked_add(group.instance_count)
                            .context("Static instance range overflow")?;
                        let transforms = instances
                            .transforms
                            .get(group.instance_start as usize..end as usize)
                            .context("Static instance range")?;
                        let keys = self.static_meshes(model.0)?;
                        for key in keys {
                            self.scene
                                .groups
                                .entry(RenderObjectKey {
                                    mesh: key,
                                    feature: TfxFeatureRenderer::ChunkedInstanceObjects,
                                })
                                .or_default()
                                .extend(transforms.iter().map(|t| {
                                    Mat4::from_scale_rotation_translation(
                                        Vec3::splat(t.scale),
                                        t.rotation,
                                        t.translation,
                                    )
                                }));
                        }
                    }
                }
                ComponentData::SStaticTerrainPatchesComponent(source) => {
                    if self
                        .absolute_collections
                        .insert((data.class_id(), source.terrain.0))
                    {
                        self.scene
                            .groups
                            .entry(RenderObjectKey {
                                mesh: MeshKey::Terrain(source.terrain.0),
                                feature: TfxFeatureRenderer::TerrainPatch,
                            })
                            .or_default()
                            .push(Mat4::IDENTITY);
                    }
                }
                ComponentData::SSkyObjectCollectionComponent(source) => {
                    let tag = source.objects.taghash().0;
                    if tag == 0
                        || tag == u32::MAX
                        || !self.absolute_collections.insert((data.class_id(), tag))
                    {
                        continue;
                    }
                    let objects: deimos_data::tfx::features::sky_objects::SSkyObjectCollection =
                        self.installation.read_type(tag)?;
                    for object in objects.unk8.iter().filter(|o| o.is_game_sky()) {
                        let model = self.installation.follow(&object.model_ref)?;
                        self.model(
                            model.entity_model.0,
                            TfxFeatureRenderer::SkyTransparent,
                            object.transform,
                        );
                    }
                }
                ComponentData::SWaterPlaneComponent(source) => {
                    self.model(source.model.0, TfxFeatureRenderer::Water, transform);
                }
                ComponentData::SDecoratorsComponent(source) => {
                    let tag = source.decorators.taghash().0;
                    if !self.absolute_collections.insert((data.class_id(), tag)) {
                        continue;
                    }
                    let decorator: deimos_data::tfx::features::decorators::SDecorator =
                        self.installation.read_type(tag)?;
                    ensure!(
                        decorator.unk8.len() == 1,
                        "Multi-model decorator selection is unresolved: {tag:08X}"
                    );
                    let model = self.installation.follow(&decorator.unk8[0])?;
                    ensure!(
                        model.unk14.taghash().0 == u32::MAX || model.unk14.taghash().0 == 0,
                        "Speedtree vertex reconstruction is unresolved: {tag:08X}"
                    );
                    let placement = self.installation.follow(&decorator.unk48)?;
                    let constants = self.installation.follow(&placement.unk14)?;
                    let header: deimos_data::tfx::buffers::VertexBufferHeader =
                        self.installation.read_type(placement.instance_buffer.0)?;
                    ensure!(
                        usize::from(header.stride) == DecoratorGpuInstance::SIZE,
                        "Decorator GPU instance stride changed"
                    );
                    let bytes = self
                        .installation
                        .read(self.installation.reference(placement.instance_buffer.0)?)?;
                    ensure!(
                        bytes.len() == header.data_size as usize,
                        "Decorator instance payload size mismatch"
                    );
                    ensure!(
                        decorator.unk18.last().copied()
                            == Some((bytes.len() / DecoratorGpuInstance::SIZE) as u32),
                        "Decorator ranges do not cover instance buffer"
                    );
                    for (identifier, range) in decorator.unk18.windows(2).enumerate() {
                        let records = bytes
                            .get(
                                range[0] as usize * DecoratorGpuInstance::SIZE
                                    ..range[1] as usize * DecoratorGpuInstance::SIZE,
                            )
                            .context("Decorator instance range")?;
                        let group = self
                            .scene
                            .groups
                            .entry(RenderObjectKey {
                                mesh: MeshKey::Decorator(
                                    model.entity_model.0,
                                    u16::try_from(identifier)?,
                                ),
                                feature: TfxFeatureRenderer::SpeedtreeTrees,
                            })
                            .or_default();
                        for record in records.chunks_exact(DecoratorGpuInstance::SIZE) {
                            let instance = DecoratorGpuInstance::from_bytes(record);
                            let p = instance.position_scale(&constants);
                            let q = instance.rotation(&constants);
                            // Retail shader uses the reconstructed quaternion directly.
                            let u = q.truncate();
                            let rotate = |v: Vec3| {
                                (q.w * q.w - u.length_squared()) * v
                                    + 2.0 * u * u.dot(v)
                                    + 2.0 * q.w * u.cross(v)
                            };
                            group.push(Mat4::from_cols(
                                (rotate(Vec3::X) * p.w).extend(0.0),
                                (rotate(Vec3::Y) * p.w).extend(0.0),
                                (rotate(Vec3::Z) * p.w).extend(0.0),
                                p.truncate().extend(1.0),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}
