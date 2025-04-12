use std::fs::File;

use anyhow::Context;
use deimos_core::package_manager;
use deimos_data::tfx::{
    buffers::{IndexBufferHeader, VertexBufferHeader},
    features::dynamic::{SDynamicMeshPart, SDynamicModel},
    render_globals::SRenderGlobals,
};
use destiny_pkg::TagHash;
use gltf::{
    buffer::View,
    validation::{Checked, USize64, Validate},
    Buffer, Index,
};
use serde_json::json;
use tiger_parse::PackageManagerExt;

mod layout;

fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let h_entity = TagHash(0x80E2C914);
    let h_model = TagHash(0x80E2C8EE);

    load_input_layouts()?;

    let model: SDynamicModel = package_manager().read_tag_struct(h_model)?;
    let mut root = gltf::Root {
        extensions_required: vec!["KHR_mesh_quantization".to_owned()],
        extensions_used: vec!["KHR_mesh_quantization".to_owned()],
        ..Default::default()
    };

    let mut scene = gltf::Scene {
        nodes: vec![],
        extensions: None,
        extras: Default::default(),
    };

    let mut gmesh = gltf::Mesh {
        extensions: None,
        extras: Default::default(),
        primitives: vec![],
        weights: None,
    };

    for mesh in model.meshes {
        let (vb0, vertex_count, vb0_stride) = load_vertex_buffer(&mut root, mesh.vertex0_buffer)?;
        let (ib, ib_is_32bit, new_parts) =
            load_index_buffer(&mut root, mesh.index_buffer, &mesh.parts)?;
        println!("Loaded vertex buffer with stride {}", vb0_stride);

        let range = mesh.get_range_for_stage(deimos_data::tfx::RenderStage::GenerateGbuffer);
        for part in new_parts[range]
            .iter()
            .filter(|p| p.lod_category.is_highest_detail())
        {
            let ib_accessor = root.push(gltf::Accessor {
                buffer_view: Some(ib),
                byte_offset: Some(USize64(
                    part.index_start as u64 * if ib_is_32bit { 4 } else { 2 },
                )),
                type_: Checked::Valid(gltf::accessor::Type::Scalar),
                count: USize64(part.index_count as _),
                component_type: Checked::Valid(gltf::accessor::GenericComponentType(
                    // if ib_is_32bit {
                    gltf::accessor::ComponentType::U32,
                    // } else {
                    //     gltf::accessor::ComponentType::U16
                    // },
                )),
                extensions: None,
                extras: Default::default(),
                min: None,
                max: None,
                normalized: false,
                sparse: None,
            });

            let mut prim = gltf::mesh::Primitive {
                extensions: None,
                extras: Default::default(),
                attributes: Default::default(),
                indices: Some(ib_accessor),
                mode: Checked::Valid(gltf::mesh::Mode::Triangles),
                material: None,
                targets: None,
            };

            let accessor = root.push(gltf::Accessor {
                buffer_view: Some(vb0),
                // byte_offset: Some(USize64(part.index_start as u64 * vb0_stride)),
                // count: USize64(part.index_count as _),
                byte_offset: None,
                count: USize64(vertex_count as _),
                component_type: Checked::Valid(gltf::accessor::GenericComponentType(
                    gltf::accessor::ComponentType::I16,
                )),
                type_: Checked::Valid(gltf::accessor::Type::Vec3),
                min: None,
                max: None,
                normalized: true,
                sparse: None,
                extensions: None,
                extras: Default::default(),
            });

            prim.attributes
                .insert(Checked::Valid(gltf::mesh::Semantic::Positions), accessor);

            let accessor = root.push(gltf::Accessor {
                buffer_view: Some(vb0),
                // byte_offset: Some(USize64(part.index_start as u64 * vb0_stride)),
                // count: USize64(part.index_count as _),
                byte_offset: Some(USize64(8)),
                count: USize64(vertex_count as _),
                component_type: Checked::Valid(gltf::accessor::GenericComponentType(
                    gltf::accessor::ComponentType::I16,
                )),
                type_: Checked::Valid(gltf::accessor::Type::Vec3),
                min: None,
                max: None,
                normalized: true,
                sparse: None,
                extensions: None,
                extras: Default::default(),
            });

            prim.attributes
                .insert(Checked::Valid(gltf::mesh::Semantic::Normals), accessor);

            gmesh.primitives.push(prim);
        }
    }

    let node = gltf::Node {
        mesh: Some(root.push(gmesh)),
        ..Default::default()
    };

    scene.nodes.push(root.push(node));

    root.push(scene);

    let mut f = File::create("output.gltf").unwrap();
    serde_json::to_writer_pretty(&mut f, &root)?;

    Ok(())
}

