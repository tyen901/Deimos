//! Engine-independent geometry. Source draw ranges, quantization and vertex
//! channels come from Deimos; the embedding engine owns material resources.
use crate::Installation;
use anyhow::{Context, Result, bail, ensure};
use deimos_data::dxgi::Format;
use deimos_data::tfx::{
    PrimitiveType, RenderStage,
    buffers::{IndexBufferHeader, VertexBufferHeader},
    features::{
        dynamic::SDynamicModel,
        statics::{SStaticMesh, SStaticMeshData},
    },
    render_globals::{SRenderGlobals, SVertexInputElementSets, SVertexInputLayoutMapping},
    vertex_input::{INPUT_FORMATS, InputLayoutIndex, VertexSemantic},
};
use glam::{Vec2, Vec3, Vec4};
use tiger_pkg::TagHash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MeshKey {
    StaticDraw {
        tag: u32,
        draw: StaticDrawKey,
    },
    DynamicDraw {
        tag: u32,
        stage: RenderStage,
        mesh: u16,
        part: u16,
    },
    StaticSpecial(u32, u16),
    TerrainDraw {
        tag: u32,
        part: u16,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StaticDrawKey {
    pub part: u16,
    pub layout: u8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DynamicDrawKey {
    pub stage: RenderStage,
    pub mesh: u16,
    pub part: u16,
}
pub struct DynamicGeometry {
    pub draws: Vec<(DynamicDrawKey, Mesh)>,
}
pub struct StaticGeometry {
    pub draws: Vec<(StaticDrawKey, Mesh)>,
}
pub struct TerrainGeometry {
    pub draws: Vec<TerrainDraw>,
}
pub struct TerrainDraw {
    pub part: u16,
    pub mesh: Mesh,
    pub material: TerrainMaterial,
}
#[derive(Clone, Copy, Debug)]
pub struct TerrainMaterial {
    pub technique: u32,
    pub dyemap: Option<u32>,
    pub texcoord_transform: Vec4,
}
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub texcoords: Vec<Vec2>,
    pub colors: Vec<Vec4>,
    pub tangents: Vec<Vec4>,
    pub indices: Vec<u32>,
}

struct PositionInput {
    slot: usize,
    offset: usize,
    format: Format,
    size: usize,
}
pub struct Layouts {
    mapping: SVertexInputLayoutMapping,
    elements: SVertexInputElementSets,
}
impl Layouts {
    pub fn load(installation: &Installation) -> Result<Self> {
        let globals: SRenderGlobals = installation.named("render_globals")?;
        let data =
            installation.follow(&globals.unk8.first().context("Empty render globals")?.unk8)?;
        let inputs = installation.follow(&data.input_layouts)?;
        Ok(Self {
            mapping: installation.follow(&inputs.mapping)?,
            elements: installation.follow(&inputs.elements_c)?,
        })
    }
    fn position(&self, index: u8) -> Result<PositionInput> {
        self.channel(index, VertexSemantic::Position)
    }
    fn channel(&self, index: u8, semantic: VertexSemantic) -> Result<PositionInput> {
        self.channel_index(index, semantic, 0)
    }
    fn channel_index(
        &self,
        index: u8,
        semantic: VertexSemantic,
        semantic_index: u8,
    ) -> Result<PositionInput> {
        self.optional_channel(index, semantic, semantic_index)?
            .with_context(|| format!("Layout {index} has no semantic {} index{semantic_index}", semantic as u8))
    }
    fn optional_channel(&self, index: u8, semantic: VertexSemantic, semantic_index: u8) -> Result<Option<PositionInput>> {
        let layout = self
            .mapping
            .layouts
            .iter()
            .find(|l| l.index == index)
            .with_context(|| format!("Missing source input layout {index}"))?;
        for (slot, (set, instanced)) in [
            (layout.buffer_0, layout.buffer_0_instanced),
            (layout.buffer_1, layout.buffer_1_instanced),
            (layout.buffer_2, layout.buffer_2_instanced),
            (layout.buffer_3, layout.buffer_3_instanced),
        ]
        .into_iter()
        .enumerate()
        {
            if set == u32::MAX || instanced {
                continue;
            }
            let elements = &self
                .elements
                .sets
                .get(set as usize)
                .context("Invalid source element set")?
                .elements;
            let mut offset = 0;
            for element in elements {
                if element.semantic == semantic as u8 && element.semantic_index == semantic_index {
                    return Ok(Some(PositionInput {
                        slot,
                        offset,
                        format: INPUT_FORMATS
                            .get(element.format as usize)
                            .context("Unknown vertex format")?
                            .format,
                        size: INPUT_FORMATS[element.format as usize].stride as usize,
                    }));
                }
                offset += INPUT_FORMATS
                    .get(element.format as usize)
                    .context("Unknown source vertex format")?
                    .stride as usize;
            }
        }
        Ok(None)
    }
}

struct Buffer {
    bytes: Vec<u8>,
    stride: usize,
}
impl Buffer {
    fn vectors(&self, input: &PositionInput) -> Result<Vec<Vec4>> {
        ensure!(input.offset + input.size <= self.stride, "Vector channel exceeds stride");
        self.bytes.chunks_exact(self.stride).map(|vertex| {
            let bytes = &vertex[input.offset..input.offset + input.size];
            Ok(match input.format {
                Format::R16g16b16a16Snorm => Vec4::from_array(std::array::from_fn(|i| {
                    (i16::from_le_bytes([bytes[i*2],bytes[i*2+1]]) as f32 / i16::MAX as f32).max(-1.0)
                })),
                Format::R8g8b8a8Unorm => Vec4::from_array(std::array::from_fn(|i| bytes[i] as f32 / u8::MAX as f32)),
                format => bail!("Unsupported authored vector format {format:?}"),
            })
        }).collect()
    }
    fn tangent_frames(&self, input: &PositionInput) -> Result<(Vec<Vec3>, Vec<Vec4>)> {
        ensure!(
            input.offset + input.size <= self.stride,
            "Tangent channel exceeds stride"
        );
        let mut normals = Vec::with_capacity(self.bytes.len() / self.stride);
        let mut tangents = Vec::with_capacity(normals.capacity());
        for vertex in self.bytes.chunks_exact(self.stride) {
            let data = &vertex[input.offset..input.offset + input.size];
            let q = match input.format {
                Format::R16g16b16a16Snorm => Vec4::from_array(std::array::from_fn(|i| {
                    (i16::from_le_bytes([data[i * 2], data[i * 2 + 1]]) as f32 / i16::MAX as f32)
                        .max(-1.0)
                })),
                Format::R8g8b8a8Snorm => Vec4::from_array(std::array::from_fn(|i| {
                    (data[i] as i8 as f32 / i8::MAX as f32).max(-1.0)
                })),
                format => bail!("Unsupported tangent quaternion format {format:?}"),
            };
            // Original static VS: quaternion column0 is NORMAL, column1 TANGENT.
            // Keep the quantized components and W sign rather than normalizing Q.
            normals.push(Vec3::new(
                1.0 - 2.0 * q.z * q.z - 2.0 * q.y * q.y,
                2.0 * q.x * q.y + 2.0 * q.z * q.w,
                2.0 * q.x * q.z - 2.0 * q.y * q.w,
            ));
            tangents.push(
                Vec3::new(
                    2.0 * q.x * q.y - 2.0 * q.z * q.w,
                    1.0 - 2.0 * q.z * q.z - 2.0 * q.x * q.x,
                    2.0 * q.y * q.z + 2.0 * q.x * q.w,
                )
                .extend(if q.w >= 0.0 { 1.0 } else { -1.0 }),
            );
        }
        Ok((normals, tangents))
    }
    fn load(installation: &Installation, hash: TagHash) -> Result<Self> {
        let header: VertexBufferHeader = installation.read_type(hash.0)?;
        ensure!(header.stride > 0, "Zero vertex stride in {hash}");
        let bytes = installation.read(installation.reference(hash.0)?)?;
        ensure!(
            bytes.len() == header.data_size as usize && bytes.len() % header.stride as usize == 0,
            "Vertex payload size/stride mismatch for {hash}"
        );
        Ok(Self {
            bytes,
            stride: header.stride as usize,
        })
    }
    fn texcoords(&self, input: &PositionInput, scale: Vec2, offset: Vec2) -> Result<Vec<Vec2>> {
        ensure!(
            input.offset + input.size <= self.stride,
            "UV channel exceeds vertex stride"
        );
        self.bytes
            .chunks_exact(self.stride)
            .map(|vertex| {
                let data = &vertex[input.offset..input.offset + input.size];
                let uv = match input.format {
                    Format::R16g16Snorm => {
                        Vec2::new(
                            i16::from_le_bytes(data[0..2].try_into()?) as f32,
                            i16::from_le_bytes(data[2..4].try_into()?) as f32,
                        ) / i16::MAX as f32
                    }
                    Format::R32g32Float => Vec2::new(
                        f32::from_le_bytes(data[0..4].try_into()?),
                        f32::from_le_bytes(data[4..8].try_into()?),
                    ),
                    Format::R16g16Float => Vec2::new(
                        half::f16::from_le_bytes(data[0..2].try_into()?).to_f32(),
                        half::f16::from_le_bytes(data[2..4].try_into()?).to_f32(),
                    ),
                    format => bail!("Unsupported TEXCOORD format {format:?}"),
                };
                let uv = if input.format == Format::R16g16Snorm {
                    uv.max(Vec2::splat(-1.0))
                } else {
                    uv
                };
                let uv = uv * scale + offset;
                ensure!(uv.is_finite(), "Non-finite decoded UV");
                Ok(uv)
            })
            .collect()
    }
    fn positions(&self, input: &PositionInput, scale: Vec3, offset: Vec3) -> Result<Vec<Vec3>> {
        let size = input.size;
        ensure!(
            input.offset + size <= self.stride,
            "POSITION channel exceeds vertex stride"
        );
        self.bytes
            .chunks_exact(self.stride)
            .map(|vertex| {
                let data = &vertex[input.offset..input.offset + size];
                let p = match input.format {
                    Format::R32g32b32Float | Format::R32g32b32a32Float => Vec3::new(
                        f32::from_le_bytes(data[0..4].try_into()?),
                        f32::from_le_bytes(data[4..8].try_into()?),
                        f32::from_le_bytes(data[8..12].try_into()?),
                    ),
                    Format::R16g16b16a16Snorm => Vec3::new(
                        i16::from_le_bytes(data[0..2].try_into()?) as f32,
                        i16::from_le_bytes(data[2..4].try_into()?) as f32,
                        i16::from_le_bytes(data[4..6].try_into()?) as f32,
                    ),
                    format => bail!("Unsupported model POSITION format {format:?}"),
                };
                let p = if input.format == Format::R16g16b16a16Snorm {
                    (p / i16::MAX as f32).max(Vec3::splat(-1.0))
                } else {
                    p
                };
                let p = p * scale + offset;
                ensure!(p.is_finite(), "Non-finite decoded position");
                Ok(p)
            })
            .collect()
    }
}

struct Indices {
    values: Vec<u32>,
    restart: u32,
}
impl Indices {
    fn load(installation: &Installation, hash: TagHash) -> Result<Self> {
        let header: IndexBufferHeader = installation.read_type(hash.0)?;
        let bytes = installation.read(installation.reference(hash.0)?)?;
        let stride = if header.is_32bit { 4 } else { 2 };
        ensure!(
            bytes.len() as u64 == header.data_size && bytes.len() % stride == 0,
            "Index payload size mismatch for {hash}"
        );
        let values = bytes
            .chunks_exact(stride)
            .map(|b| {
                if stride == 4 {
                    u32::from_le_bytes(b.try_into().unwrap())
                } else {
                    u16::from_le_bytes(b.try_into().unwrap()) as u32
                }
            })
            .collect();
        Ok(Self {
            values,
            restart: if stride == 4 {
                u32::MAX
            } else {
                u16::MAX as u32
            },
        })
    }
    fn triangles(&self, start: u32, count: u32, primitive: PrimitiveType) -> Result<Vec<u32>> {
        let end = start.checked_add(count).context("Index range overflow")?;
        let source = self
            .values
            .get(start as usize..end as usize)
            .context("Draw range outside index buffer")?;
        match primitive {
            PrimitiveType::Triangles => {
                ensure!(source.len() % 3 == 0, "Incomplete triangle list");
                Ok(source.to_vec())
            }
            PrimitiveType::TriangleStrip => {
                ensure!(source.len() >= 3, "Incomplete triangle strip");
                Ok(meshopt::unstripify(source, self.restart)?)
            }
            _ => bail!("Non-triangle inspection draw: {primitive:?}"),
        }
    }
}
impl Mesh {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            texcoords: Vec::new(),
            colors: Vec::new(),
            tangents: Vec::new(),
            indices: Vec::new(),
        }
    }
    fn append(&mut self, positions: Vec<Vec3>, mut indices: Vec<u32>) -> Result<()> {
        ensure!(
            indices.iter().all(|&i| (i as usize) < positions.len()),
            "Index outside vertex buffer"
        );
        let positions = meshopt::optimize_vertex_fetch(&mut indices, &positions);
        let base = u32::try_from(self.positions.len())?;
        self.positions.extend(positions);
        self.indices.extend(indices.into_iter().map(|i| i + base));
        Ok(())
    }
    fn finish(mut self) -> Result<Self> {
        if !self.normals.is_empty() {
            return Ok(self);
        }
        self.normals.resize(self.positions.len(), Vec3::ZERO);
        for triangle in self.indices.chunks_exact(3) {
            let [a, b, c] = [
                triangle[0] as usize,
                triangle[1] as usize,
                triangle[2] as usize,
            ];
            let normal = (self.positions[b] - self.positions[a])
                .cross(self.positions[c] - self.positions[a]);
            self.normals[a] += normal;
            self.normals[b] += normal;
            self.normals[c] += normal;
        }
        for normal in &mut self.normals {
            *normal = normal.normalize_or_zero();
        }
        Ok(self)
    }
}

