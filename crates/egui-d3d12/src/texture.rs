use std::sync::Arc;

use d3d12::{
    CpuDescriptorHandle, DescriptorHeapType, DeviceChild, Format, GpuDescriptorHandle,
    ResourceBarrier, ResourceStates, ShaderResourceViewDesc, TextureCopyLocation,
};
use deimos_render::gpu::{alloc::resource::OwnedResource, Gpu};
use egui::{epaint::ahash::HashMap, Color32, ImageData, TextureId, TexturesDelta};
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
    height: usize,
    resource: OwnedResource,
}

pub struct TextureAllocator {
    allocated: HashMap<TextureId, ManagedTexture>,
    pub(crate) descriptor_heap_alloc: DescriptorHeapAllocator,

    // upload_command_list: CommandList,
    // upload_fence: GpuFenceWaiter,
    pending_uploads: Vec<OwnedResource>,

    // allocated_unmanaged: HashMap<TextureId, (TextureView, Option<egui::TextureFilter>, bool)>,
    // unmanaged_free_handles: Vec<TextureId>,
    // unmanaged_index: u64,
    // unmanaged_temporary_index: u64,
    gpu: Arc<Gpu>,
}

impl TextureAllocator {
    pub fn new(gpu: &Arc<Gpu>) -> Result<Self, RenderError> {
        let descriptor_heap =
            DescriptorHeapAllocator::new(gpu, DescriptorHeapType::CbvSrvUav, 2048)?;

        Ok(TextureAllocator {
            allocated: HashMap::default(),
            // upload_command_list: CommandList::new(gpu).unwrap(),
            // upload_fence: GpuFenceWaiter::new(gpu)?,
            pending_uploads: Vec::new(),
            descriptor_heap_alloc: descriptor_heap,
            // unmanaged_free_handles: Vec::new(),
            // unmanaged_index: 0,
            // unmanaged_temporary_index: 0,
            gpu: gpu.clone(),
        })
    }

