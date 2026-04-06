use std::sync::Arc;

use anyhow::Context;
use d3d12::{DeviceChild, ResourceFlags};

use crate::gpu::{
    Gpu,
    alloc::{descriptors::ResourceView, resource::OwnedResource},
};

pub struct RenderTarget {
    pub resource: OwnedResource,
    size: (u32, u32),
    rtv_heap: d3d12::DescriptorHeap,
    srv: ResourceView,
    gpu: Arc<Gpu>,

    name: String,
    format: d3d12::Format,
    view_format: d3d12::Format,
}

impl RenderTarget {
    pub fn new(
        gpu: &Arc<Gpu>,
        name: &str,
        format: d3d12::Format,
        view_format: d3d12::Format,
        (width, height): (u32, u32),
    ) -> anyhow::Result<Self> {
        let resource = gpu
            .allocate_resource(&gpu_allocator::d3d12::ResourceCreateDesc {
                name,
                memory_location: gpu_allocator::MemoryLocation::GpuOnly,
                resource_category: gpu_allocator::d3d12::ResourceCategory::RtvDsvTexture,
                resource_desc: d3d12::ResourceDesc::new(d3d12::ResourceDimension::Texture2D)
                    .width(width as u64)
                    .height(height)
                    .format(format)
                    .flags(ResourceFlags::ALLOW_RENDER_TARGET)
                    .as_ref(),
                castable_formats: &[],
                clear_value: None,
                initial_state_or_layout:
                    gpu_allocator::d3d12::ResourceStateOrBarrierLayout::ResourceState(
                        d3d12::D3D12_RESOURCE_STATE_RENDER_TARGET,
                    ),
                resource_type: &gpu_allocator::d3d12::ResourceType::Placed,
            })
            .context("allocating resource")?;

        resource.resource().set_debug_name(name);

        let rtv_heap = gpu.create_descriptor_heap(d3d12::DescriptorHeapType::Rtv, 1, false, 0)?;
        gpu.create_render_target_view(
            Some(resource.resource()),
            Some(
                &d3d12::RenderTargetViewDesc::builder()
                    .format(view_format)
                    .view_dimension(d3d12::RtvDimension::Texture2D {
                        mip_slice: 0,
                        plane_slice: 0,
                    })
                    .build(),
            ),
            rtv_heap.cpu_descriptor_handle_for_heap_start(),
        );

        let srv = gpu.resource_heap.lock().allocate_srv(
            name,
            resource.resource(),
            &d3d12::ShaderResourceViewDesc::texture_2d(view_format, 0, 1, 0.0, 0),
        );

        Ok(Self {
            gpu: gpu.clone(),
            resource,
            rtv_heap,
            srv,
            size: (width, height),

            name: name.to_string(),
            format,
            view_format,
        })
    }

    pub const fn output_format(&self) -> d3d12::Format {
        self.view_format
    }

    pub fn cpu_handle(&self) -> d3d12::CpuDescriptorHandle {
        self.rtv_heap.cpu_descriptor_handle_for_heap_start()
    }

    pub fn gpu_handle(&self) -> d3d12::GpuDescriptorHandle {
        self.rtv_heap.gpu_descriptor_handle_for_heap_start()
    }

    pub const fn srv(&self) -> ResourceView {
        self.srv
    }

    pub const fn resolution(&self) -> (u32, u32) {
        self.size
    }

    pub fn transition(
        &mut self,
        cmd: &d3d12::GraphicsCommandList,
        new_state: d3d12::ResourceStates,
    ) {
        self.resource.transition(cmd, new_state);
    }

    /// Resize the depth buffer.
    ///
    /// # Remarks
    /// - Won't resize if the new size is the same as the current size.
    /// - The old depth buffer is put into the GPU resource bin, meaning it won't actually be destroyed until it's no longer used
    pub fn resize(&mut self, (width, height): (u32, u32)) -> anyhow::Result<()> {
        if width == self.size.0 && height == self.size.1 {
            return Ok(());
        }

        let mut new_depth_buffer = Self::new(
            &self.gpu,
            &self.name,
            self.format,
            self.view_format,
            (width, height),
        )?;
        std::mem::swap(self, &mut new_depth_buffer);

        Ok(())
    }