/// Decode each original static stream once, then compact each color draw without
/// copying complete vertex streams. The engine owns the resulting resource lifetime.
pub(crate) fn load_static(
    installation: &Installation,
    tag: u32,
    layouts: &Layouts,
) -> Result<StaticGeometry> {
    let data: SStaticMeshData = installation.read_type(tag)?;
    let mut groups = std::collections::BTreeMap::<(u8, u8), std::collections::BTreeSet<u16>>::new();
    for group in &data.mesh_groups {
        let part = data
            .parts
            .get(group.part_index as usize)
            .context("Static group part")?;
        if group.render_stage == RenderStage::GenerateGbuffer
            && part.lod_category.is_highest_detail()
        {
            groups
                .entry((part.buffer_index, group.input_layout_index))
                .or_default()
                .insert(group.part_index);
        }
    }
    let mut draws = Vec::new();
    for ((buffer_index, layout), parts) in groups {
        let &(index, v0, v1, color) = data
            .buffers
            .get(buffer_index as usize)
            .context("Static buffer index")?;
        let buffers = [
            Buffer::load(installation, v0)?,
            Buffer::load(installation, v1)?,
        ];
        let position = layouts.position(layout)?;
        let uv = layouts.channel(layout, VertexSemantic::TexCoord)?;
        let tangent = layouts.channel(layout, VertexSemantic::Tangent)?;
        let positions = buffers
            .get(position.slot)
            .context("Static position stream")?
            .positions(&position, Vec3::splat(data.mesh_scale), data.mesh_offset)?;
        let texcoords = buffers
            .get(uv.slot)
            .context("Static UV stream")?
            .texcoords(
                &uv,
                Vec2::splat(data.texture_coordinate_scale),
                data.texture_coordinate_offset,
            )?;
        let (normals, tangents) = buffers
            .get(tangent.slot)
            .context("Static tangent stream")?
            .tangent_frames(&tangent)?;
        ensure!(
            positions.len() == texcoords.len() && positions.len() == normals.len(),
            "Static vertex channel count mismatch"
        );
        let colors = if color.is_some() {
            let buffer = Buffer::load(installation, color)?;
            ensure!(
                buffer.stride == 4 && !buffer.bytes.is_empty(),
                "Static color buffer must contain RGBA8 vertices"
            );
            Some(
                buffer
                    .bytes
                    .chunks_exact(4)
                    .map(|v| {
                        Vec4::new(v[0] as f32, v[1] as f32, v[2] as f32, v[3] as f32)
                            / u8::MAX as f32
                    })
                    .collect::<Vec<_>>(),
            )
        } else {
            None
        };
        let index = Indices::load(installation, index)?;
        for part_index in parts {
            let part = &data.parts[part_index as usize];
            let indices =
                index.triangles(part.index_start, part.index_count, part.primitive_type)?;
            let mesh = compact_draw(&positions, &normals, &texcoords, &tangents,
                colors.as_deref(), data.max_color_index as usize, indices)?;
            draws.push((
                StaticDrawKey {
                    part: part_index,
                    layout,
                },
                mesh,
            ));
        }
    }
    Ok(StaticGeometry { draws })
}

