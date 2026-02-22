use std::{mem::size_of, slice::from_raw_parts_mut, sync::Arc};

use d3d12::{
    ext::GpuFence, CpuDescriptorHandle, DescriptorHeapType, Format, GpuDescriptorHandle,
    ShaderResourceViewDesc, TextureCopyLocation,
};
use deimos_render::gpu::{command_list::CommandList, Gpu};
use egui::{epaint::ahash::HashMap, Color32, ColorImage, ImageData, TextureId, TexturesDelta};
use gpu_allocator::{
    d3d12::{ResourceCategory, ResourceCreateDesc, ResourceStateOrBarrierLayout},
    MemoryLocation,
};

use crate::RenderError;

struct ManagedTexture {
    cpu_handle: CpuDescriptorHandle,
    gpu_handle: GpuDescriptorHandle,

    // resource: d3d12::ShaderResourceView,
    // texture: d3d12::Texture2D,
    pixels: Vec<Color32>,
    width: usize,
    resource: gpu_allocator::d3d12::Resource,
}

pub struct TextureAllocator {
    allocated: HashMap<TextureId, ManagedTexture>,
    pub(crate) descriptor_heap_alloc: DescriptorHeapAllocator,

    upload_command_list: CommandList,
    upload_fence: GpuFence,
    pending_uploads: Vec<gpu_allocator::d3d12::Resource>,

    // allocated_unmanaged: HashMap<TextureId, (TextureView, Option<egui::TextureFilter>, bool)>,
    unmanaged_free_handles: Vec<TextureId>,
    unmanaged_index: u64,
    unmanaged_temporary_index: u64,

    gpu: Arc<Gpu>,
}

impl TextureAllocator {
    pub fn new(gpu: &Arc<Gpu>) -> Result<Self, RenderError> {
        let descriptor_heap =
            DescriptorHeapAllocator::new(gpu, DescriptorHeapType::CbvSrvUav, 2048)?;

        Ok(TextureAllocator {
            allocated: HashMap::default(),
            upload_command_list: CommandList::new(gpu).unwrap(),
            upload_fence: GpuFence::new(gpu)?,
            pending_uploads: Vec::new(),

            descriptor_heap_alloc: descriptor_heap,
            unmanaged_free_handles: Vec::new(),
            unmanaged_index: 0,
            unmanaged_temporary_index: 0,

            gpu: gpu.clone(),
        })
    }

    pub fn process_deltas(
        &mut self,
        gpu: &Arc<Gpu>,
        delta: &TexturesDelta,
    ) -> Result<(), RenderError> {
        self.upload_fence.wait()?;

        self.upload_command_list
            .begin()
            .expect("begin upload_command_list");
        println!("Processing texture deltas");
        for (tid, delta) in &delta.set {
            if delta.is_whole() {
                self.allocate_new(gpu, *tid, &delta.image)?;
            } else {
                let _did_update =
                    self.update_partial(gpu, *tid, &delta.image, delta.pos.unwrap())?;
            }
        }

        for tid in &delta.free {
            self.free(*tid);
        }
        println!("Finished processing texture deltas");
        self.upload_command_list
            .end()
            .expect("end upload_command_list");

        gpu.queue
            .execute_command_lists(std::slice::from_ref(&self.upload_command_list));
        self.upload_fence.signal(&gpu.queue);

        for upload_buffer in self.pending_uploads.drain(..) {
            gpu.allocator
                .lock()
                .free_resource(upload_buffer)
                .expect("Failed to free upload buffer");
        }

        Ok(())
    }

    pub fn get_by_id(
        &self,
        tid: TextureId,
    ) -> Option<(
        d3d12::GpuDescriptorHandle,
        Option<egui::TextureFilter>,
        bool,
    )> {
        self.allocated.get(&tid).map(|t| (t.gpu_handle, None, true))
        //     .or_else(|| self.allocated_unmanaged.get(&tid).cloned())
    }

    // pub fn allocate_dx(
    //     &mut self,
    //     srv: TextureView,
    //     filter: Option<egui::TextureFilter>,
    // ) -> TextureId {
    //     todo!()
    //     // let tid = if let Some(t) = self.unmanaged_free_handles.pop() {
    //     //     t
    //     // } else {
    //     //     self.unmanaged_index += 1;
    //     //     TextureId::User((1 << 60) + self.unmanaged_index)
    //     // };
    //     // self.allocated_unmanaged.insert(tid, (srv, filter, true));
    //     // tid
    // }

    // /// Allocate a temporary texture that will be freed after the current frame finishes painting
    // pub fn allocate_dx_temporary(
    //     &mut self,
    //     srv: TextureView,
    //     filter: Option<egui::TextureFilter>,
    //     alpha: bool,
    // ) -> TextureId {
    //     todo!()
    //     // self.unmanaged_temporary_index += 1;
    //     // let tid = TextureId::User((1 << 63) + self.unmanaged_temporary_index);

    //     // self.allocated_unmanaged.insert(tid, (srv, filter, alpha));
    //     // tid
    // }

