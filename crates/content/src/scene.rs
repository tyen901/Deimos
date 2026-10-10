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
            statics::{SStaticMesh, SStaticMeshData, SStaticMeshInstances, SUnk808082D5},
        },
    },
};
use glam::{Mat4, Vec3};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaterialBinding {
    /// Techniques indexed by the shared geometry's mesh-group index.
    StaticModel(u32),
    /// Inline dynamic model definition, including its technique variant tables.
    RigidComponent(u32),
    /// Default techniques stored on dynamic mesh parts (sky, water, decorators).
    Model(u32),
    Terrain(u32),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RenderObjectKey {
    pub mesh: MeshKey,
    pub feature: TfxFeatureRenderer,
    pub materials: MaterialBinding,
    pub technique: Option<u32>,
}
pub struct Scene {
    pub groups: BTreeMap<RenderObjectKey, Vec<Mat4>>,
    pub issues: Vec<String>,
    pub compilation: SceneCompilation,
}
#[derive(Default, Debug)]
pub struct SceneCompilation {
    pub pattern_decode: Duration,
    pub static_decode: Duration,
    pub rigid_decode: Duration,
    pub decorator_decode: Duration,
}
struct Component {
    tag: u32,
    header: SComponent,
    definition: Definition,
}
enum Definition {
    Rigid(u32),
    Attached(Vec<u32>),
    Instance,
}
struct Builder<'a> {
    installation: &'a Installation,
    scene: Scene,
    // A request-local graph compilation table, discarded after traversal.
    // Engine resources, not this table, own long-lived asset reuse.
    patterns: HashMap<u32, Arc<Vec<Component>>>,
    absolute_collections: HashSet<(u32, u32)>,
    visiting: HashSet<u32>,
    statics: HashMap<u32, Vec<RenderObjectKey>>,
    static_geometry: HashMap<u32, Arc<SStaticMeshData>>,
    sky_draws: HashMap<u32, Vec<RenderObjectKey>>,
}