/// Decode each dynamic color stream once. Child draw meshes are compact views
/// owned together by the engine's cached model resource.
pub(crate) fn load_dynamic(installation: &Installation, tag: u32, layouts: &Layouts) -> Result<DynamicGeometry> {
    let model: SDynamicModel = installation.read_type(tag)?;
    let mut draws = Vec::new();
    for (mesh_index, source) in model.meshes.iter().enumerate() {
        let mut streams = std::collections::BTreeMap::new();
        let index = Indices::load(installation, source.index_buffer)?;
        for stage in [RenderStage::GenerateGbuffer, RenderStage::Transparents] {
            let range = source.get_range_for_stage(stage);
            let parts = source.parts.get(range.clone()).context("Dynamic color part range")?;
            if !parts.iter().any(|part| part.lod_category.is_highest_detail()) { continue; }
            let layout = source.get_input_layout_for_stage(stage);
            let hashes = [source.vertex0_buffer, source.vertex1_buffer, source.buffer2, source.buffer3];
            for (slot, hash) in hashes.into_iter().enumerate().filter(|(_, hash)| hash.is_some()) {
                if !streams.contains_key(&slot) { streams.insert(slot, Buffer::load(installation, hash)?); }
            }
            let position = layouts.position(layout)?;
            let uv = layouts.channel(layout, VertexSemantic::TexCoord)?;
            let positions = streams.get(&position.slot).context("Dynamic position stream")?
                .positions(&position, model.model_scale.truncate(), model.model_offset.truncate())?;
            let texcoords = streams.get(&uv.slot).context("Dynamic UV stream")?
                .texcoords(&uv, model.texcoord_scale, model.texcoord_offset)?;
            let (normals, tangents) = match (
                layouts.optional_channel(layout, VertexSemantic::Normal, 0)?,
                layouts.optional_channel(layout, VertexSemantic::Tangent, 0)?) {
                (None, Some(frame)) => streams.get(&frame.slot).context("Dynamic quaternion stream")?.tangent_frames(&frame)?,
                (Some(normal), None) => (streams.get(&normal.slot).context("Dynamic normal stream")?
                    .positions(&normal, Vec3::ONE, Vec3::ZERO)?, Vec::new()),
                (Some(normal), Some(tangent)) => (
                    streams.get(&normal.slot).context("Dynamic normal stream")?.positions(&normal, Vec3::ONE, Vec3::ZERO)?,
                    streams.get(&tangent.slot).context("Dynamic tangent stream")?.vectors(&tangent)?,
                ),
                _ => bail!("Unimplemented dynamic frame layout {layout}"),
            };
            ensure!(positions.len() == texcoords.len() && positions.len() == normals.len(), "Dynamic vertex channel count mismatch");
            let inline_color = layouts.optional_channel(layout, VertexSemantic::Color, 0)?;
            let colors = if let Some(color) = inline_color {
                ensure!(!source.color_buffer.is_some(), "Dynamic layout has two color stream owners");
                Some(streams.get(&color.slot).context("Dynamic inline color stream")?.vectors(&color)?)
            } else if source.color_buffer.is_some() {
                let buffer = Buffer::load(installation, source.color_buffer)?;
                ensure!(buffer.stride == 4 && !buffer.bytes.is_empty(), "Dynamic color buffer must contain RGBA8 vertices");
                Some(buffer.bytes.chunks_exact(4).map(|v| Vec4::new(v[0] as f32,v[1] as f32,v[2] as f32,v[3] as f32) / u8::MAX as f32).collect::<Vec<_>>())
            } else { None };
            for (offset, part) in parts.iter().enumerate().filter(|(_,p)| p.lod_category.is_highest_detail()) {
                let indices = index.triangles(part.index_start,part.index_count,part.primitive_type)?;
                let mesh = compact_draw(&positions,&normals,&texcoords,&tangents,colors.as_deref(),colors.as_ref().map_or(0, |v| v.len()-1),indices)?;
                draws.push((DynamicDrawKey {stage,mesh:u16::try_from(mesh_index)?,part:u16::try_from(range.start+offset)?},mesh));
            }
        }
    }
    Ok(DynamicGeometry { draws })
}

