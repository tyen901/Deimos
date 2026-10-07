use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, ensure};
use serde::Serialize;
use tiger_pkg::{PackageManager, TagHash};

#[derive(Serialize)]
struct Component {
    attachments: Vec<Attachment>,
    tag: String,
    instance_class: Option<String>,
    definition_class: Option<String>,
    definition_offset: Option<usize>,
    model: Option<String>,
    skeleton_bones: Option<usize>,
}

#[derive(Serialize)]
struct Attachment {
    name_hash: String,
    bone_index: i32,
    transform: Transform,
}

#[derive(Serialize)]
struct Entity {
    tag: String,
    package: String,
    name: Option<String>,
    components: Vec<Component>,
}

fn localized_name(
    manager: &PackageManager,
    data: &[u8],
    base: usize,
) -> anyhow::Result<Option<String>> {
    let hash = word(data, base + 0x340)?;
    if hash == u32::MAX || hash == 0 {
        return Ok(None);
    }
    let field = base + 0x350;
    let tag = if word(data, field + 4)? != 0 {
        TagHash(word(data, field)?)
    } else {
        let wide_hash = wide(data, field + 8)? as u64;
        if wide_hash == u64::MAX || wide_hash == 0 {
            return Ok(None);
        }
        manager
            .lookup
            .tag64_entries
            .get(&wide_hash)
            .context("name container wide tag unresolved")?
            .hash32
    };
    if tag.is_none() {
        return Ok(None);
    }
    let header = manager.read_tag(tag)?;
    ensure!(
        manager
            .get_entry(tag)
            .context("string container missing")?
            .reference
            == 0x8080B9B8,
        "string container class mismatch"
    );
    let count = usize::try_from(wide(&header, 8)?)?;
    let hashes_base = relative(&header, 16)?.context("missing string hash array")?;
    ensure!(
        wide(&header, hashes_base)? as usize == count,
        "string hash count mismatch"
    );
    let index = (0..count)
        .find(|i| word(&header, hashes_base + 16 + i * 4).ok() == Some(hash))
        .context("entity name hash absent from string container")?;
    let english = manager.read_tag(word(&header, 0x18)?)?;
    let combinations = array(&english, 0x38, 16, 0x8080B9BE)?;
    let combination = *combinations
        .get(index)
        .context("string combination missing")?;
    let parts_base = relative(&english, combination)?.context("string parts pointer missing")?;
    let parts_count = usize::try_from(wide(&english, combination + 8)?)?;
    let mut result = String::new();
    for part in 0..parts_count {
        let part = parts_base + part * 32;
        let text_base = relative(&english, part + 8)?.context("string text pointer missing")?;
        let byte_count = half(&english, part + 20)? as usize;
        let text = english
            .get(text_base..text_base + byte_count)
            .context("string outside payload")?;
        result.push_str(std::str::from_utf8(text).context("non-UTF8 source name")?);
    }
    Ok(Some(result))
}

#[derive(Serialize)]
struct Transform {
    rotation: [f32; 4],
    translation_scale: [f32; 4],
}

#[derive(Serialize)]
struct Bone {
    name_hash: String,
    parent: i32,
    first_child: i32,
    next_sibling: i32,
    object_transform: Option<Transform>,
    inverse_object_transform: Transform,
}

#[derive(Serialize)]
struct Skeleton {
    component: String,
    definition_class: String,
    bones: Vec<Bone>,
}

#[derive(Serialize)]
struct MeshPart {
    technique: String,
    variant_shader_index: u16,
    primitive_type: u8,
    index_start: u32,
    index_count: u32,
    external_identifier: u16,
    flags: u32,
    lod_category: u8,
}

#[derive(Serialize)]
struct Mesh {
    buffers: BTreeMap<String, Option<String>>,
    parts: Vec<MeshPart>,
    render_stage_part_ranges: Vec<u16>,
    render_stage_input_layouts: Vec<u8>,
}

#[derive(Serialize)]
struct Model {
    tag: String,
    scale: [f32; 4],
    offset: [f32; 4],
    meshes: Vec<Mesh>,
}

#[derive(Serialize)]
struct Vertex {
    influences: Option<Influences>,
    position: [f32; 3],
    position_w: i16,
    normal: [f32; 4],
    tangent: [f32; 4],
    texcoord: [f32; 2],
}

