//! Engine-independent geometry. Source draw ranges, quantization and vertex
//! channels come from Deimos; the embedding engine owns material resources.
use crate::Installation;
use anyhow::{Context, Result, bail, ensure};
use deimos_data::dxgi::Format;
use deimos_data::tfx::{
    PrimitiveType, RenderStage,
    buffers::{IndexBufferHeader, VertexBufferHeader},
    features::{
        decorators::DecoratorQuality,
        dynamic::SDynamicModel,
        statics::{SStaticMesh, SStaticMeshData},
    },
    render_globals::{SRenderGlobals, SVertexInputElementSets, SVertexInputLayoutMapping},
    vertex_input::{INPUT_FORMATS, InputLayoutIndex, VertexSemantic},
};
use glam::{Vec2, Vec3, Vec4};
use std::collections::HashSet;
use tiger_pkg::TagHash;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MeshKey {
    Static(u32),
    Dynamic(u32, RenderStage),
    DynamicDraw {
        tag: u32,
        stage: RenderStage,
        mesh: u16,
        part: u16,
    },
    StaticSpecial(u32, u16),
    Terrain(u32),
    Decorator(u32, u16),
}
pub struct Mesh {
    pub positions: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub texcoords: Vec<Vec2>,
    pub colors: Vec<Vec4>,
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
                if element.semantic == semantic as u8 && element.semantic_index == 0 {
                    return Ok(PositionInput {
                        slot,
                        offset,
                        format: INPUT_FORMATS
                            .get(element.format as usize)
                            .context("Unknown vertex format")?
                            .format,
                        size: INPUT_FORMATS[element.format as usize].stride as usize,
                    });
                }
                offset += INPUT_FORMATS
                    .get(element.format as usize)
                    .context("Unknown source vertex format")?
                    .stride as usize;
            }
        }
        bail!(
            "Layout {index} has no per-vertex semantic {} index0",
            semantic as u8
        )
    }
}