fn compact_draw(positions: &[Vec3], normals: &[Vec3], texcoords: &[Vec2], tangents: &[Vec4], colors: Option<&[Vec4]>, max_color: usize, indices: Vec<u32>) -> Result<Mesh> {
    ensure!(indices.iter().all(|&index| (index as usize) < positions.len()), "Draw index outside source buffer");
    let mut remap = vec![u32::MAX; positions.len()];
    // SAFETY: range-checked indices; one remap destination per original vertex.
    let count = unsafe { meshopt::ffi::meshopt_optimizeVertexFetchRemap(remap.as_mut_ptr(),indices.as_ptr(),indices.len(),positions.len()) };
    let mut mesh = Mesh::new();
    mesh.positions.resize(count,Vec3::ZERO);
    mesh.normals.resize(count,Vec3::ZERO);
    mesh.texcoords.resize(count,Vec2::ZERO);
    if !tangents.is_empty() { mesh.tangents.resize(count,Vec4::ZERO); }
    if colors.is_some() { mesh.colors.resize(count,Vec4::ZERO); }
    for (source,&target) in remap.iter().enumerate().filter(|(_,target)| **target != u32::MAX) {
        let target = target as usize;
        mesh.positions[target] = positions[source];
        mesh.normals[target] = normals[source];
        mesh.texcoords[target] = texcoords[source];
        if !tangents.is_empty() { mesh.tangents[target] = tangents[source]; }
        if let Some(colors) = colors {
            let index = source.min(max_color);
            mesh.colors[target] = *colors.get(index).context("Draw color index outside source buffer")?;
        }
    }
    mesh.indices = indices.into_iter().map(|i| remap[i as usize]).collect();
    Ok(mesh)
}