#[derive(Serialize)]
struct GeometryMesh {
    vertex_buffer: String,
    texcoord_buffer: String,
    index_buffer: String,
    skinning_buffer: Option<String>,
    vertices: Vec<Vertex>,
    indices: Vec<u32>,
    parts: Vec<MeshPart>,
    render_stage_part_ranges: Vec<u16>,
    render_stage_input_layouts: Vec<u8>,
}

#[derive(Serialize)]
struct Geometry {
    model: String,
    coordinate_space: &'static str,
    meshes: Vec<GeometryMesh>,
}

#[derive(Serialize)]
struct Influences {
    bones: [u16; 4],
    weights_unorm8: [u8; 4],
}

fn influences(data: &[u8], vertex: usize, w: i16, bone_count: usize) -> anyhow::Result<Influences> {
    let mut result = Influences {
        bones: [0; 4],
        weights_unorm8: [0; 4],
    };
    if (0..0x800).contains(&w) {
        result.bones[0] = w as u16;
        result.weights_unorm8[0] = 255;
    } else {
        let four = w < 0;
        let chunk = usize::try_from((w as i32).abs() - 0x800)?;
        let offset = chunk * 32 + (vertex % if four { 4 } else { 8 }) * if four { 8 } else { 4 };
        let record = data
            .get(offset..offset + if four { 8 } else { 4 })
            .context("skin weights outside payload")?;
        result.bones[0] = record[0] as u16;
        result.bones[1] = record[1] as u16;
        result.weights_unorm8[0] = record[2];
        result.weights_unorm8[1] = record[3];
        if four {
            result.bones[2] = record[4] as u16;
            result.bones[3] = record[5] as u16;
            result.weights_unorm8[2] = record[6];
            if record[4] != record[5] {
                result.weights_unorm8[3] = record[7];
            }
        }
    }
    ensure!(
        result.weights_unorm8.iter().map(|w| *w as u16).sum::<u16>() == 255,
        "skin weights do not sum to 255 at vertex {vertex}"
    );
    ensure!(
        result
            .bones
            .iter()
            .zip(&result.weights_unorm8)
            .all(|(b, w)| *w == 0 || (*b as usize) < bone_count),
        "skin bone outside skeleton at vertex {vertex}"
    );
    Ok(result)
}

fn buffer(manager: &PackageManager, hash: &str, subtype: u8) -> anyhow::Result<(Vec<u8>, Vec<u8>)> {
    let tag: TagHash = hash.parse()?;
    let entry = manager.get_entry(tag).context("buffer header missing")?;
    ensure!(
        entry.file_type == 32 && entry.file_subtype == subtype,
        "unexpected buffer kind {tag}"
    );
    let header = manager.read_tag(tag)?;
    let data = manager.read_tag(entry.reference)?;
    Ok((header, data))
}

pub fn geometry(
    manager: &PackageManager,
    tags: &[TagHash],
    bone_count: Option<usize>,
    output: &Path,
) -> anyhow::Result<()> {
    fs::create_dir_all(output)?;
    for tag in tags {
        let geometry = decode_geometry(manager, *tag, bone_count)?;
        fs::write(output.join(format!("{tag}.json")), serde_json::to_vec(&geometry)?)?;
    }
    Ok(())
}

