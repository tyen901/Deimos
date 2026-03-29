use std::sync::Arc;

use anyhow::Context;
use bytemuck::{Pod, Zeroable};
use crossbeam::queue::SegQueue;
use d3d12::{
    ElementOffset, Format, GraphicsPipelineStateDesc, RootSignatureBuilder, RootSignatureFlags,
};
use deimos_data::tfx::{PrimitiveType, ShaderStage, geometry::AxisAlignedBBox};
use glam::{Vec3, Vec4Swizzles, vec3};
use lazy_static::lazy_static;

use crate::gpu::{Gpu, command_list::CommandList};

lazy_static! {
    pub static ref IMMEDIATE_SHAPES: SegQueue<ImmediateShape> = SegQueue::default();
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct ImmediateVertex {
    position: Vec3,
    color: [u8; 4],
}

pub struct ImmediateRenderer {
    rs_immediate: d3d12::RootSignature,
    pso_immediate: d3d12::PipelineState,
}

impl ImmediateRenderer {
    pub fn new(gpu: &Arc<Gpu>) -> anyhow::Result<Self> {
        let root_signature_raw = RootSignatureBuilder::default()
            .flags(RootSignatureFlags::ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT)
            .with_param(
                d3d12::RootParameter::CbvDescriptor {
                    shader_register: 12,
                    register_space: 0,
                },
                d3d12::ShaderVisibility::Vertex,
            )
            .serialize()?;

        let layout = vec![
            d3d12::InputElementDesc::builder()
                .semantic_name("POSITION")
                .semantic_index(0)
                .format(d3d12::Format::R32g32b32Float)
                .aligned_byte_offset(ElementOffset::Absolute(0))
                .build(),
            d3d12::InputElementDesc::builder()
                .semantic_name("COLOR")
                .semantic_index(0)
                .format(d3d12::Format::R8g8b8a8Unorm)
                .aligned_byte_offset(ElementOffset::Append)
                .build(),
        ];

        let rs_immediate = gpu
            .create_root_signature(&root_signature_raw)
            .context("rs_immediate")?;
        let pso_immediate = gpu
            .create_graphics_pipeline_state(
                &GraphicsPipelineStateDesc::new(&rs_immediate)
                    .with_vs(include_bytes!("../../builtin/shaders/immediate.vs.dxil"))
                    .with_ps(include_bytes!("../../builtin/shaders/immediate.ps.dxil"))
                    .with_primitive_topology(d3d12::PrimitiveTopology2::Line)
                    .with_input_layout(&layout)
                    .with_rtv_formats(&[Format::R11g11b10Float]),
            )
            .context("pso_immediate")?;

        Ok(Self {
            rs_immediate,
            pso_immediate,
        })
    }

    pub fn draw_shapes(&self, cmd: &mut CommandList) {
        let mut vertices = vec![];

        while let Some(shape) = IMMEDIATE_SHAPES.pop() {
            emit_shape(shape, &mut vertices);
        }

        if vertices.is_empty() {
            return;
        }

        let num_vertices = vertices.len() as u32;

        let vb = match cmd
            .upload_ring()
            .upload_bytes(bytemuck::cast_slice(&vertices))
        {
            Ok(vb) => vb,
            Err(e) => {
                warn!("Failed to upload immediate vertex buffer: {}", e);
                return;
            }
        };

        cmd.set_input_topology(PrimitiveType::LineList);
        cmd.set_input_layout(0);
        cmd.flush_states();

        cmd.set_root_signature(&self.rs_immediate);
        cmd.set_pipeline_state(&self.pso_immediate);

        let Some(view_scope_cbv) = cmd.get_shader_constant_buffer_view(ShaderStage::Vertex, 12)
        else {
            return;
        };

        cmd.set_graphics_root_constant_buffer_view(0, view_scope_cbv);

        cmd.ia_set_vertex_buffers(
            0,
            &[d3d12::VertexBufferView::new(
                vb,
                num_vertices * size_of::<ImmediateVertex>() as u32,
                size_of::<ImmediateVertex>() as u32,
            )],
        );

        cmd.draw_instanced(0..num_vertices, 0..1);
    }
}

pub enum ImmediatePrimitive {
    BoundingBox(AxisAlignedBBox),
}

pub struct ImmediateShape {
    pub primitive: ImmediatePrimitive,
    pub color: [u8; 4],
}

fn emit_shape(shape: ImmediateShape, out: &mut Vec<ImmediateVertex>) {
    match shape.primitive {
        ImmediatePrimitive::BoundingBox(bb) => {
            let a = bb.min.xyz();
            let b = bb.max.xyz();

            // +Z
            // ^  .1------b
            // |.' |    .'|
            // +------2'  | +X
            // |   |  |   | /
            // |  ,+--+---3
            // |.'    | .'
            // a------+'   -> -Y

            // Point axis (A)
            emit_line(a, vec3(b.x, a.y, a.z), shape.color, out); // X
            emit_line(a, vec3(a.x, b.y, a.z), shape.color, out); // Y
            emit_line(a, vec3(a.x, a.y, b.z), shape.color, out); // Z

            // Point axis (B)
            emit_line(b, vec3(a.x, b.y, b.z), shape.color, out); // X
            emit_line(b, vec3(b.x, a.y, b.z), shape.color, out); // Y
            emit_line(b, vec3(b.x, b.y, a.z), shape.color, out); // Z

            let c1 = vec3(b.x, a.y, b.z);
            let c2 = vec3(a.x, b.y, b.z);
            let c3 = vec3(b.x, b.y, a.z);

            // Infill corner 1
            emit_line(c1, vec3(c1.x, c1.y, a.z), shape.color, out);
            emit_line(c1, vec3(a.x, c1.y, c1.z), shape.color, out);

            // Infill corner 2
            emit_line(c2, vec3(c2.x, a.y, c2.z), shape.color, out);
            emit_line(c2, vec3(c2.x, c2.y, a.z), shape.color, out);

            // Infill corner 3
            emit_line(c3, vec3(a.x, c3.y, c3.z), shape.color, out);
            emit_line(c3, vec3(c3.x, a.y, c3.z), shape.color, out);
        }
    }
}

fn emit_line(start: Vec3, end: Vec3, color: [u8; 4], out: &mut Vec<ImmediateVertex>) {
    out.push(ImmediateVertex {
        position: start,
        color,
    });
    out.push(ImmediateVertex {
        position: end,
        color,
    });
}
