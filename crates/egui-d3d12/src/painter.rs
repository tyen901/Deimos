use std::{mem::size_of, sync::Arc, time::Instant};

use d3d12::{
    ElementOffset, Format, GraphicsPipelineStateDesc, RootSignatureBuilder, RootSignatureFlags,
    VertexBufferView,
};
use deimos_render::gpu::{buffer::Buffer, Gpu};
use egui::{epaint::Primitive, Context};

use crate::{
    mesh::{create_index_buffer, create_vertex_buffer, GpuMesh, GpuVertex},
    texture::TextureAllocator,
    RenderError,
};

/// Heart and soul of this integration.
/// Main methods you are going to use are:
/// * [`Self::present`] - Should be called inside of hook or before present.
/// * [`Self::resize_buffers`] - Should be called **INSTEAD** of swapchain's `ResizeBuffers`.
/// * [`Self::wnd_proc`] - Should be called on each `WndProc`.
pub struct D3D12Renderer {
    // render_view: Option<d3d12::RenderTargetView>,
    tex_alloc: TextureAllocator,
    pipeline: d3d12::PipelineState,
    // input_layout: d3d12::InputLayout,
    // shaders: CompiledShaders,
    // // backup: BackupState,
    // // hwnd: HWND,
    // samplers: [d3d12::SamplerState; 2],
    // blend_state: d3d12::BlendState,
    // raster_state: d3d12::RasterizerState,
    buffers: [Vec<(Buffer, Buffer)>; Gpu::FRAMES_IN_FLIGHT],
}

// impl D3D12Renderer {
//     const INPUT_ELEMENTS_DESC: [d3d12::InputElementDesc; 3] = [
//         d3d12::InputElementDesc::builder()
//             .semantic_name("POSITION")
//             .semantic_index(0)
//             .format(Format::R32g32Float)
//             .input_slot(0)
//             .aligned_byte_offset(ElementOffset::Absolute(0))
//             .input_slot_class(d3d12::InputClassification::PerVertexData)
//             .instance_data_step_rate(0)
//             .build(),
//         d3d12::InputElementDesc::builder()
//             .semantic_name("TEXCOORD")
//             .semantic_index(0)
//             .format(Format::R32g32Float)
//             .input_slot(0)
//             .aligned_byte_offset(ElementOffset::Append)
//             .input_slot_class(d3d12::InputClassification::PerVertexData)
//             .instance_data_step_rate(0)
//             .build(),
//         d3d12::InputElementDesc::builder()
//             .semantic_name("COLOR")
//             .semantic_index(0)
//             .format(Format::R32g32b32a32Float)
//             .input_slot(0)
//             .aligned_byte_offset(ElementOffset::Append)
//             .input_slot_class(d3d12::InputClassification::PerVertexData)
//             .instance_data_step_rate(0)
//             .build(),
//     ];
// }