fn decode_geometry(manager: &PackageManager, tag: TagHash, bone_count: Option<usize>) -> anyhow::Result<Geometry> {
        let source = model(manager, tag)?;
        let model_bytes = manager.read_tag(tag)?;
        let uv_scale = [
            f32::from_bits(word(&model_bytes, 0xC0)?),
            f32::from_bits(word(&model_bytes, 0xC4)?),
        ];
        let uv_offset = [
            f32::from_bits(word(&model_bytes, 0xC8)?),
            f32::from_bits(word(&model_bytes, 0xCC)?),
        ];
        let mut meshes = Vec::new();
        for mesh in source.meshes {
            let vb = mesh.buffers["vertex0"]
                .as_ref()
                .context("missing vertex0 buffer")?;
            let uvb = mesh.buffers["vertex1"]
                .as_ref()
                .context("missing texcoord buffer")?;
            let ib = mesh.buffers["index"]
                .as_ref()
                .context("missing index buffer")?;
            let (vh, vd) = buffer(manager, vb, 4)?;
            let (uh, ud) = buffer(manager, uvb, 4)?;
            let (ih, id) = buffer(manager, ib, 6)?;
            let skin_data = bone_count
                .map(|_| -> anyhow::Result<Vec<u8>> {
                    let skin = mesh.buffers["skinning"]
                        .as_ref()
                        .context("weighted export requires skinning buffer")?;
                    let (header, data) = buffer(manager, skin, 4)?;
                    ensure!(
                        half(&header, 4)? == 4 && half(&header, 6)? == 6,
                        "unsupported skinning buffer layout"
                    );
                    ensure!(
                        word(&header, 0)? as usize == data.len(),
                        "skin buffer size mismatch"
                    );
                    Ok(data)
                })
                .transpose()?;
            let stride = half(&vh, 4)? as usize;
            let uv_stride = half(&uh, 4)? as usize;
            ensure!(
                stride == 24,
                "unsupported position stride {stride} for {tag}/{vb}"
            );
            ensure!(
                uv_stride == 4,
                "unsupported UV stride {uv_stride} for {tag}/{uvb}"
            );
            ensure!(
                vd.len() == word(&vh, 0)? as usize && ud.len() == word(&uh, 0)? as usize,
                "vertex data size mismatch"
            );
            ensure!(
                vd.len() % stride == 0 && ud.len() / uv_stride == vd.len() / stride,
                "vertex/UV count mismatch for {tag}"
            );
            let mut vertices = Vec::new();
            for (index, vertex) in vd.chunks_exact(stride).enumerate() {
                let mut position = [0.0; 3];
                let mut normal = [0.0; 4];
                let mut tangent = [0.0; 4];
                for axis in 0..3 {
                    position[axis] = half(vertex, axis * 2)? as i16 as f32 / 32767.0
                        * source.scale[axis]
                        + source.offset[axis];
                }
                for axis in 0..4 {
                    normal[axis] = half(vertex, 8 + axis * 2)? as i16 as f32 / 32767.0;
                    tangent[axis] = half(vertex, 16 + axis * 2)? as i16 as f32 / 32767.0;
                }
                let mut texcoord = [0.0; 2];
                for axis in 0..2 {
                    texcoord[axis] = half(&ud, index * uv_stride + axis * 2)? as i16 as f32
                        / 32767.0
                        * uv_scale[axis]
                        + uv_offset[axis];
                }
                vertices.push(Vertex {
                    influences: skin_data
                        .as_ref()
                        .map(|data| {
                            influences(
                                data,
                                index,
                                half(vertex, 6)? as i16,
                                bone_count.context("missing skeleton count")?,
                            )
                        })
                        .transpose()?,
                    position,
                    position_w: half(vertex, 6)? as i16,
                    normal,
                    tangent,
                    texcoord,
                });
            }
            ensure!(
                wide(&ih, 8)? as usize == id.len(),
                "index buffer size mismatch for {tag}"
            );
            let index_stride = match ih[1] {
                0 => 2,
                1 => 4,
                _ => anyhow::bail!("invalid index width flag"),
            };
            ensure!(id.len() % index_stride == 0, "incomplete index data");
            let indices = (0..id.len() / index_stride)
                .map(|i| {
                    if index_stride == 2 {
                        Ok(half(&id, i * 2)? as u32)
                    } else {
                        word(&id, i * 4)
                    }
                })
                .collect::<anyhow::Result<Vec<_>>>()?;
            for part in &mesh.parts {
                let end = part
                    .index_start
                    .checked_add(part.index_count)
                    .context("part range overflow")?;
                ensure!(
                    end as usize <= indices.len(),
                    "part outside index buffer for {tag}"
                );
            }
            meshes.push(GeometryMesh {
                vertex_buffer: vb.clone(),
                texcoord_buffer: uvb.clone(),
                index_buffer: ib.clone(),
                skinning_buffer: mesh.buffers["skinning"].clone(),
                vertices,
                indices,
                parts: mesh.parts,
                render_stage_part_ranges: mesh.render_stage_part_ranges,
                render_stage_input_layouts: mesh.render_stage_input_layouts,
            });
        }
        let geometry = Geometry {
            model: tag.to_string(),
            coordinate_space: "Marathon source model space",
            meshes,
        };
        Ok(geometry)
}

