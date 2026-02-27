use std::{mem::size_of, sync::Arc};

use d3d12::{
    DescriptorRange, ElementOffset, Format, GraphicsPipelineStateDesc, RootSignatureBuilder,
    RootSignatureFlags,
};
use deimos_render::gpu::{buffer::DynamicBuffer, Gpu};
use egui::{epaint::Primitive, Context};

use crate::{
    mesh::{create_index_buffer, create_vertex_buffer, GpuMesh, GpuVertex},
    texture::TextureAllocator,
    RenderError,
};

pub struct D3D12Renderer {
    tex_alloc: TextureAllocator,
    pipeline: d3d12::PipelineState,
    buffers: [Vec<(DynamicBuffer, DynamicBuffer)>; Gpu::FRAMES_IN_FLIGHT],
    root_signature: d3d12::RootSignature,
}

impl D3D12Renderer {
    /// Create a new directx11 renderer from a swapchain
    pub fn new(gpu: &Arc<Gpu>) -> Result<Self, RenderError> {
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
            .with_sampler(d3d12::StaticSamplerDesc::builder(0, 0).build())
            .with_param(
                d3d12::RootParameter::DescriptorTable(&[DescriptorRange {
                    base_shader_register: 0,
                    register_space: 0,
                    num_descriptors: 1,
                    range_type: d3d12::DescriptorRangeType::Srv,
                    offset_in_descriptors_from_table_start: 0,
                }]),
                d3d12::ShaderVisibility::Pixel,
            )
            .serialize()?;

        let blend_desc = d3d12::BlendDesc::from_single_target(
            d3d12::RenderTargetBlendDesc::builder()
                .blend_enable(true)
                .src_blend(d3d12::Blend::SrcAlpha)
                .dest_blend(d3d12::Blend::InvSrcAlpha)
                .blend_op(d3d12::BlendOp::Add)
                .src_blend_alpha(d3d12::Blend::One)
                .dest_blend_alpha(d3d12::Blend::InvSrcAlpha)
                .blend_op_alpha(d3d12::BlendOp::Add)
                .render_target_write_mask(15)
                .logic_op(d3d12::LogicOp::Noop)
                .logic_op_enable(false)
                .build(),
        );

        let root_signature = gpu.create_root_signature(&root_signature_raw)?;
        let pipeline = gpu.create_graphics_pipeline_state(
            &GraphicsPipelineStateDesc::new(&root_signature)
                .with_vs(include_bytes!("shader/vertex.dxil"))
                .with_ps(include_bytes!("shader/pixel.dxil"))
                .with_rtv_formats(&[Format::R8g8b8a8Unorm])
                .with_input_layout(&input_layout)
                .with_blend_state(blend_desc),
        )?;

        Ok(Self {
            tex_alloc: TextureAllocator::new(gpu)?,
            pipeline,
            root_signature,
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

        if !output.textures_delta.is_empty() {
            self.tex_alloc.process_deltas(gpu, &output.textures_delta)?;
        }

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
        cmd.set_root_signature(&self.root_signature);
        cmd.set_pipeline_state(&self.pipeline);
        cmd.set_descriptor_heaps(std::slice::from_ref(
            &self.tex_alloc.descriptor_heap_alloc.descriptor_heap,
        ));

        self.buffers[gpu.frame_index() % Gpu::FRAMES_IN_FLIGHT].clear();

        cmd.ia_set_primitive_topology(d3d12::PrimitiveTopology::TriangleList);
        for mesh in primitives {
            let vtx = create_vertex_buffer(gpu, &mesh)?;
            let idx = create_index_buffer(gpu, &mesh)?;

            cmd.ia_set_vertex_buffers(0, &[vtx.vb_view(size_of::<GpuVertex>() as u32)]);
            cmd.ia_set_index_buffer(
                idx.gpu_virtual_address(),
                idx.size() as u32,
                Format::R32Uint,
            );
            self.buffers[gpu.frame_index() % Gpu::FRAMES_IN_FLIGHT].push((vtx, idx));

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

            let texture = self.tex_alloc.get_by_id(mesh.texture_id);
            if let Some((texture, texture_filter, _texture_uses_alpha)) = &texture {
                self.set_sampler_state(cmd, texture_filter.unwrap_or(egui::TextureFilter::Linear))?;
                // use_alpha = *texture_uses_alpha;
                // cmd.pixel_set_shader_resources(0, &[Some(texture)]);
                cmd.set_graphics_root_descriptor_table(0, *texture);
            }

            cmd.draw_indexed_instanced(0..mesh.indices.len() as _, 0..1, 0);

            //     if texture.is_some() {
            //         self.set_sampler_state(cmd, egui::TextureFilter::Linear)?;
            //     }
        }

        // // self.backup.restore(ctx);
        self.textures_mut().clear_temporaries();

        Ok(output)
    }

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
        _cmd: &d3d12::GraphicsCommandList,
        _filter: egui::TextureFilter,
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