pub(crate) fn load(installation: &Installation, world: &World) -> Result<Scene> {
    let mut builder = Builder {
        installation,
        scene: Scene {
            groups: BTreeMap::new(),
            issues: Vec::new(),
            compilation: SceneCompilation::default(),
        },
        patterns: HashMap::new(),
        absolute_collections: HashSet::new(),
        visiting: HashSet::new(),
        statics: HashMap::new(),
        static_geometry: HashMap::new(),
        sky_draws: HashMap::new(),
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
    fn model(
        &mut self,
        tag: u32,
        feature: TfxFeatureRenderer,
        materials: MaterialBinding,
        transform: Mat4,
    ) {
        for stage in [RenderStage::GenerateGbuffer, RenderStage::Transparents] {
            self.scene
                .groups
                .entry(RenderObjectKey {
                    mesh: MeshKey::Dynamic(tag, stage),
                    feature,
                    materials,
                    technique: None,
                })
                .or_default()
                .push(transform);
        }
    }
    fn sky_model(&mut self, tag: u32, transform: Mat4) -> Result<()> {
        if !self.sky_draws.contains_key(&tag) {
            let model: deimos_data::tfx::features::dynamic::SDynamicModel =
                self.installation.read_type(tag)?;
            let mut draws = Vec::new();
            for (mesh_index, mesh) in model.meshes.iter().enumerate() {
                for stage in [RenderStage::GenerateGbuffer, RenderStage::Transparents] {
                    for part_index in mesh.get_range_for_stage(stage) {
                        let part = &mesh.parts[part_index];
                        if !part.lod_category.is_highest_detail() {
                            continue;
                        }
                        draws.push(RenderObjectKey {
                            mesh: MeshKey::DynamicDraw {
                                tag,
                                stage,
                                mesh: u16::try_from(mesh_index)?,
                                part: u16::try_from(part_index)?,
                            },
                            feature: TfxFeatureRenderer::SkyTransparent,
                            materials: MaterialBinding::Model(tag),
                            technique: Some(part.technique.0),
                        });
                    }
                }
            }
            self.sky_draws.insert(tag, draws);
        }
        for draw in &self.sky_draws[&tag] {
            self.scene.groups.entry(*draw).or_default().push(transform);
        }
        Ok(())
    }
    fn static_meshes(&mut self, tag: u32) -> Result<Vec<RenderObjectKey>> {
        if let Some(keys) = self.statics.get(&tag) {
            return Ok(keys.clone());
        }
        let started = Instant::now();
        let model: SStaticMesh = self.installation.read_type(tag)?;
        let geometry_tag = model.opaque_meshes.taghash().0;
        let geometry = if let Some(geometry) = self.static_geometry.get(&geometry_tag) {
            geometry.clone()
        } else {
            let geometry = Arc::new(self.installation.follow(&model.opaque_meshes)?);
            self.static_geometry.insert(geometry_tag, geometry.clone());
            geometry
        };
        let mut keys = Vec::new();
        for (index, group) in geometry.mesh_groups.iter().enumerate() {
            let part = geometry
                .parts
                .get(group.part_index as usize)
                .context("Static draw part")?;
            if group.render_stage != RenderStage::GenerateGbuffer
                || !part.lod_category.is_highest_detail()
            {
                continue;
            }
            let technique = *model
                .techniques
                .get(index)
                .context("Static group technique")?;
            ensure!(technique.is_some(), "Static color draw has no technique");
            keys.push(RenderObjectKey {
                mesh: MeshKey::StaticDraw {
                    tag: model.opaque_meshes.taghash().0,
                    draw: crate::StaticDrawKey {
                        part: group.part_index,
                        layout: group.input_layout_index,
                    },
                },
                feature: TfxFeatureRenderer::ChunkedInstanceObjects,
                materials: MaterialBinding::StaticModel(tag),
                technique: Some(technique.0),
            });
        }
        for (index, part) in model.special_meshes.iter().enumerate() {
            if part.render_stage == RenderStage::Transparents && part.lod.is_highest_detail() {
                ensure!(
                    part.technique.is_some(),
                    "Static special draw has no technique"
                );
                keys.push(RenderObjectKey {
                    mesh: MeshKey::StaticSpecial(tag, u16::try_from(index)?),
                    feature: TfxFeatureRenderer::ChunkedInstanceObjects,
                    materials: MaterialBinding::StaticModel(tag),
                    technique: Some(part.technique.0),
                });
            }
        }
        self.scene.compilation.static_decode += started.elapsed();
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
            let started = Instant::now();
            let pattern: SPattern = self.installation.read_type(tag)?;
            self.scene.compilation.pattern_decode += started.elapsed();
            let mut components = Vec::with_capacity(pattern.components.len());
            for reference in pattern.components {
                let started = Instant::now();
                let bytes = self.installation.read(reference.component.0)?;
                let header = read_at::<SComponent>(&bytes, 0)?;
                self.scene.compilation.pattern_decode += started.elapsed();
                // Definitions belong to the source component, not its placement.
                // Decode once; material owners still retain the component tag so
                // independent material resources resolve the full variant tables.
                let definition = if header.dynamic_data.first().is_none() {
                    Definition::Instance
                } else if header.default_instance.resource_type == ComponentKind::RigidModel as u32
                {
                    let started = Instant::now();
                    let model: SDynamicModelComponent = read_at(&bytes, header.definition.offset)?;
                    self.scene.compilation.rigid_decode += started.elapsed();
                    Definition::Rigid(model.model_hash.0)
                } else if header.default_instance.resource_type
                    == ComponentKind::AttachedPatterns as u32
                {
                    let attached: S8080A313 = read_at(&bytes, header.definition.offset)?;
                    let mut patterns = Vec::new();
                    for item in attached.unka8.into_iter().flat_map(|a| a.unk8) {
                        if item.pattern.is_some() {
                            patterns.push(self.installation.resolve(item.pattern)?);
                        }
                    }
                    Definition::Attached(patterns)
                } else {
                    Definition::Instance
                };
                components.push(Component {
                    tag: reference.component.0,
                    header,
                    definition,
                });
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
            if let Definition::Rigid(model) = component.definition {
                self.model(
                    model,
                    TfxFeatureRenderer::RigidObject,
                    MaterialBinding::RigidComponent(component.tag),
                    transform,
                );
                continue;
            }
            if let Definition::Attached(patterns) = &component.definition {
                for &pattern in patterns {
                    self.pattern(pattern, None, transform)?;
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
                            self.scene.groups.entry(key).or_default().extend(
                                transforms.iter().map(|t| {
                                    Mat4::from_scale_rotation_translation(
                                        Vec3::splat(t.scale),
                                        t.rotation,
                                        t.translation,
                                    )
                                }),
                            );
                        }
                    }
                }
                ComponentData::SStaticTerrainPatchesComponent(source) => {
                    if self
                        .absolute_collections
                        .insert((data.class_id(), source.terrain.0))
                    {
                        let terrain: deimos_data::tfx::features::terrain::STerrain =
                            self.installation.read_type(source.terrain.0)?;
                        for (part_index, part) in
                            terrain.mesh_parts.iter().enumerate().filter(|(_, p)| {
                                p.detail_level
                                    == deimos_data::tfx::features::terrain::TerrainDetailLevel::High
                            })
                        {
                            ensure!(
                                part.technique.is_some(),
                                "Terrain color part has no technique"
                            );
                            ensure!(
                                (part.group_index as usize) < terrain.mesh_groups.len(),
                                "Terrain part group index"
                            );
                            self.scene
                                .groups
                                .entry(RenderObjectKey {
                                    mesh: MeshKey::TerrainDraw {
                                        tag: source.terrain.0,
                                        part: u16::try_from(part_index)?,
                                    },
                                    feature: TfxFeatureRenderer::TerrainPatch,
                                    materials: MaterialBinding::Terrain(source.terrain.0),
                                    technique: Some(part.technique.0),
                                })
                                .or_default()
                                .push(Mat4::IDENTITY);
                        }
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
                        self.sky_model(model.entity_model.0, object.transform)?;
                    }
                }
                ComponentData::SWaterPlaneComponent(source) => {
                    self.model(
                        source.model.0,
                        TfxFeatureRenderer::Water,
                        MaterialBinding::Model(source.model.0),
                        transform,
                    );
                }
                ComponentData::SDecoratorsComponent(source) => {
                    let started = Instant::now();
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
                                materials: MaterialBinding::Model(model.entity_model.0),
                                technique: None,
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
                    self.scene.compilation.decorator_decode += started.elapsed();
                }
                _ => {}
            }
        }
        Ok(())
    }
}