/// Decode only the requested entity. No payload is exported or persisted.
pub fn read_character(manager: &PackageManager, tag: TagHash) -> anyhow::Result<Vec<u8>> {
    ensure!(manager.get_entry(tag).context("entity missing")?.reference == 0x8080BAAD, "entity class mismatch {tag}");
    let bytes = manager.read_tag(tag)?;
    let mut bones = None;
    let mut render_model = None;
    let mut components = Vec::new();
    let mut attachments = Vec::new();
    for offset in array(&bytes, 8, 12, 0x8080BAA2)? {
        let component = TagHash(word(&bytes, offset)?);
        ensure!(manager.get_entry(component).context("component missing")?.reference == 0x8080BADB, "component class mismatch {component}");
        let data = manager.read_tag(component)?;
        let definition = resource(&data, 0x18)?;
        components.push(serde_json::json!({"tag":component.to_string(), "definition_class":definition.map(|(_,c)|format!("{c:08X}"))}));
        if let Some((base,class)) = definition {
            match class {
                0x80809FB7 | 0x80809FAF => {
                    ensure!(bones.is_none(), "multiple skeletons in {tag}");
                    bones = Some(skeleton(&data,component,base,class)?);
                }
                0x80809F76 => {
                    for attachment in array(&data,base+0xA8,0x30,0x80809F82)? {
                        attachments.push(Attachment {
                            name_hash:format!("{:08X}",word(&data,attachment+0x28)?),
                            bone_index:word(&data,attachment+0x24)? as i32,
                            transform:transform(&data,attachment)?,
                        });
                    }
                }
                0x80808678 => {
                    ensure!(render_model.is_none(), "multiple render models in {tag}");
                    render_model = Some(TagHash(word(&data,base+0x234)?));
                }
                _ => {}
            }
        }
    }
    let skeleton = bones.context("entity has no decoded skeleton")?;
    let model = render_model.context("entity has no render model")?;
    let geometry = decode_geometry(manager,model,Some(skeleton.bones.len()))?;
    Ok(serde_json::to_vec(&serde_json::json!({"entity":tag.to_string(),"components":components,"skeleton":skeleton,"geometry":geometry,"attachments":attachments}))?)
}

fn half(data: &[u8], offset: usize) -> anyhow::Result<u16> {
    Ok(u16::from_le_bytes(
        data.get(offset..offset + 2)
            .context("half outside payload")?
            .try_into()?,
    ))
}

fn vector(data: &[u8], offset: usize) -> anyhow::Result<[f32; 4]> {
    let mut result = [0.0; 4];
    for (index, value) in result.iter_mut().enumerate() {
        *value = f32::from_bits(word(data, offset + index * 4)?);
    }
    ensure!(
        result.iter().all(|v| v.is_finite()),
        "nonfinite model vector"
    );
    Ok(result)
}

fn model(manager: &PackageManager, tag: TagHash) -> anyhow::Result<Model> {
    ensure!(
        manager
            .get_entry(tag)
            .context("model entry missing")?
            .reference
            == 0x8080881C,
        "model class mismatch for {tag}"
    );
    let bytes = manager.read_tag(tag)?;
    let mut meshes = Vec::new();
    for base in array(&bytes, 0x10, 0x88, 0x808087CB)? {
        let mut buffers = BTreeMap::new();
        for (index, name) in [
            "vertex0", "vertex1", "buffer2", "buffer3", "index", "color", "skinning",
        ]
        .iter()
        .enumerate()
        {
            let hash = TagHash(word(&bytes, base + index * 4)?);
            buffers.insert(
                name.to_string(),
                if hash.is_none() {
                    None
                } else {
                    Some(hash.to_string())
                },
            );
        }
        let mut parts = Vec::new();
        for part in array(&bytes, base + 0x20, 0x28, 0x808087D1)? {
            parts.push(MeshPart {
                technique: TagHash(word(&bytes, part)?).to_string(),
                variant_shader_index: half(&bytes, part + 4)?,
                primitive_type: bytes[part + 6],
                index_start: word(&bytes, part + 8)?,
                index_count: word(&bytes, part + 12)?,
                external_identifier: half(&bytes, part + 0x14)?,
                flags: word(&bytes, part + 0x18)?,
                lod_category: bytes[part + 0x21],
            });
        }
        let ranges = (0..26)
            .map(|i| half(&bytes, base + 0x30 + i * 2))
            .collect::<anyhow::Result<Vec<_>>>()?;
        ensure!(
            ranges.windows(2).all(|v| v[0] <= v[1])
                && ranges.iter().all(|v| *v as usize <= parts.len()),
            "invalid model render-stage ranges for {tag}"
        );
        meshes.push(Mesh {
            buffers,
            parts,
            render_stage_part_ranges: ranges,
            render_stage_input_layouts: bytes[base + 0x64..base + 0x7D].to_vec(),
        });
    }
    Ok(Model {
        tag: tag.to_string(),
        scale: vector(&bytes, 0xA0)?,
        offset: vector(&bytes, 0xB0)?,
        meshes,
    })
}

