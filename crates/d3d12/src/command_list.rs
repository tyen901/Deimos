use std::{mem::transmute, ops::Range};

use bitflags::bitflags;
use bon::Builder;
use static_assertions::assert_eq_size;
use windows::Win32::Graphics::Direct3D12::*;

use crate::{
    CpuDescriptorHandle, DescriptorHeap, Format, GpuDescriptorHandle, GpuVirtualAddress,
    PipelineState, PrimitiveTopology, Result,
};

#[repr(transparent)]
#[derive(Clone)]
pub struct GraphicsCommandList(pub(crate) ID3D12GraphicsCommandList);

impl GraphicsCommandList {
    pub fn close(&self) -> Result<()> {
        unsafe {
            self.0.Close()?;
        }

        Ok(())
    }

    pub fn reset(
        &self,
        allocator: &CommandAllocator,
        initial_state: Option<&PipelineState>,
    ) -> Result<()> {
        unsafe {
            self.0.Reset(&allocator.0, initial_state.map(|p| &p.0))?;
        }

        Ok(())
    }

    pub fn set_pipeline_state(&self, pipeline_state: &PipelineState) {
        unsafe {
            self.0.SetPipelineState(&pipeline_state.0);
        }
    }

    pub fn set_graphics_root_descriptor_table(
        &self,
        root_parameter_index: u32,
        base_descriptor: GpuDescriptorHandle,
    ) {
        unsafe {
            self.0
                .SetGraphicsRootDescriptorTable(root_parameter_index, base_descriptor.into());
        }
    }

    pub fn set_descriptor_heaps(&self, heaps: &[DescriptorHeap]) {
        unsafe {
            self.0.SetDescriptorHeaps(transmute::<
                &[DescriptorHeap],
                &[Option<ID3D12DescriptorHeap>],
            >(heaps));
        }
    }

    pub fn draw_instanced(&self, vertices: Range<u32>, instances: Range<u32>) {
        unsafe {
            self.0.DrawInstanced(
                vertices.len() as u32,
                instances.len() as u32,
                vertices.start,
                instances.start,
            );
        }
    }

    pub fn draw_indexed_instanced(
        &self,
        indices: Range<u32>,
        instances: Range<u32>,
        base_vertex_location: i32,
    ) {
        unsafe {
            self.0.DrawIndexedInstanced(
                indices.len() as u32,
                instances.len() as u32,
                indices.start,
                base_vertex_location,
                instances.start,
            );
        }
    }

    pub fn set_scissor_rects(&self, rects: &[Rect]) {
        unsafe {
            self.0
                .RSSetScissorRects(transmute::<&[Rect], &[windows::Win32::Foundation::RECT]>(
                    rects,
                ));
        }
    }

    pub fn set_viewports(&self, viewports: &[Viewport]) {
        unsafe {
            self.0
                .RSSetViewports(transmute::<&[Viewport], &[D3D12_VIEWPORT]>(viewports));
        }
    }

    pub fn input_assembler_set_primitive_topology(&self, topology: PrimitiveTopology) {
        unsafe {
            self.0.IASetPrimitiveTopology(topology.into());
        }
    }

    pub fn input_assembler_set_index_buffer(
        &self,
        buffer_location: GpuVirtualAddress,
        size_in_bytes: u32,
        format: Format,
    ) {
        unsafe {
            self.0.IASetIndexBuffer(Some(&D3D12_INDEX_BUFFER_VIEW {
                BufferLocation: buffer_location.0,
                SizeInBytes: size_in_bytes,
                Format: format.into(),
            }));
        }
    }

    pub fn input_assembler_set_vertex_buffers(&self, start_slot: u32, views: &[VertexBufferView]) {
        unsafe {
            self.0.IASetVertexBuffers(
                start_slot,
                Some(transmute::<&[VertexBufferView], &[D3D12_VERTEX_BUFFER_VIEW]>(views)),
            );
        }
    }

    pub fn output_merger_set_render_targets(
        &self,
        render_target_descriptors: &[CpuDescriptorHandle],
        rts_single_handle_to_descriptor_range: bool,
        depth_stencil_descriptor: Option<CpuDescriptorHandle>,
    ) {
        let depth_stencil_descriptor = depth_stencil_descriptor.map(|d| d.into());
        unsafe {
            self.0.OMSetRenderTargets(
                render_target_descriptors.len() as u32,
                Some(render_target_descriptors.as_ptr().cast()),
                rts_single_handle_to_descriptor_range,
                depth_stencil_descriptor.as_ref().map(|d| d as *const _),
            );
        }
    }

    pub fn clear_render_target_view(&self, rtv: CpuDescriptorHandle, color: &[f32; 4]) {
        unsafe {
            self.0.ClearRenderTargetView(rtv.into(), color, None);
        }
    }

    pub fn clear_depth_stencil_view(
        &self,
        dsv: CpuDescriptorHandle,
        flags: ClearFlags,
        depth: f32,
        stencil: u8,
    ) {
        unsafe {
            self.0.ClearDepthStencilView(
                dsv.into(),
                D3D12_CLEAR_FLAGS(flags.bits()),
                depth,
                stencil,
                None,
            );
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Builder)]
pub struct Viewport {
    #[builder(default = 0.0)]
    pub top_left_x: f32,
    #[builder(default = 0.0)]
    pub top_left_y: f32,
    pub width: f32,
    pub height: f32,
    #[builder(default = 0.0)]
    pub min_depth: f32,
    #[builder(default = 1.0)]
    pub max_depth: f32,
}
assert_eq_size!(Viewport, D3D12_VIEWPORT);

impl Viewport {
    /// Scales the viewport to fit the given mip level (eg. 0 is full size, 1 is half size, etc.)
    ///
    /// Only the width and height are scaled, the top left x and y are not.
    pub fn scale_to_mip(&self, mip: u32) -> Self {
        let scale = (1 << mip) as f32;
        Self {
            top_left_x: self.top_left_x,
            top_left_y: self.top_left_y,
            width: self.width / scale,
            height: self.height / scale,
            min_depth: self.min_depth,
            max_depth: self.max_depth,
        }
    }
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            top_left_x: 0.0,
            top_left_y: 0.0,
            width: 0.0,
            height: 0.0,
            min_depth: 0.0,
            max_depth: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Builder)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
assert_eq_size!(Rect, windows::Win32::Foundation::RECT);

#[repr(transparent)]
#[derive(Clone)]
pub struct CommandAllocator(pub(crate) ID3D12CommandAllocator);

impl CommandAllocator {
    pub fn reset(&self) -> Result<()> {
        Ok(unsafe { self.0.Reset() }?)
    }
}

bitflags! {
    pub struct ClearFlags: i32 {
        const DEPTH = D3D12_CLEAR_FLAG_DEPTH.0;
        const STENCIL = D3D12_CLEAR_FLAG_STENCIL.0;
    }
}

#[repr(C)]
#[derive(Clone, Debug, Builder)]
pub struct VertexBufferView {
    pub buffer_location: GpuVirtualAddress,
    pub size_in_bytes: u32,
    pub stride_in_bytes: u32,
}
assert_eq_size!(VertexBufferView, D3D12_VERTEX_BUFFER_VIEW);