    pub fn take(&mut self) -> anyhow::Result<Self> {
        let new = Self::new(
            &self.gpu,
            &self.name,
            self.format,
            self.view_format,
            self.size,
        )?;

        Ok(std::mem::replace(self, new))
    }
}

pub struct DepthBuffer {
    pub resource: OwnedResource,
    size: (u32, u32),
    dsv_heap: d3d12::DescriptorHeap,
    srv: ResourceView,
    gpu: Arc<Gpu>,

    current_state: d3d12::ResourceStates,
    output_format: d3d12::Format,
}

impl DepthBuffer {
    pub fn new(gpu: &Arc<Gpu>, (width, height): (u32, u32)) -> anyhow::Result<Self> {
        let resource = gpu.allocate_resource(&gpu_allocator::d3d12::ResourceCreateDesc {
            name: "depth_buffer",
            memory_location: gpu_allocator::MemoryLocation::GpuOnly,
            resource_category: gpu_allocator::d3d12::ResourceCategory::RtvDsvTexture,
            resource_desc: d3d12::ResourceDesc::new(d3d12::ResourceDimension::Texture2D)
                .width(width as u64)
                .height(height)
                .format(d3d12::Format::R32g8x24Typeless)
                .flags(ResourceFlags::ALLOW_DEPTH_STENCIL)
                .as_ref(),
            castable_formats: &[],
            clear_value: Some(&d3d12::D3D12_CLEAR_VALUE {
                Format: d3d12::Format::D32FloatS8x24Uint.into(),
                Anonymous: d3d12::D3D12_CLEAR_VALUE_0 {
                    DepthStencil: d3d12::D3D12_DEPTH_STENCIL_VALUE {
                        Depth: 0.0,
                        Stencil: 0,
                    },
                },
            }),
            initial_state_or_layout:
                gpu_allocator::d3d12::ResourceStateOrBarrierLayout::ResourceState(
                    d3d12::D3D12_RESOURCE_STATE_DEPTH_WRITE,
                ),
            resource_type: &gpu_allocator::d3d12::ResourceType::Placed,
        })?;
        resource.resource().set_debug_name("depth_buffer");

        let rtv_heap = gpu.create_descriptor_heap(d3d12::DescriptorHeapType::Dsv, 1, false, 0)?;
        gpu.create_depth_stencil_view(
            Some(resource.resource()),
            Some(&d3d12::DepthStencilViewDesc::texture_2d(
                d3d12::Format::D32FloatS8x24Uint,
            )),
            rtv_heap.cpu_descriptor_handle_for_heap_start(),
        );

        let srv = gpu.resource_heap.lock().allocate_srv(
            "depth",
            resource.resource(),
            &d3d12::ShaderResourceViewDesc::texture_2d(
                d3d12::Format::R32FloatX8x24Typeless,
                0,
                1,
                0.0,
                0,
            ),
        );

        Ok(Self {
            gpu: gpu.clone(),
            resource,
            dsv_heap: rtv_heap,
            srv,
            size: (width, height),
            output_format: d3d12::Format::D32FloatS8x24Uint,

            current_state: d3d12::ResourceStates::DEPTH_WRITE,
        })
    }

    pub const fn output_format(&self) -> d3d12::Format {
        self.output_format
    }

    pub fn cpu_handle(&self) -> d3d12::CpuDescriptorHandle {
        self.dsv_heap.cpu_descriptor_handle_for_heap_start()
    }

    pub fn gpu_handle(&self) -> d3d12::GpuDescriptorHandle {
        self.dsv_heap.gpu_descriptor_handle_for_heap_start()
    }

    pub fn transition(
        &mut self,
        cmd: &d3d12::GraphicsCommandList,
        new_state: d3d12::ResourceStates,
    ) {
        self.resource.transition(cmd, new_state);
    }

    pub const fn srv(&self) -> ResourceView {
        self.srv
    }

    /// Resize the depth buffer.
    ///
    /// # Remarks
    /// - Won't resize if the new size is the same as the current size.
    /// - The old depth buffer is put into the GPU resource bin, meaning it won't actually be destroyed until it's no longer used
    pub fn resize(&mut self, (width, height): (u32, u32)) -> anyhow::Result<()> {
        if width == self.size.0 && height == self.size.1 {
            return Ok(());
        }

        let mut new_depth_buffer = Self::new(&self.gpu, (width, height))?;
        std::mem::swap(self, &mut new_depth_buffer);

        Ok(())
    }
}
