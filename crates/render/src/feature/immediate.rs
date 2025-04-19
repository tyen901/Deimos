use anyhow::Context;
use d3d11::{dxgi, InputElementDesc, ShaderTarget};
use glam::Vec3;

use crate::{gpu::command_list::CommandList, gpu_span, Gpu};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ImmediateVertex {
    pos: Vec3,
    color: u32,
}

pub struct ImmediateShapeRenderer {
    line_vertices: Vec<ImmediateVertex>,
    vbuffer: d3d11::Buffer,
    vbuffer_capacity: usize,
    vbuffer_len: usize,

    shader_vs: d3d11::VertexShader,
    shader_ps: d3d11::PixelShader,
    input_layout: d3d11::InputLayout,
}

const IMMEDIATE_SHADER: &str = include_str!("../../builtin/shaders/immediate.hlsl");

impl ImmediateShapeRenderer {
    const DEFAULT_CAPACITY: usize = 2048;

    pub fn new(gpu: &Gpu) -> anyhow::Result<Self> {
        let vs_data = d3d11::shader::fxc_compile(
            IMMEDIATE_SHADER.as_bytes(),
            Some("immediate_vs"),
            &[],
            "mainVS",
            ShaderTarget::Vertex,
        )
        .context("Failed to compile vertex shader")?;

        let ps_data = d3d11::shader::fxc_compile(
            IMMEDIATE_SHADER.as_bytes(),
            Some("immediate_ps"),
            &[],
            "mainPS",
            ShaderTarget::Pixel,
        )
        .context("Failed to compile pixel shader")?;
        let shader_vs = gpu
            .create_vertex_shader(&vs_data)
            .context("Failed to create vertex shader")?;
        let shader_ps = gpu
            .create_pixel_shader(&ps_data)
            .context("Failed to create pixel shader")?;

        let input_layout = gpu
            .create_input_layout(
                &[
                    InputElementDesc::builder()
                        .semantic_name("POSITION")
                        .semantic_index(0)
                        .format(dxgi::Format::R32g32b32Float)
                        .input_slot(0)
                        .input_slot_class(d3d11::InputClassification::PerVertexData)
                        .build(),
                    InputElementDesc::builder()
                        .semantic_name("COLOR")
                        .semantic_index(0)
                        .format(dxgi::Format::B8g8r8a8Unorm)
                        .input_slot(0)
                        .input_slot_class(d3d11::InputClassification::PerVertexData)
                        .build(),
                ],
                &vs_data,
            )
            .context("Failed to create input layout")?;

        Ok(Self {
            line_vertices: Vec::new(),
            vbuffer: Self::create_vb(gpu, Self::DEFAULT_CAPACITY),
            vbuffer_capacity: Self::DEFAULT_CAPACITY,
            vbuffer_len: 0,

            shader_vs,
            shader_ps,
            input_layout,
        })
    }

    fn create_vb(gpu: &Gpu, vertices: usize) -> d3d11::Buffer {
        gpu.create_buffer(
            &d3d11::BufferDesc::builder()
                .byte_width((vertices * std::mem::size_of::<ImmediateVertex>()) as u32)
                .usage(d3d11::Usage::Dynamic)
                .bind_flags(d3d11::BindFlags::VERTEX_BUFFER)
                .cpu_access_flags(d3d11::CpuAccessFlags::WRITE)
                .build(),
            None,
        )
        .unwrap()
    }

    // pub fn draw_line(&mut self, start: Vec3, end: Vec3, color: u32) {
    //     self.line_vertices.push(Vertex { pos: start, color });
    //     self.line_vertices.push(Vertex { pos: end, color });
    // }

    pub fn add_vertices(&mut self, vertices: &[ImmediateVertex]) {
        self.line_vertices.extend_from_slice(vertices);
    }

    #[profiling::function]
    pub fn prepare(&mut self, gpu: &crate::Gpu) {
        if self.line_vertices.len() > self.vbuffer_capacity {
            self.vbuffer = Self::create_vb(gpu, self.line_vertices.len());
            self.vbuffer_capacity = self.line_vertices.len();
        }

        unsafe {
            let ptr = gpu
                .context()
                .map(&self.vbuffer, 0, d3d11::MapType::WriteDiscard, false)
                .unwrap();

            std::ptr::copy_nonoverlapping(
                self.line_vertices.as_ptr(),
                ptr.data as *mut ImmediateVertex,
                self.line_vertices.len(),
            );
        }

        self.vbuffer_len = self.line_vertices.len();
        self.line_vertices.clear();
    }

    pub fn submit(&self, cmd: &mut CommandList) {
        gpu_span!();
        cmd.input_assembler_set_vertex_buffers(
            0,
            &[Some(self.vbuffer.clone())],
            Some(&[16]),
            Some(&[0u32]),
        )
        .unwrap();
        cmd.input_assembler_set_input_layout(&self.input_layout);
        cmd.set_input_topology(deimos_data::tfx::PrimitiveType::LineList);

        cmd.output_merger_set_blend_state(None, Some(&[1.0, 1.0, 1.0, 1.0]), 0xFFFF_FFFF);

        cmd.vertex_set_shader(&self.shader_vs);
        cmd.pixel_set_shader(&self.shader_ps);

        cmd.draw(self.vbuffer_len as u32, 0);
    }
}