fn transform(data: &[u8], offset: usize) -> anyhow::Result<Transform> {
    let mut values = [0.0; 8];
    for (index, value) in values.iter_mut().enumerate() {
        *value = f32::from_bits(word(data, offset + index * 4)?);
        ensure!(value.is_finite(), "nonfinite bone transform");
    }
    Ok(Transform {
        rotation: values[..4].try_into()?,
        translation_scale: values[4..].try_into()?,
    })
}

fn skeleton(data: &[u8], component: TagHash, base: usize, class: u32) -> anyhow::Result<Skeleton> {
    let field = base + if class == 0x80809FB7 { 0xB0 } else { 0x80 };
    let nodes = array(data, field, 16, 0x8080AF42)?;
    let objects = if class == 0x80809FB7 {
        Some(array(data, field + 16, 32, 0x8080BF47)?)
    } else {
        None
    };
    let inverses = array(
        data,
        field + if objects.is_some() { 32 } else { 16 },
        32,
        0x8080BF47,
    )?;
    ensure!(
        inverses.len() == nodes.len(),
        "inverse bind count mismatch for {component}"
    );
    if let Some(ref objects) = objects {
        ensure!(
            objects.len() == nodes.len(),
            "bind count mismatch for {component}"
        );
    }
    let mut bones = Vec::new();
    for (index, offset) in nodes.iter().enumerate() {
        let parent = word(data, offset + 4)? as i32;
        let first_child = word(data, offset + 8)? as i32;
        let next_sibling = word(data, offset + 12)? as i32;
        for link in [parent, first_child, next_sibling] {
            ensure!(
                link == -1 || (link >= 0 && (link as usize) < nodes.len()),
                "invalid bone link in {component}"
            );
        }
        bones.push(Bone {
            name_hash: format!("{:08X}", word(data, *offset)?),
            parent,
            first_child,
            next_sibling,
            object_transform: objects
                .as_ref()
                .map(|v| transform(data, v[index]))
                .transpose()?,
            inverse_object_transform: transform(data, inverses[index])?,
        });
    }
    Ok(Skeleton {
        component: component.to_string(),
        definition_class: format!("{class:08X}"),
        bones,
    })
}

fn word(data: &[u8], offset: usize) -> anyhow::Result<u32> {
    let bytes = data
        .get(offset..offset.checked_add(4).context("word offset overflow")?)
        .context("word outside payload")?;
    Ok(u32::from_le_bytes(bytes.try_into()?))
}

fn wide(data: &[u8], offset: usize) -> anyhow::Result<i64> {
    let bytes = data
        .get(offset..offset.checked_add(8).context("wide offset overflow")?)
        .context("wide outside payload")?;
    Ok(i64::from_le_bytes(bytes.try_into()?))
}

fn relative(data: &[u8], field: usize) -> anyhow::Result<Option<usize>> {
    let delta = wide(data, field)?;
    if delta == 0 || delta == -1 {
        return Ok(None);
    }
    let target = usize::try_from(
        (field as i64)
            .checked_add(delta)
            .context("relative offset overflow")?,
    )?;
    ensure!(target < data.len(), "relative pointer outside payload");
    Ok(Some(target))
}

fn array(data: &[u8], field: usize, stride: usize, class: u32) -> anyhow::Result<Vec<usize>> {
    let count = usize::try_from(wide(data, field)?)?;
    if count == 0 {
        return Ok(Vec::new());
    }
    let header = relative(data, field + 8)?.context("nonempty array has null pointer")?;
    ensure!(
        usize::try_from(wide(data, header)?)? == count,
        "array length mismatch"
    );
    ensure!(
        word(data, header + 8)? == class,
        "array element class mismatch at {field:X}"
    );
    let start = header.checked_add(16).context("array start overflow")?;
    let size = count.checked_mul(stride).context("array length overflow")?;
    ensure!(
        start.checked_add(size).context("array end overflow")? <= data.len(),
        "array outside payload"
    );
    Ok((0..count).map(|i| start + i * stride).collect())
}