pub(crate) fn load(installation: &Installation, key: MeshKey, layouts: &Layouts) -> Result<Mesh> {
    let mut mesh = Mesh::new();
    match key {
        MeshKey::StaticDraw { tag, draw } => {
            return load_static(installation, tag, layouts)?
                .draws
                .into_iter()
                .find(|(key, _)| *key == draw)
                .map(|(_, mesh)| mesh)
                .context("Static draw not found");
        }
        MeshKey::StaticSpecial(tag, part) => {
            let model: SStaticMesh = installation.read_type(tag)?;
            let data = installation.follow(&model.opaque_meshes)?;
            let special = model
                .special_meshes
                .get(part as usize)
                .context("Static special part")?;
            let input = layouts.position(special.input_layout_index)?;
            let hash = *[
                special.vertex0_buffer,
                special.vertex1_buffer,
                special.color_buffer,
            ]
            .get(input.slot)
            .context("Special position stream")?;
            let positions = Buffer::load(installation, hash)?.positions(
                &input,
                Vec3::splat(data.mesh_scale),
                data.mesh_offset,
            )?;
            let indices = Indices::load(installation, special.index_buffer)?.triangles(
                special.index_start,
                special.index_count,
                special.primitive_type,
            )?;
            mesh.append(positions, indices)?;
        }
        MeshKey::DynamicDraw { tag, stage, mesh: mesh_index, part } => {
            let key = DynamicDrawKey { stage, mesh: mesh_index, part };
            return load_dynamic(installation, tag, layouts)?.draws.into_iter()
                .find_map(|(draw, mesh)| (draw == key).then_some(mesh)).context("Dynamic color draw missing");
        }
        MeshKey::TerrainDraw { tag, part } => {
            return load_terrain(installation, tag, layouts)?
                .draws
                .into_iter()
                .find(|draw| draw.part == part)
                .map(|draw| draw.mesh)
                .context("Missing terrain color draw");
        }
    }
    mesh.finish()
}