fn load_vertex_buffer(
    root: &mut gltf::Root,
    vb: TagHash,
) -> anyhow::Result<(Index<gltf::buffer::View>, usize, u64)> {
    let header: VertexBufferHeader = package_manager().read_tag_struct(vb)?;
    let entry = package_manager().get_entry(vb).unwrap();

    let buffer = root.push(Buffer {
        byte_length: USize64(header.data_size as _),
        uri: Some(format!("vb_{vb}.bin")),
        extensions: None,
        extras: Default::default(),
    });

    let data = package_manager().read_tag(entry.reference)?;
    std::fs::write(format!("vb_{vb}.bin"), data)?;

    Ok((
        root.push(gltf::buffer::View {
            buffer,
            byte_length: USize64(header.data_size as _),
            byte_offset: None,
            byte_stride: Some(gltf::buffer::Stride(header.stride as _)),
            target: Some(Checked::Valid(gltf::buffer::Target::ArrayBuffer)),
            extensions: None,
            extras: Default::default(),
        }),
        header.data_size as usize / header.stride as usize,
        header.stride as u64,
    ))
}

fn load_index_buffer(
    root: &mut gltf::Root,
    ib: TagHash,
    parts: &[SDynamicMeshPart],
) -> anyhow::Result<(Index<gltf::buffer::View>, bool, Vec<SDynamicMeshPart>)> {
    let header: IndexBufferHeader = package_manager().read_tag_struct(ib)?;
    let entry = package_manager().get_entry(ib).unwrap();

    let data = package_manager().read_tag(entry.reference)?;
    let indices: Vec<u32> = if header.is_32bit {
        bytemuck::cast_slice(data.as_slice()).to_vec()
    } else {
        bytemuck::cast_slice::<u8, u16>(data.as_slice())
            .iter()
            .map(|&x| if x == u16::MAX { u32::MAX } else { x as u32 })
            .collect()
    };

    let mut new_parts = vec![];
    let mut new_indices = vec![];

    for p in parts {
        let range = p.index_start as usize..(p.index_start + p.index_count) as usize;
        let start = new_indices.len();
        let mut i = 0;
        for start in range {
            if start + 3 > indices.len() {
                break;
            }

            let tri = &indices[start..start + 3];
            if tri.contains(&u32::MAX) {
                i = 0;
                continue;
            }

            let flip_vertices = i % 2 != 0;
            if flip_vertices {
                new_indices.push(tri[1]);
                new_indices.push(tri[0]);
                new_indices.push(tri[2]);
            } else {
                new_indices.push(tri[0]);
                new_indices.push(tri[1]);
                new_indices.push(tri[2]);
            }
            i += 1;
        }

        new_parts.push(SDynamicMeshPart {
            index_start: start as u32,
            index_count: new_indices.len() as u32 - start as u32,
            ..*p
        });
    }

    let data: &[u8] = bytemuck::cast_slice(&new_indices);
    let buffer = root.push(Buffer {
        byte_length: USize64(data.len() as _),
        uri: Some(format!("ib_{ib}.bin")),
        extensions: None,
        extras: Default::default(),
    });

    std::fs::write(format!("ib_{ib}.bin"), data)?;

    let view = root.push(gltf::buffer::View {
        buffer,
        byte_length: USize64(data.len() as _),
        byte_offset: None,
        byte_stride: None, //Some(gltf::buffer::Stride(if header.is_32bit { 4 } else { 2 })),
        extensions: None,
        extras: Default::default(),
        target: Some(Checked::Valid(gltf::buffer::Target::ElementArrayBuffer)),
    });

    Ok((view, true, new_parts))
}

fn load_input_layouts() -> anyhow::Result<()> {
    let data: SRenderGlobals = package_manager().read_named_tag_struct("render_globals")?;
    let globs = &data.unk8[0].unk8;
    let element_set = &globs.input_layouts.elements_c;
    // for l in &globs.input_layouts.mapping.layouts {
    //     let mut layout_elements = vec![];
    //     for (buffer_index, &(element_index, is_instance_data)) in [
    //         (l.buffer_0, l.buffer_0_instanced),
    //         (l.buffer_1, l.buffer_1_instanced),
    //         (l.buffer_2, l.buffer_2_instanced),
    //         (l.buffer_3, l.buffer_3_instanced),
    //     ]
    //     .iter()
    //     .enumerate()
    //     {
    //         if element_index == u32::MAX {
    //             continue;
    //         }
    //         for e in &element_set.sets[element_index as usize].elements {
    //             let semantic = INPUT_SEMANTICS[e.semantic as usize];
    //             let format = &INPUT_FORMATS[e.format as usize];
    //             layout_elements.push(TigerInputLayoutElement {
    //                 hlsl_type: format.hlsl_type,
    //                 format: format.format,
    //                 stride: format.stride,
    //                 semantic_name: semantic,
    //                 semantic_index: e.semantic_index as _,
    //                 buffer_index: buffer_index as _,
    //                 is_instance_data,
    //             });
    //         }
    //     }

    //     let layout = Self::create_input_layout(device, &layout_elements)?;
    //     layout.set_debug_name(format!("stream_input_layout_{}", l.index));
    //     input_layouts[l.index as usize] = Some(layout);
    // }

    Ok(())
}