fn resource(data: &[u8], field: usize) -> anyhow::Result<Option<(usize, u32)>> {
    relative(data, field)?
        .map(|offset| {
            let class_offset = offset.checked_sub(4).context("resource class underflow")?;
            Ok((offset, word(data, class_offset)?))
        })
        .transpose()
}

pub fn catalog(manager: &PackageManager, package_name: &str, output: &Path) -> anyhow::Result<()> {
    let mut entities = Vec::new();
    let mut skeletons = BTreeMap::new();
    let mut models = BTreeMap::new();
    let mut tags = manager.get_all_by_reference(0x8080BAAD);
    tags.sort_by_key(|(tag, _)| *tag);
    for (tag, _) in tags {
        let package = manager
            .package_paths
            .get(&tag.pkg_id())
            .context("entity package missing")?;
        if !package.name.contains(package_name) {
            continue;
        }
        let data = manager
            .read_tag(tag)
            .with_context(|| format!("entity {tag}"))?;
        let refs =
            array(&data, 8, 12, 0x8080BAA2).with_context(|| format!("entity {tag} components"))?;
        let mut components = Vec::new();
        let mut name = None;
        for offset in refs {
            let component_tag = TagHash(word(&data, offset)?);
            let entry = manager
                .get_entry(component_tag)
                .context("component entry missing")?;
            ensure!(
                entry.reference == 0x8080BADB,
                "component {component_tag} class mismatch"
            );
            let bytes = manager
                .read_tag(component_tag)
                .with_context(|| format!("component {component_tag}"))?;
            let instance = resource(&bytes, 0x10)?;
            let definition = resource(&bytes, 0x18)?;
            let mut component = Component {
                attachments: Vec::new(),
                tag: component_tag.to_string(),
                instance_class: instance.map(|(_, c)| format!("{c:08X}")),
                definition_class: definition.map(|(_, c)| format!("{c:08X}")),
                definition_offset: definition.map(|(o, _)| o),
                model: None,
                skeleton_bones: None,
            };
            if let Some((base, class)) = definition {
                match class {
                    0x80809F76 => {
                        for attachment in array(&bytes, base + 0xA8, 0x30, 0x80809F82)? {
                            component.attachments.push(Attachment {
                                name_hash: format!("{:08X}", word(&bytes, attachment + 0x28)?),
                                bone_index: word(&bytes, attachment + 0x24)? as i32,
                                transform: transform(&bytes, attachment)?,
                            });
                        }
                    }
                    0x808035B7 => {
                        name = localized_name(manager, &bytes, base)
                            .with_context(|| format!("entity name {tag}/{component_tag}"))?;
                    }
                    0x80808678 | 0x80808655 => {
                        let model_tag = TagHash(word(&bytes, base + 0x234)?);
                        component.model = Some(model_tag.to_string());
                        if let std::collections::btree_map::Entry::Vacant(entry) =
                            models.entry(model_tag)
                        {
                            entry.insert(
                                model(manager, model_tag).with_context(|| {
                                    format!("model {model_tag} for entity {tag}")
                                })?,
                            );
                        }
                    }
                    0x80809FB7 | 0x80809FAF => {
                        let decoded = skeleton(&bytes, component_tag, base, class)
                            .with_context(|| format!("skeleton component {component_tag}"))?;
                        component.skeleton_bones = Some(decoded.bones.len());
                        skeletons.insert(component_tag, decoded);
                    }
                    _ => {}
                }
            }
            components.push(component);
        }
        entities.push(Entity {
            tag: tag.to_string(),
            package: package.filename.clone(),
            name,
            components,
        });
    }
    ensure!(
        !entities.is_empty(),
        "no entities matched package name {package_name}"
    );
    fs::create_dir_all(output)?;
    fs::write(
        output.join("entities.json"),
        serde_json::to_vec_pretty(&entities)?,
    )?;
    fs::write(
        output.join("skeletons.json"),
        serde_json::to_vec_pretty(&skeletons.values().collect::<Vec<_>>())?,
    )?;
    fs::write(
        output.join("models.json"),
        serde_json::to_vec_pretty(&models.values().collect::<Vec<_>>())?,
    )?;
    println!(
        "Decoded {} entities and {} components",
        entities.len(),
        entities.iter().map(|e| e.components.len()).sum::<usize>()
    );
    Ok(())
}