impl D3D12Renderer {
    /// Create a new directx11 renderer from a swapchain
    pub fn new(gpu: &Gpu) -> Result<Self, RenderError> {
        let input_layout = [
            d3d12::InputElementDesc::builder()
                .semantic_name("POSITION")
                .semantic_index(0)
                .format(Format::R32g32Float)
                .input_slot(0)
                .aligned_byte_offset(ElementOffset::Absolute(0))
                .instance_data_step_rate(0)
                .build(),
            d3d12::InputElementDesc::builder()
                .semantic_name("TEXCOORD")
                .semantic_index(0)
                .format(Format::R32g32Float)
                .input_slot(0)
                .aligned_byte_offset(ElementOffset::Append)
                .instance_data_step_rate(0)
                .build(),
            d3d12::InputElementDesc::builder()
                .semantic_name("COLOR")
                .semantic_index(0)
                .format(Format::R32g32b32a32Float)
                .input_slot(0)
                .aligned_byte_offset(ElementOffset::Append)
                .instance_data_step_rate(0)
                .build(),
        ];

        let root_signature_raw = RootSignatureBuilder::default()
            .flags(RootSignatureFlags::ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT)
            .serialize()?;

        let root_signature = gpu.create_root_signature(&root_signature_raw)?;
        let pipeline = gpu.create_graphics_pipeline_state(
            &GraphicsPipelineStateDesc::new(&root_signature)
                .vs_bytecode(include_bytes!("shader/vertex.dxil"))
                .ps_bytecode(include_bytes!("shader/pixel.dxil"))
                .rtv_formats(&[Format::R8g8b8a8Unorm])
                .input_layout(&input_layout),
        )?;

        // let samplers = {
        //     let desc = d3d12::SamplerDesc::builder()
        //         .filter(d3d12::Filter::MinMagMipLinear)
        //         .address_u(d3d12::TextureAddress::Border)
        //         .address_v(d3d12::TextureAddress::Border)
        //         .address_w(d3d12::TextureAddress::Border)
        //         .border_color([1., 1., 1., 1.])
        //         .build();

        //     let sampler_linear = gpu.create_sampler_state(&desc)?;
        //     let sampler_point = gpu.create_sampler_state(&d3d12::SamplerDesc {
        //         filter: d3d12::Filter::MinMagMipPoint,
        //         ..desc
        //     })?;

        //     [sampler_linear, sampler_point]
        // };

        // let blend_desc = d3d12::BlendDesc::from_single_target(
        //     d3d12::RenderTargetBlendDesc::builder()
        //         .blend_enable(true)
        //         .src_blend(d3d12::Blend::SrcAlpha)
        //         .dest_blend(d3d12::Blend::InvSrcAlpha)
        //         .blend_op(d3d12::BlendOp::Add)
        //         .src_blend_alpha(d3d12::Blend::One)
        //         .dest_blend_alpha(d3d12::Blend::InvSrcAlpha)
        //         .blend_op_alpha(d3d12::BlendOp::Add)
        //         .render_target_write_mask(15)
        //         .build(),
        // );
        // let raster_desc = d3d12::RasterizerDesc::builder()
        //     .fill_mode(d3d12::FillMode::Solid)
        //     .cull_mode(d3d12::CullMode::None)
        //     .front_counter_clockwise(false)
        //     .depth_bias(0)
        //     .depth_bias_clamp(0.)
        //     .slope_scaled_depth_bias(0.)
        //     .depth_clip_enable(false)
        //     .scissor_enable(true)
        //     .multisample_enable(false)
        //     .antialiased_line_enable(false)
        //     .build();

        // let blend_state = gpu.create_blend_state(&blend_desc)?;
        // let raster_state = gpu.create_rasterizer_state(&raster_desc)?;

        Ok(Self {
            tex_alloc: TextureAllocator::default(),
            pipeline,
            // // backup: BackupState::default(),
            // input_layout,
            // render_view: Some(render_view),
            // shaders,
            // // hwnd,
            // samplers,
            // blend_state,
            // raster_state,
            buffers: [const { Vec::new() }; Gpu::FRAMES_IN_FLIGHT],
        })
    }
}