    pub fn clear_temporaries(&mut self) {
        // self.unmanaged_temporary_index = 0;
        // self.allocated_unmanaged.retain(|id, _| match id {
        //     TextureId::Managed(_) => true,
        //     TextureId::User(id) => *id < (1 << 63),
        // });
    }

    pub fn set_filter(&mut self, tid: TextureId, filter: Option<egui::TextureFilter>) {
        // if let Some((_, f, _)) = self.allocated_unmanaged.get_mut(&tid) {
        //     *f = filter;
        // }
    }

    pub fn free(&mut self, tid: TextureId) -> bool {
        if let Some(removed) = self.allocated.remove(&tid)
        // .map(|_| ())
        // .or_else(|| {
        //     let s = self.allocated_unmanaged.remove(&tid).map(|_| ());
        //     if s.is_some() {
        //         self.unmanaged_free_handles.push(tid);
        //     }
        //     s
        // })
        {
            self.descriptor_heap_alloc.free(removed.cpu_handle);
            self.gpu.allocator.lock().free_resource(removed.resource);
            true
        } else {
            false
        }
    }
}

impl Drop for TextureAllocator {
    fn drop(&mut self) {
        let ids = self.allocated.keys().copied().collect::<Vec<_>>();
        for tid in ids {
            self.free(tid);
        }
    }
}

impl TextureAllocator {
    fn allocate_new(
        &mut self,
        gpu: &Gpu,
        tid: TextureId,
        image: &ImageData,
    ) -> Result<(), RenderError> {
        let tex = self.allocate_texture(gpu, image)?;
        self.allocated.insert(tid, tex);
        Ok(())
    }

    fn update_partial(
        &mut self,
        gpu: &Arc<Gpu>,
        tid: TextureId,
        image: &ImageData,
        [nx, ny]: [usize; 2],
    ) -> Result<bool, RenderError> {
        Ok(false)
        // todo!()
        // if let Some(old) = self.allocated.get_mut(&tid) {
        //     let subr = ctx.map(&old.texture, 0, d3d12::MapType::WriteDiscard, false)?;

        //     match image {
        //         ImageData::Color(f) => unsafe {
        //             let data: &mut [Color32] =
        //                 from_raw_parts_mut(subr.data as *mut Color32, old.pixels.len());
        //             data.as_mut_ptr()
        //                 .copy_from_nonoverlapping(old.pixels.as_ptr(), old.pixels.len());

        //             let new: Vec<Color32> = f.pixels.to_vec();

        //             for y in 0..f.height() {
        //                 for x in 0..f.width() {
        //                     let whole = (ny + y) * old.width + nx + x;
        //                     let frac = y * f.width() + x;
        //                     old.pixels[whole] = new[frac];
        //                     data[whole] = new[frac];
        //                 }
        //             }
        //         },
        //     }

        //     Ok(true)
        // } else {
        //     Ok(false)
        // }
    }

