use std::{mem::transmute, ops::Range};

use bitflags::bitflags;
use bon::Builder;
use static_assertions::assert_eq_size;
use windows::Win32::Graphics::Direct3D12::*;

use crate::{
    pix::{pix3_begin_event_blob, pix_color},
    verify_ffi_type, CpuDescriptorHandle, DescriptorHeap, Format, GpuDescriptorHandle,
    GpuVirtualAddress, PipelineState, PrimitiveTopology, QueryHeap, QueryType, Resource,
    ResourceBarrier, Result, RootSignature, TextureCopyLocation,
};

#[repr(transparent)]
#[derive(Clone)]
pub struct GraphicsCommandList(pub(crate) ID3D12GraphicsCommandList);

#[profiling::all_functions]
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

    pub fn execute_bundle(&self, command_list: &Self) {
        unsafe {
            self.0.ExecuteBundle(&command_list.0);
        }
    }

    pub fn begin_event_raw(&self, metadata: EventMetadata, data: &[u8]) {
        unsafe {
            self.0.BeginEvent(
                metadata as u32,
                Some(data.as_ptr().cast()),
                data.len() as u32,
            );
        }
    }

    pub fn begin_event_str(&self, name: impl AsRef<str>) {
        self.begin_event_raw(EventMetadata::Ansi, name.as_ref().as_bytes());
    }

    pub fn end_event(&self) {
        unsafe {
            self.0.EndEvent();
        }
    }

    /// Creates a new event scope with the given name. The event will automatically end when the returned RAII guard is dropped.
    #[must_use]
    pub fn event_scope(&self, name: impl AsRef<str>, (r, g, b): (u8, u8, u8)) -> EventGuard {
        #[cfg(feature = "pix")]
        self.begin_event_raw(
            EventMetadata::Pix3Blob,
            &pix3_begin_event_blob(pix_color(r, g, b), name.as_ref()),
        );
        EventGuard {
            #[cfg(feature = "pix")]
            this: self.clone(),
        }
    }

    pub fn begin_query(&self, query_heap: &QueryHeap, query_type: QueryType, index: u32) {
        unsafe {
            self.0.BeginQuery(&query_heap.0, query_type.into(), index);
        }
    }

    pub fn end_query(&self, query_heap: &QueryHeap, query_type: QueryType, index: u32) {
        unsafe {
            self.0.EndQuery(&query_heap.0, query_type.into(), index);
        }
    }

    pub fn resolve_query_data(
        &self,
        query_heap: &QueryHeap,
        query_type: QueryType,
        start_index: u32,
        num_queries: u32,
        destination_buffer: &Resource,
        aligned_destination_buffer_offset: u64,
    ) {
        unsafe {
            self.0.ResolveQueryData(
                &query_heap.0,
                query_type.into(),
                start_index,
                num_queries,
                &destination_buffer.0,
                aligned_destination_buffer_offset,
            );
        }
    }

    pub fn resource_barriers(&self, barriers: &[ResourceBarrier<'_>]) {
        unsafe {
            self.0.ResourceBarrier(
                transmute::<&[ResourceBarrier<'_>], &[D3D12_RESOURCE_BARRIER]>(barriers),
            );
        }
    }

    pub fn copy_texture_region(
        &self,
        src: &TextureCopyLocation<'_>,
        src_box: Option<Box>,
        dst: &TextureCopyLocation<'_>,
        (dstx, dsty, dstz): (u32, u32, u32),
    ) {
        unsafe {
            self.0.CopyTextureRegion(
                &dst.0,
                dstx,
                dsty,
                dstz,
                &src.0,
                src_box.as_ref().map(|b| b.as_ffi()),
            );
        }
    }

    pub fn copy_resource(&self, src: &Resource, dst: &Resource) {
        unsafe {
            self.0.CopyResource(&dst.0, &src.0);
        }
    }

    pub fn set_pipeline_state(&self, pipeline_state: &PipelineState) {
        unsafe {
            self.0.SetPipelineState(&pipeline_state.0);
        }
    }

    pub fn set_root_signature(&self, root_signature: &RootSignature) {
        unsafe {
            self.0.SetGraphicsRootSignature(&root_signature.0);
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

    pub fn set_graphics_root_constant_buffer_view(
        &self,
        root_parameter_index: u32,
        buffer_location: GpuVirtualAddress,
    ) {
        if buffer_location == GpuVirtualAddress::NULL {
            tracing::error!("Binding NULL to b{root_parameter_index} is undefined behavior! ({buffer_location:?})");
        }
        unsafe {
            self.0
                .SetGraphicsRootConstantBufferView(root_parameter_index, buffer_location.0);
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

    pub fn ia_set_primitive_topology(&self, topology: PrimitiveTopology) {
        unsafe {
            self.0.IASetPrimitiveTopology(topology.into());
        }
    }

    pub fn ia_set_index_buffer(
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

    pub fn ia_set_vertex_buffers(&self, start_slot: u32, views: &[VertexBufferView]) {
        unsafe {
            self.0.IASetVertexBuffers(
                start_slot,
                Some(transmute::<&[VertexBufferView], &[D3D12_VERTEX_BUFFER_VIEW]>(views)),
            );
        }
    }

    pub fn om_set_render_targets(
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
verify_ffi_type!(Rect, windows::Win32::Foundation::RECT);

#[repr(C)]
#[derive(Clone, Debug, Builder)]
pub struct Box {
    pub left: u32,
    pub top: u32,
    pub front: u32,
    pub right: u32,
    pub bottom: u32,
    pub back: u32,
}
verify_ffi_type!(Box, D3D12_BOX);

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
#[derive(Clone, Debug)]
pub struct VertexBufferView {
    pub buffer_location: GpuVirtualAddress,
    pub size_in_bytes: u32,
    pub stride_in_bytes: u32,
}
assert_eq_size!(VertexBufferView, D3D12_VERTEX_BUFFER_VIEW);

impl VertexBufferView {
    pub const fn new(
        buffer_location: GpuVirtualAddress,
        size_in_bytes: u32,
        stride_in_bytes: u32,
    ) -> Self {
        Self {
            buffer_location,
            size_in_bytes,
            stride_in_bytes,
        }
    }
}

pub struct EventGuard {
    #[cfg(feature = "pix")]
    this: GraphicsCommandList,
}

impl Drop for EventGuard {
    fn drop(&mut self) {
        #[cfg(feature = "pix")]
        self.this.end_event();
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum EventMetadata {
    Unicode = 0,
    Ansi = 1,
    Pix3Blob = 2,
}