pub(crate) fn load_terrain(
    installation: &Installation,
    tag: u32,
    layouts: &Layouts,
) -> Result<TerrainGeometry> {
    use deimos_data::tfx::features::terrain::{STerrain, TerrainDetailLevel};
    let terrain: STerrain = installation.read_type(tag)?;
    let input = layouts.position(InputLayoutIndex::TERRAIN.0)?;
    ensure!(
        input.slot == 0 && input.offset == 0 && input.format == Format::R16g16b16a16Sint,
        "Terrain POSITION contract changed"
    );
    let buffer = Buffer::load(installation, terrain.vertex0_buffer)?;
    ensure!(
        buffer.stride == input.size,
        "Terrain position stride changed"
    );
    // Original terrain vertex reconstruction documented in INTEGRATION.md.
    // All four signed lanes participate; positions are already world-space.
    const HORIZONTAL_UNITS_PER_METRE: f32 = 64.0;
    const VERTICAL_UNITS_PER_METRE: f32 = 8192.0;
    const HIGH_LANE_WEIGHT: f32 = (u16::MAX as u32 + 1) as f32;
    let offset = terrain.unk30;
    let positions = buffer
        .bytes
        .chunks_exact(input.size)
        .map(|b| {
            let lanes = std::array::from_fn::<_, 4, _>(|i| {
                i16::from_le_bytes([b[i * 2], b[i * 2 + 1]]) as f32
            });
            Vec3::new(
                (lanes[0] + offset.x) / HORIZONTAL_UNITS_PER_METRE,
                (lanes[1] + offset.y) / HORIZONTAL_UNITS_PER_METRE,
                ((lanes[2] + offset.z) + (lanes[3] + offset.w) * HIGH_LANE_WEIGHT)
                    / VERTICAL_UNITS_PER_METRE,
            )
        })
        .collect::<Vec<_>>();
    let index = Indices::load(installation, terrain.index_buffer)?;
    let normal_input = layouts.channel(InputLayoutIndex::TERRAIN.0, VertexSemantic::Normal)?;
    let uv_input =
        layouts.channel_index(InputLayoutIndex::TERRAIN.0, VertexSemantic::TexCoord, 1)?;
    let attributes = Buffer::load(installation, terrain.vertex1_buffer)?;
    ensure!(
        normal_input.slot == 1
            && normal_input.format == Format::R16g16b16a16Snorm
            && normal_input.offset + normal_input.size <= attributes.stride
            && uv_input.slot == 1,
        "Terrain normal/UV stream contract changed"
    );
    let texcoords = attributes.texcoords(&uv_input, Vec2::ONE, Vec2::ZERO)?;
    ensure!(
        positions.len() == texcoords.len(),
        "Terrain vertex channel count mismatch"
    );
    #[derive(Clone, Copy, Default)]
    struct Vertex {
        position: Vec3,
        normal: Vec3,
        tangent: Vec4,
        uv: Vec2,
    }
    let source_vertices: Vec<_> = positions
        .into_iter()
        .zip(texcoords)
        .zip(attributes.bytes.chunks_exact(attributes.stride))
        .map(|((position, uv), data)| {
            let data = &data[normal_input.offset..normal_input.offset + normal_input.size];
            let normal = Vec3::from_array(std::array::from_fn(|i| {
                (i16::from_le_bytes([data[i * 2], data[i * 2 + 1]]) as f32 / i16::MAX as f32)
                    .max(-1.0)
            }));
            // Retail terrain VS derives this frame once per source vertex.
            // Its NORMAL.xyz is direct SNORM; W is not a quaternion component.
            let bitangent = Vec3::new(0.0, -normal.z, normal.y).normalize();
            let tangent = normal.cross(bitangent).normalize().extend(-1.0);
            Vertex {
                position,
                normal,
                tangent,
                uv,
            }
        })
        .collect();
    let mut draws = Vec::new();
    for (part_index, part) in terrain
        .mesh_parts
        .iter()
        .enumerate()
        .filter(|(_, p)| p.detail_level == TerrainDetailLevel::High)
    {
        let mut mesh = Mesh::new();
        let mut triangles = index.triangles(
            part.index_start,
            part.index_count as u32,
            PrimitiveType::TriangleStrip,
        )?;
        ensure!(
            triangles
                .iter()
                .all(|&i| (i as usize) < source_vertices.len()),
            "Terrain index outside vertex buffer"
        );
        let vertices = meshopt::optimize_vertex_fetch(&mut triangles, &source_vertices);
        mesh.positions.reserve(vertices.len());
        mesh.normals.reserve(vertices.len());
        mesh.texcoords.reserve(vertices.len());
        mesh.tangents.reserve(vertices.len());
        for vertex in vertices {
            ensure!(
                vertex.normal.is_finite() && vertex.tangent.is_finite() && vertex.uv.is_finite(),
                "Invalid referenced terrain vertex frame/UV"
            );
            mesh.positions.push(vertex.position);
            mesh.normals.push(vertex.normal);
            mesh.texcoords.push(vertex.uv);
            mesh.tangents.push(vertex.tangent);
        }
        mesh.indices = triangles;
        let group = terrain
            .mesh_groups
            .get(part.group_index as usize)
            .context("Terrain part group index")?;
        ensure!(
            part.technique.is_some(),
            "Terrain color draw has no technique"
        );
        draws.push(TerrainDraw {
            part: u16::try_from(part_index)?,
            mesh,
            material: TerrainMaterial {
                technique: part.technique.0,
                dyemap: group.dyemap.is_some().then_some(group.dyemap.0),
                texcoord_transform: group.unk20,
            },
        });
    }
    Ok(TerrainGeometry { draws })
}