impl D3D12Renderer {
    /// Present call. Should be called once per original present call, before or inside of hook.
    #[allow(invalid_reference_casting)]
    pub fn paint(
        &mut self,
        gpu: &Arc<Gpu>,
        cmd: &d3d12::GraphicsCommandList,
        output: egui::FullOutput,
        context: &Context,
        screen_size: (u32, u32), // mut paint: PaintFn,
    ) -> Result<egui::FullOutput, RenderError>
// where
    //     PaintFn: FnMut(&mut Self, &Context),
    {
        // self.backup.save(ctx);
        // let screen = cmd.gpu().swapchain_resolution();

        // if !output.textures_delta.is_empty() {
        //     self.tex_alloc
        //         .process_deltas(gpu, cmd, &output.textures_delta)?;
        // }

        // if output.shapes.is_empty() {
        //     // self.backup.restore(ctx);
        //     return Ok(output);
        // }

        let primitives = context
            .tessellate(output.shapes.clone(), output.pixels_per_point)
            .into_iter()
            .filter_map(|prim| {
                if let Primitive::Mesh(mesh) = prim.primitive {
                    GpuMesh::from_mesh(
                        (screen_size.0 as f32, screen_size.1 as f32),
                        mesh,
                        prim.clip_rect,
                        output.pixels_per_point,
                    )
                } else {
                    panic!("Paint callbacks are not yet supported")
                }
            })
            .collect::<Vec<_>>();

        // cmd.output_merger_set_blend_state(&self.blend_state, Some(&[0., 0., 0., 0.]), 0xffffffff);
        // cmd.rasterizer_set_state(&self.raster_state);

        cmd.set_viewports(&[d3d12::Viewport::builder()
            .width(screen_size.0 as f32)
            .height(screen_size.1 as f32)
            .build()]);
        // cmd.output_merger_set_render_targets(&[self.render_view.as_ref()], None);
        // #[allow(deprecated)]
        // cmd.input_assembler_set_input_layout(&self.input_layout);
        // #[allow(deprecated)]
        // cmd.input_assembler_set_primitive_topology(d3d12::PrimitiveTopology::TriangleList);
        cmd.set_pipeline_state(&self.pipeline);

        let buffers = &mut self.buffers[gpu.frame_index() % Gpu::FRAMES_IN_FLIGHT];
        buffers.clear();

        cmd.input_assembler_set_primitive_topology(d3d12::PrimitiveTopology::TriangleList);
        for mesh in primitives {
            let vtx = create_vertex_buffer(gpu, &mesh)?;
            let idx = create_index_buffer(gpu, &mesh)?;

            cmd.input_assembler_set_vertex_buffers(
                0,
                &[VertexBufferView {
                    buffer_location: vtx.gpu_virtual_address(),
                    size_in_bytes: vtx.size() as u32,
                    stride_in_bytes: size_of::<GpuVertex>() as u32,
                }],
            );
            cmd.input_assembler_set_index_buffer(
                idx.gpu_virtual_address(),
                idx.size() as u32,
                Format::R32Uint,
            );
            buffers.push((vtx, idx));

            //     let texture = self.tex_alloc.get_by_id(mesh.texture_id);

            cmd.set_scissor_rects(&[d3d12::Rect {
                left: mesh.clip.left() as _,
                top: mesh.clip.top() as _,
                right: mesh.clip.right() as _,
                bottom: mesh.clip.bottom() as _,
            }]);

            //     let mut use_alpha = false;
            //     if let Some((texture, texture_filter, texture_uses_alpha)) = &texture {
            //         use_alpha = *texture_uses_alpha;
            //         self.set_sampler_state(cmd, texture_filter.unwrap_or(egui::TextureFilter::Linear))?;
            //         cmd.pixel_set_shader_resources(0, &[Some(texture)]);
            //     }

            //     cmd.input_assembler_set_vertex_buffers(
            //         0,
            //         &[Some(&vtx)],
            //         Some(&[size_of::<GpuVertex>() as _]),
            //         Some(&[0]),
            //     )?;
            //     cmd.input_assembler_set_index_buffer(&idx, Format::R32Uint, 0);
            //     cmd.vertex_set_shader(&self.shaders.vertex);
            //     cmd.pixel_set_shader(if use_alpha {
            //         &self.shaders.pixel
            //     } else {
            //         &self.shaders.pixel_no_alpha
            //     });

            cmd.draw_indexed_instanced(0..mesh.indices.len() as _, 0..1, 0);

            //     if texture.is_some() {
            //         self.set_sampler_state(cmd, egui::TextureFilter::Linear)?;
            //     }
        }

        // // self.backup.restore(ctx);
        self.textures_mut().clear_temporaries();

        Ok(output)
    }

    // /// Call when resizing buffers.
    // /// Do not call the original function before it, instead call it inside of the `original` closure.
    // /// # Behavior
    // /// In `origin` closure make sure to call the original `ResizeBuffers`.
    // pub fn resize_buffers(
    //     &mut self,
    //     gpu: &Gpu,
    //     original: impl FnOnce() -> d3d12::Result<()>,
    // ) -> Result<(), RenderError> {
    //     drop(self.render_view.take());
    //     let result = original();
    //     let backbuffer: d3d12::Texture2D = gpu.swapchain.lock().get_buffer();
    //     self.render_view = Some(gpu.create_render_target_view(&backbuffer, None)?);
    //     Ok(result?)
    // }

    pub fn textures(&self) -> &TextureAllocator {
        &self.tex_alloc
    }

    pub fn textures_mut(&mut self) -> &mut TextureAllocator {
        &mut self.tex_alloc
    }
}

impl D3D12Renderer {
    fn set_sampler_state(
        &self,
        cmd: &d3d12::GraphicsCommandList,
        filter: egui::TextureFilter,
    ) -> Result<(), RenderError> {
        // cmd.pixel_set_samplers(
        //     0,
        //     &[Some(match filter {
        //         egui::TextureFilter::Linear => &self.samplers[0],
        //         egui::TextureFilter::Nearest => &self.samplers[1],
        //     })],
        // );
        Ok(())
    }
}