    fn allocate_texture(
        &mut self,
        gpu: &Gpu,
        image: &ImageData,
    ) -> Result<ManagedTexture, RenderError> {
        let ImageData::Color(image) = image;
        let pixels = image.pixels.clone();
        let Some((cpu_handle, gpu_handle)) = self.descriptor_heap_alloc.allocate() else {
            return Err(RenderError::General("Texture descriptor heap out of slots"));
        };

        // let pixels = match image {
        //     ImageData::Color(c) => c.pixels.clone(),
        // };

        // let data = d3d12_SUBRESOURCE_DATA {
        //     pSysMem: pixels.as_ptr() as _,
        //     SysMemPitch: (image.width() * size_of::<Color32>()) as u32,
        //     SysMemSlicePitch: 0,
        // };

        // let texture = dev.create_texture2d(&desc, Some(&[data]))?;

        // let resource = dev.create_shader_resource_view(
        //     &texture,
        //     &d3d12::ShaderResourceViewDesc::builder()
        //         .format(dxgi::Format::R8g8b8a8Unorm)
        //         .view_dimension(d3d12::SrvDimension::Texture2D {
        //             most_detailed_mip: 0,
        //             mip_levels: desc.mip_levels,
        //         })
        //         .build(),
        // )?;

        let tex_desc = d3d12::ResourceDesc::builder(d3d12::ResourceDimension::Texture2D)
            .alignment(0)
            .width(image.width() as _)
            .height(image.height() as _)
            .mip_levels(1)
            .format(Format::R8g8b8a8Unorm)
            .build();

        let tex = gpu
            .allocator
            .lock()
            .create_resource(&ResourceCreateDesc {
                name: "egui texture",
                memory_location: MemoryLocation::GpuOnly,
                resource_category: ResourceCategory::OtherTexture,
                resource_desc: unsafe { &*tex_desc.as_ffi() },
                castable_formats: &[],
                clear_value: None,
                initial_state_or_layout: ResourceStateOrBarrierLayout::ResourceState(
                    d3d12::D3D12_RESOURCE_STATE_COPY_DEST,
                ),
                resource_type: &gpu_allocator::d3d12::ResourceType::Placed,
            })
            .expect("Failed to create texture resource");
        let d = unsafe { tex.resource().GetDesc() };
        println!(
            "Created: Dim={:?} W={} H={} DepthOrArraySize={} Mips={} Format={:?} SampleCount={} Layout={:?} Flags={:?}",
            d.Dimension, d.Width, d.Height, d.DepthOrArraySize, d.MipLevels, d.Format, d.SampleDesc.Count, d.Layout, d.Flags
        );

        let footprint = gpu.get_copyable_footprints(&tex_desc, 0, 1, 0)?;

        let upload_desc = d3d12::ResourceDesc::builder(d3d12::ResourceDimension::Buffer)
            .width(footprint.total_bytes)
            .height(1)
            .format(Format::Unknown)
            .layout(d3d12::TextureLayout::RowMajor)
            .build();

        let upload_buffer = gpu
            .allocator
            .lock()
            .create_resource(&ResourceCreateDesc {
                name: "egui texture upload buffer",
                memory_location: MemoryLocation::CpuToGpu,
                resource_category: ResourceCategory::Buffer,
                resource_desc: unsafe { &*upload_desc.as_ffi() },
                castable_formats: &[],
                clear_value: None,
                initial_state_or_layout: ResourceStateOrBarrierLayout::ResourceState(
                    d3d12::D3D12_RESOURCE_STATE_GENERIC_READ,
                ),
                resource_type: &gpu_allocator::d3d12::ResourceType::Placed,
            })
            .expect("Failed to create texture upload buffer");

        let mut mapped_ptr = std::ptr::null_mut();
        unsafe {
            upload_buffer
                .resource()
                .Map(0, None, Some(&mut mapped_ptr))
                .expect("Failed to map upload buffer");
        }
        let data = bytemuck::cast_slice::<Color32, u8>(&pixels);
        unsafe {
            std::ptr::copy_nonoverlapping(data.as_ptr(), mapped_ptr as *mut u8, data.len());
        }
        unsafe {
            upload_buffer.resource().Unmap(0, None);
        }

        let src = TextureCopyLocation::placed_footprint(
            upload_buffer.resource().as_ref(),
            footprint.layouts[0],
        );

        let dst = TextureCopyLocation::subresource(tex.resource().as_ref(), 0);

        self.upload_command_list
            .copy_texture_region(&src, None, &dst, (0, 0, 0));

        self.pending_uploads.push(upload_buffer);

        gpu.create_shader_resource_view(
            Some(tex.resource().as_ref()),
            Some(&ShaderResourceViewDesc::texture_2d(
                Format::R8g8b8a8Unorm,
                0,
                1,
                0.0,
                0,
            )),
            cpu_handle,
        );

        println!(
            "Created SRV for {}x{} texture",
            image.width(),
            image.height()
        );

        Ok(ManagedTexture {
            resource: tex,
            width: image.width(),
            pixels,
            cpu_handle,
            gpu_handle,
        })
    }
}

pub struct DescriptorHeapAllocator {
    pub descriptor_heap: d3d12::DescriptorHeap,
    heap_type: DescriptorHeapType,
    size: usize,
    free_list: Vec<usize>,

    increment_size: u32,
    cpu_handle_base: CpuDescriptorHandle,
    gpu_handle_base: GpuDescriptorHandle,
}

impl DescriptorHeapAllocator {
    pub fn new(
        device: &d3d12::Device,
        heap_type: DescriptorHeapType,
        size: usize,
    ) -> d3d12::Result<Self> {
        let descriptor_heap = device.create_descriptor_heap(heap_type, size as u32, true, 0)?;

        Ok(DescriptorHeapAllocator {
            cpu_handle_base: descriptor_heap.cpu_descriptor_handle_for_heap_start(),
            gpu_handle_base: descriptor_heap.gpu_descriptor_handle_for_heap_start(),
            increment_size: device.descriptor_handle_increment_size(heap_type),
            descriptor_heap,
            heap_type,
            size,
            free_list: (0..size).collect(),
        })
    }

    fn handles_for_index(&self, offset: usize) -> (CpuDescriptorHandle, GpuDescriptorHandle) {
        let cpu = self.cpu_handle_base.offset(offset, self.increment_size);
        let gpu = self.gpu_handle_base.offset(offset, self.increment_size);
        (cpu, gpu)
    }

    fn index_for_handle(&self, handle: CpuDescriptorHandle) -> usize {
        handle.index(self.cpu_handle_base, self.increment_size)
    }

    pub fn allocate(&mut self) -> Option<(CpuDescriptorHandle, GpuDescriptorHandle)> {
        self.free_list
            .pop()
            .map(|index| self.handles_for_index(index))
    }

    pub fn free(&mut self, handle: CpuDescriptorHandle) {
        let index = self.index_for_handle(handle);
        self.free_list.push(index);
    }
}