    pub fn process_deltas(
        &mut self,
        gpu: &Arc<Gpu>,
        delta: &TexturesDelta,
    ) -> Result<(), RenderError> {
        gpu.immediate_pool
            .scope_immediate(|cmd| {
                for (tid, delta) in &delta.set {
                    if delta.is_whole() {
                        self.allocate_new(cmd, *tid, &delta.image)?;
                    } else {
                        let _did_update =
                            self.update_partial(cmd, *tid, &delta.image, delta.pos.unwrap())?;
                    }
                }

                Ok(())
            })
            .expect("process_deltas scope_immediate");

        // Deallocate pending upload buffers
        self.pending_uploads.clear();

        for tid in &delta.free {
            self.free(*tid);
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

    pub fn set_filter(&mut self, _tid: TextureId, _filter: Option<egui::TextureFilter>) {
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
        cmd: &d3d12::GraphicsCommandList,
        tid: TextureId,
        image: &ImageData,
    ) -> Result<(), RenderError> {
        let tex = self.allocate_texture(cmd, image)?;
        tex.resource
            .resource()
            .set_debug_name(format!("egui {tid:?}"));
        self.allocated.insert(tid, tex);
        Ok(())
    }

    fn update_partial(
        &mut self,
        cmd: &d3d12::GraphicsCommandList,
        tid: TextureId,
        image: &ImageData,
        [nx, ny]: [usize; 2],
    ) -> Result<bool, RenderError> {
        // Ok(false)
        // todo!()
        if let Some(mut tex) = self.allocated.remove(&tid) {
            //     let subr = ctx.map(&old.texture, 0, d3d12::MapType::WriteDiscard, false)?;

            match image {
                ImageData::Color(f) => {
                    // let data: &mut [Color32] =
                    //     from_raw_parts_mut(subr.data as *mut Color32, old.pixels.len());
                    // data.as_mut_ptr()
                    //     .copy_from_nonoverlapping(old.pixels.as_ptr(), old.pixels.len());

                    let new: Vec<Color32> = f.pixels.to_vec();

                    for y in 0..f.height() {
                        for x in 0..f.width() {
                            let whole = (ny + y) * tex.width + nx + x;
                            let frac = y * f.width() + x;
                            tex.pixels[whole] = new[frac];
                            // data[whole] = new[frac];
                        }
                    }
                }
            }

            self.upload_texture(cmd, &tex)?;

            self.allocated.insert(tid, tex);

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn allocate_texture(
        &mut self,
        cmd: &d3d12::GraphicsCommandList,
        image: &ImageData,
    ) -> Result<ManagedTexture, RenderError> {
        let ImageData::Color(image) = image;
        let pixels = image.pixels.clone();
        let Some((cpu_handle, gpu_handle)) = self.descriptor_heap_alloc.allocate() else {
            return Err(RenderError::General("Texture descriptor heap out of slots"));
        };

        let tex_desc = d3d12::ResourceDesc::new(d3d12::ResourceDimension::Texture2D)
            .alignment(0)
            .width(image.width() as _)
            .height(image.height() as _)
            .mip_levels(1)
            .format(Format::R8g8b8a8Unorm);

        let tex = self
            .gpu
            .allocate_resource(&ResourceCreateDesc {
                name: "egui texture",
                memory_location: MemoryLocation::GpuOnly,
                resource_category: ResourceCategory::OtherTexture,
                resource_desc: tex_desc.as_ref(),
                castable_formats: &[],
                clear_value: None,
                initial_state_or_layout: ResourceStateOrBarrierLayout::ResourceState(
                    d3d12::D3D12_RESOURCE_STATE_COMMON,
                ),
                resource_type: &gpu_allocator::d3d12::ResourceType::Placed,
            })
            .expect("Failed to create texture resource");

        self.gpu.create_shader_resource_view(
            Some(tex.resource()),
            Some(&ShaderResourceViewDesc::texture_2d(
                Format::R8g8b8a8Unorm,
                0,
                1,
                0.0,
                0,
            )),
            cpu_handle,
        );

        let tex = ManagedTexture {
            resource: tex,
            width: image.width(),
            height: image.height(),
            pixels,
            cpu_handle,
            gpu_handle,
        };

        self.upload_texture(cmd, &tex)?;

        Ok(tex)
    }

    /// Upload the texture data for an allocated texture
    fn upload_texture(
        &mut self,
        cmd: &d3d12::GraphicsCommandList,
        texture: &ManagedTexture,
    ) -> Result<(), RenderError> {
        cmd.resource_barriers(&[ResourceBarrier::transition(
            texture.resource.resource(),
            0,
            ResourceStates::COMMON,
            ResourceStates::COPY_DEST,
        )]);

        let footprint =
            self.gpu
                .get_copyable_footprints(&texture.resource.resource().desc(), 0, 1, 0)?;
        let layout = &footprint.layouts[0];

        let upload_buffer = self
            .gpu
            .allocate_upload_buffer(footprint.total_bytes)
            .expect("Failed to allocate upload buffer");

        let mapped_ptr = upload_buffer
            .resource()
            .map(0)
            .expect("Failed to map upload buffer");
        let data = bytemuck::cast_slice::<Color32, u8>(&texture.pixels);
        unsafe {
            for row in 0..texture.height {
                let src = data.as_ptr().add(row * texture.width * 4);
                let dst = mapped_ptr.add(row * layout.footprint.row_pitch as usize);
                dst.copy_from_nonoverlapping(src, layout.footprint.row_pitch as usize);
            }
        }
        upload_buffer.resource().unmap(0);

        let src =
            TextureCopyLocation::placed_footprint(upload_buffer.resource(), footprint.layouts[0]);

        let dst = TextureCopyLocation::subresource(texture.resource.resource(), 0);

        cmd.copy_texture_region(&src, None, &dst, (0, 0, 0));

        cmd.resource_barriers(&[ResourceBarrier::transition(
            texture.resource.resource(),
            0,
            ResourceStates::COPY_DEST,
            ResourceStates::COMMON,
        )]);
        self.pending_uploads.push(upload_buffer);

        Ok(())
    }
}

pub struct DescriptorHeapAllocator {
    pub descriptor_heap: d3d12::DescriptorHeap,
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