struct Buffer {
    bytes: Vec<u8>,
    stride: usize,
}
impl Buffer {
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
                    format => bail!("Unsupported TEXCOORD0 format {format:?}"),
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
    fn append_textured(
        &mut self,
        positions: Vec<Vec3>,
        texcoords: Vec<Vec2>,
        colors: Option<Vec<Vec4>>,
        mut indices: Vec<u32>,
    ) -> Result<()> {
        #[derive(Clone, Copy, Default)]
        struct Vertex {
            position: Vec3,
            texcoord: Vec2,
            color: Option<Vec4>,
        }
        ensure!(
            positions.len() == texcoords.len(),
            "Position/UV stream vertex count mismatch"
        );
        ensure!(
            indices.iter().all(|&i| (i as usize) < positions.len()),
            "Index outside textured vertex buffer"
        );
        let vertices: Vec<_> = positions
            .into_iter()
            .zip(texcoords)
            .enumerate()
            .map(|(index, (position, texcoord))| Vertex {
                position,
                texcoord,
                color: colors
                    .as_ref()
                    .map(|values| values[index.min(values.len() - 1)]),
            })
            .collect();
        let vertices = meshopt::optimize_vertex_fetch(&mut indices, &vertices);
        let base = u32::try_from(self.positions.len())?;
        for vertex in vertices {
            self.positions.push(vertex.position);
            self.texcoords.push(vertex.texcoord);
            if let Some(color) = vertex.color {
                self.colors.push(color);
            }
        }
        self.indices.extend(indices.into_iter().map(|i| i + base));
        Ok(())
    }
    fn finish(mut self) -> Result<Self> {
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

pub(crate) fn load(installation: &Installation, key: MeshKey, layouts: &Layouts) -> Result<Mesh> {
    let mut mesh = Mesh::new();
    match key {
        MeshKey::Static(tag) => {
            let data: SStaticMeshData = installation.read_type(tag)?;
            let mut drawn = HashSet::new();
            let mut draws = std::collections::BTreeMap::<(u8, u8), Vec<usize>>::new();
            for group in &data.mesh_groups {
                if group.render_stage != RenderStage::GenerateGbuffer
                    || !drawn.insert(group.part_index)
                {
                    continue;
                }
                let part = data
                    .parts
                    .get(group.part_index as usize)
                    .context("Static part index")?;
                if part.lod_category.is_highest_detail() {
                    draws
                        .entry((part.buffer_index, group.input_layout_index))
                        .or_default()
                        .push(group.part_index as usize);
                }
            }
            for ((buffer, layout), parts) in draws {
                let &(index, v0, v1, color) = data
                    .buffers
                    .get(buffer as usize)
                    .context("Static buffer index")?;
                let input = layouts.position(layout)?;
                let hash = *[v0, v1, color]
                    .get(input.slot)
                    .context("Static position stream")?;
                let positions = Buffer::load(installation, hash)?.positions(
                    &input,
                    Vec3::splat(data.mesh_scale),
                    data.mesh_offset,
                )?;
                let index = Indices::load(installation, index)?;
                let mut triangles = Vec::new();
                for part in parts {
                    let part = &data.parts[part];
                    triangles.extend(index.triangles(
                        part.index_start,
                        part.index_count,
                        part.primitive_type,
                    )?);
                }
                mesh.append(positions, triangles)?;
            }
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
        MeshKey::DynamicDraw {
            tag,
            stage,
            mesh: mesh_index,
            part: part_index,
        } => {
            let model: SDynamicModel = installation.read_type(tag)?;
            let source = model
                .meshes
                .get(mesh_index as usize)
                .context("Dynamic draw mesh index")?;
            ensure!(
                source
                    .get_range_for_stage(stage)
                    .contains(&(part_index as usize)),
                "Dynamic draw part outside stage"
            );
            let part = source
                .parts
                .get(part_index as usize)
                .context("Dynamic draw part index")?;
            ensure!(
                part.lod_category.is_highest_detail(),
                "Dynamic draw is not highest detail"
            );
            let layout = source.get_input_layout_for_stage(stage);
            let position = layouts.position(layout)?;
            let uv = layouts.channel(layout, VertexSemantic::TexCoord)?;
            let buffers = [
                source.vertex0_buffer,
                source.vertex1_buffer,
                source.buffer2,
                source.buffer3,
            ];
            let positions = Buffer::load(installation, buffers[position.slot])?.positions(
                &position,
                model.model_scale.truncate(),
                model.model_offset.truncate(),
            )?;
            let texcoords = Buffer::load(installation, buffers[uv.slot])?.texcoords(
                &uv,
                model.texcoord_scale,
                model.texcoord_offset,
            )?;
            let indices = Indices::load(installation, source.index_buffer)?.triangles(
                part.index_start,
                part.index_count,
                part.primitive_type,
            )?;
            let colors = if source.color_buffer.is_some() {
                let buffer = Buffer::load(installation, source.color_buffer)?;
                ensure!(
                    buffer.stride == 4 && !buffer.bytes.is_empty(),
                    "Source color buffer must contain RGBA8 vertices"
                );
                Some(
                    buffer
                        .bytes
                        .chunks_exact(4)
                        .map(|v| {
                            Vec4::new(v[0] as f32, v[1] as f32, v[2] as f32, v[3] as f32)
                                / u8::MAX as f32
                        })
                        .collect(),
                )
            } else {
                None
            };
            mesh.append_textured(positions, texcoords, colors, indices)?;
        }
        MeshKey::Dynamic(tag, _) | MeshKey::Decorator(tag, _) => {
            let model: SDynamicModel = installation.read_type(tag)?;
            for source in &model.meshes {
                let stages: &[RenderStage] = match key {
                    MeshKey::Dynamic(_, ref stage) => std::slice::from_ref(stage),
                    MeshKey::Decorator(..) => &[RenderStage::GenerateGbuffer],
                    _ => unreachable!(),
                };
                for &stage in stages {
                    let parts = source
                        .parts
                        .get(source.get_range_for_stage(stage))
                        .context("Rigid part range")?;
                    let selected = |p: &&deimos_data::tfx::features::dynamic::SDynamicMeshPart| {
                        p.lod_category.is_highest_detail()
                            && match key {
                                MeshKey::Decorator(_, identifier) => {
                                    p.external_identifier == identifier
                                        && p.unk17 == DecoratorQuality::High as u8
                                }
                                _ => true,
                            }
                    };
                    if parts.iter().filter(selected).next().is_none() {
                        continue;
                    }
                    let input = layouts.position(source.get_input_layout_for_stage(stage))?;
                    let hash = *[
                        source.vertex0_buffer,
                        source.vertex1_buffer,
                        source.buffer2,
                        source.buffer3,
                    ]
                    .get(input.slot)
                    .context("Rigid position stream")?;
                    let positions = Buffer::load(installation, hash)?.positions(
                        &input,
                        model.model_scale.truncate(),
                        model.model_offset.truncate(),
                    )?;
                    let index = Indices::load(installation, source.index_buffer)?;
                    let mut triangles = Vec::new();
                    for part in parts.iter().filter(selected) {
                        triangles.extend(index.triangles(
                            part.index_start,
                            part.index_count,
                            part.primitive_type,
                        )?);
                    }
                    mesh.append(positions, triangles)?;
                }
            }
        }
        MeshKey::Terrain(tag) => {
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
            let mut triangles = Vec::new();
            for part in terrain
                .mesh_parts
                .iter()
                .filter(|p| p.detail_level == TerrainDetailLevel::High)
            {
                triangles.extend(index.triangles(
                    part.index_start,
                    part.index_count as u32,
                    PrimitiveType::TriangleStrip,
                )?);
            }
            mesh.append(positions, triangles)?;
        }
    }
    mesh.finish()
}
