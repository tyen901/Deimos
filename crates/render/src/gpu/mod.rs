// pub mod cbuffer;
// pub mod command_list;
// pub mod debug_text;
mod global_state;
pub mod pipeline_cache;
pub mod profiler;
// pub mod spinner;
// pub mod state;
pub mod alloc;
pub mod buffer;
pub mod command_list;
pub mod frame;
pub mod magic_textures;
pub mod native_command_list;
pub mod render_target;
pub mod stream;
pub mod swapchain;

use std::{
    any::Any,
    rc::Rc,
    sync::{Arc, atomic::AtomicUsize},
    time::Duration,
};

use anyhow::Context;
use d3d12::{
    CommandQueueDesc, D3D12GetDebugInterface, DxgiUsage, ID3D12Debug, SwapChainDesc, SwapEffect,
    error::D3DResultExt,
    ext::{GpuFence, GpuFenceWaiter},
};
use gpu_allocator::{
    AllocationSizes, AllocatorDebugSettings,
    d3d12::{ID3D12DeviceVersion, ResourceStateOrBarrierLayout},
};
use itertools::Itertools;
use parking_lot::{Mutex, RwLock};
use swapchain::Swapchain;
use windows::{
    Win32::{
        Foundation::HWND,
        Graphics::Dxgi::{
            CreateDXGIFactory2, DXGI_CREATE_FACTORY_DEBUG, DXGI_CREATE_FACTORY_FLAGS,
            DXGI_MEMORY_SEGMENT_GROUP_LOCAL, DXGI_QUERY_VIDEO_MEMORY_INFO, IDXGIAdapter3,
            IDXGIFactory4,
        },
    },
    core::Interface,
};

use crate::{
    gpu::{
        alloc::{descriptors::DescriptorHeapAllocator, resource::OwnedResource},
        frame::FrameContext,
        magic_textures::MagicTextureContainer,
        native_command_list::{AsyncCommandListRing, NativeCommandList},
        pipeline_cache::PipelineCache,
        profiler::ScopeGuard,
        stream::FrameCommandStream,
    },
    tfx::externs::BaseExternSource,
};

pub struct Gpu {
    adapter: IDXGIAdapter3,
    pub device: d3d12::Device,
    pub swapchain: Mutex<Swapchain>,
    // global_states: global_state::RenderStates,
    pub allocator: Mutex<gpu_allocator::d3d12::Allocator>,
    pub queue: d3d12::CommandQueue,

    // TODO(cohae): This (more or less) belongs in renderer
    pub pipeline_cache: Mutex<PipelineCache>,

    pub(crate) frames: [FrameContext; Self::FRAMES_IN_FLIGHT],
    pub(crate) frame_index: AtomicUsize,
    pub(crate) frame_fence: GpuFence,

    cmd_ring: AsyncCommandListRing,
    pub resource_heap: Mutex<DescriptorHeapAllocator>,

    /// List of resources to be destroyed after the frame is finished.
    bin: Mutex<Vec<(Box<dyn Any + Send>, u8)>>,

    // TODO(cohae): This belongs in renderer
    pub extern_source: RwLock<BaseExternSource>,

    pub magic_textures: MagicTextureContainer,

    pub num_bytes_allocated_for_textures: AtomicUsize,
    pub num_bytes_allocated_for_render_targets: AtomicUsize,
    pub num_bytes_allocated_for_buffers: AtomicUsize,
    pub num_bytes_allocated_for_transfer_buffers: AtomicUsize,
}

unsafe impl Sync for Gpu {}
unsafe impl Send for Gpu {}

#[profiling::all_functions]
impl Gpu {
    pub const FRAMES_IN_FLIGHT: usize = 3;

    pub fn create(window: &Rc<sdl3::video::Window>) -> anyhow::Result<Self> {
        d3d12::enable_dred()?;
        if cfg!(debug_assertions) {
            unsafe {
                let mut debug: Option<ID3D12Debug> = None;
                if let Some(debug) = D3D12GetDebugInterface(&mut debug).ok().and(debug) {
                    debug.EnableDebugLayer();
                }
            }
        }

        let dxgi_factory: IDXGIFactory4 = unsafe {
            CreateDXGIFactory2(if cfg!(debug_assertions) {
                DXGI_CREATE_FACTORY_DEBUG
            } else {
                DXGI_CREATE_FACTORY_FLAGS(0)
            })
        }?;

        let use_warp = std::env::var("DEIMOS_USE_WARP") == Ok("1".to_string());
        let adapter = if use_warp {
            info!("Creating WARP adapter");
            unsafe {
                dxgi_factory
                    .EnumWarpAdapter()
                    .context("No warp adapter found")?
            }
        } else {
            unsafe { dxgi_factory.EnumAdapters(0).context("No adapters found")? }
        };
        let adapter3 = adapter
            .cast::<IDXGIAdapter3>()
            .context("Couldn't find a compatible adapter")?;

        let output_window = {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            match window.window_handle().unwrap().as_raw() {
                RawWindowHandle::Win32(h) => HWND(h.hwnd.get() as *mut _),
                u => anyhow::bail!("Can't open window for {u:?}"),
            }
        };

        let device = d3d12::Device::create(Some(adapter)).context("Failed to create device")?;

        let queue = device.create_command_queue(
            &CommandQueueDesc::builder()
                .type_(d3d12::CommandListType::Direct)
                .build(),
        )?;

        let swap_chain = d3d12::SwapChain::create(
            &dxgi_factory,
            &queue,
            Some(output_window),
            &SwapChainDesc::builder()
                .format(d3d12::Format::R8g8b8a8Unorm)
                .buffer_usage(DxgiUsage::RENDER_TARGET_OUTPUT)
                .buffer_count(2)
                .swap_effect(SwapEffect::FlipDiscard)
                .build(),
        )
        .context("Failed to create swap chain")?;

        let mut allocator =
            gpu_allocator::d3d12::Allocator::new(&gpu_allocator::d3d12::AllocatorCreateDesc {
                device: ID3D12DeviceVersion::Device(device.as_windows().clone()),
                debug_settings: AllocatorDebugSettings::default(),
                allocation_sizes: AllocationSizes::default(),
            })
            .context("Failed to create allocator")?;

        let window_size = window.size();
        Ok(Self {
            cmd_ring: AsyncCommandListRing::new(&device, queue.clone(), 4)?,
            resource_heap: Mutex::new(DescriptorHeapAllocator::new(
                &device,
                d3d12::DescriptorHeapType::CbvSrvUav,
                1_000_000,
                false,
            )?),
            pipeline_cache: Mutex::new(PipelineCache::new(device.clone())),
            queue,
            adapter: adapter3,
            swapchain: Mutex::new(Swapchain::new(swap_chain, &device, window_size)?),
            // global_states: global_state::RenderStates::new(&device)?,
            frames: std::array::from_fn(|_| {
                FrameContext::new(&device, &mut allocator).expect("Failed to create frame context")
            }),
            frame_fence: GpuFence::new(&device)?,
            device,
            allocator: Mutex::new(allocator),
            frame_index: AtomicUsize::new(0),
            bin: Mutex::new(Vec::new()),
            extern_source: RwLock::new(BaseExternSource::None),
            magic_textures: MagicTextureContainer::default(),

            num_bytes_allocated_for_textures: AtomicUsize::new(0),
            num_bytes_allocated_for_render_targets: AtomicUsize::new(0),
            num_bytes_allocated_for_buffers: AtomicUsize::new(0),
            num_bytes_allocated_for_transfer_buffers: AtomicUsize::new(0),
        })
    }

    pub fn allocate_resource(
        self: &Arc<Self>,
        desc: &gpu_allocator::d3d12::ResourceCreateDesc<'_>,
    ) -> anyhow::Result<OwnedResource> {
        let res = self
            .allocator
            .lock()
            .create_resource(desc)
            .catch_device_removal(self)?;

        let current_state =
            if let ResourceStateOrBarrierLayout::ResourceState(s) = desc.initial_state_or_layout {
                d3d12::ResourceStates::from_bits_truncate(s.0)
            } else {
                error!("Gpu::allocate_resource used with BarrierLayout, expected ResourceState");
                d3d12::ResourceStates::COMMON
            };

        Ok(OwnedResource::new(
            self.clone(),
            desc.resource_category,
            desc.memory_location,
            res,
            current_state,
        ))
    }

    pub fn allocate_upload_buffer(self: &Arc<Self>, size: u64) -> anyhow::Result<OwnedResource> {
        self.allocate_resource(&gpu_allocator::d3d12::ResourceCreateDesc {
            name: "generic_upload_buffer",
            memory_location: gpu_allocator::MemoryLocation::CpuToGpu,
            resource_category: gpu_allocator::d3d12::ResourceCategory::Buffer,
            resource_desc: d3d12::ResourceDesc::buffer(size).as_ref(),
            castable_formats: &[],
            clear_value: None,
            initial_state_or_layout: ResourceStateOrBarrierLayout::ResourceState(
                d3d12::D3D12_RESOURCE_STATE_COPY_DEST,
            ),
            resource_type: &gpu_allocator::d3d12::ResourceType::Placed,
        })
    }
}

// Frame management
impl Gpu {
    pub fn begin_frame(&self) -> &FrameContext {
        d3d12::check_device_removed(self);
        self.swapchain.lock().wait_on_present();
        let frame_index = self.frame_index.load(std::sync::atomic::Ordering::Relaxed);
        let frame = &self.frames[frame_index % Self::FRAMES_IN_FLIGHT];
        frame.begin_frame(&self.queue);
        frame
            .wait_for_completion(&self.frame_fence)
            .expect("wait for frame completion");

        let _to_bin = self
            .bin
            .lock()
            .extract_if(.., |(_res, lifetime)| {
                if *lifetime == 0 {
                    true
                } else {
                    *lifetime -= 1;
                    false
                }
            })
            .collect_vec();

        frame
    }

    pub fn frame_index(&self) -> usize {
        self.frame_index.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn frame(&self) -> &FrameContext {
        &self.frames[self.frame_index() % Self::FRAMES_IN_FLIGHT]
    }

    pub fn previous_frame(&self) -> &FrameContext {
        &self.frames[(self.frame_index() - 1) % Self::FRAMES_IN_FLIGHT]
    }

    /// Signal the frame fence and increments the frame index
    ///
    /// Should be called after submitting the current frame's commandlist
    pub fn end_frame(self: &Arc<Self>) {
        let frame_index = self.frame_index.load(std::sync::atomic::Ordering::Relaxed);
        let frame = &self.frames[frame_index % Self::FRAMES_IN_FLIGHT];

        frame.stream.acquire_cmd(self).scope(|cmd| {
            frame.profiler.resolve_query_data(cmd);
        });

        frame.stream.submit(&self.queue);
        frame.signal(&self.frame_fence, &self.queue);
        self.cmd_ring.advance().expect("advance command ring");

        self.frame_index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        d3d12::check_device_removed(self);
    }

    pub fn cmd_scope<F>(&self, func: F) -> anyhow::Result<Arc<GpuFenceWaiter>>
    where
        F: FnOnce(&NativeCommandList) -> anyhow::Result<()>,
    {
        self.cmd_ring.submit(func)
    }

    #[profiling::function]
    pub fn present(&self, vsync: bool) {
        self.swapchain.lock().present(vsync);
        d3d12::check_device_removed(self);
    }

    pub fn swapchain_resolution(&self) -> (u32, u32) {
        self.swapchain.lock().swapchain_resolution
    }

    pub fn wait_for_idle(&self) {
        let fence = GpuFence::new(&self.device).unwrap();
        let fence_value = fence.signal(&self.queue);
        fence
            .wait(fence_value, None)
            .expect("Failed to wait for idle fence");
    }

    pub fn shutdown(&self) {
        self.wait_for_idle();
    }

    pub fn bin_resource<T: 'static + Send>(&self, resource: T) {
        self.bin
            .try_lock_for(Duration::from_secs(5))
            .expect("Failed to acquire bin lock")
            .push((Box::new(resource), 4));
    }

    pub fn profiler_scope<'a>(
        self: &'a Arc<Self>,
        stream: &'a FrameCommandStream,
        name: &'static str,
    ) -> ScopeGuard<'a> {
        self.frame().profiler.scope(self, stream, name)
    }

    // #[profiling::function]
    // pub fn resize_swapchain(&self, size: (u32, u32)) {
    //     self.swapchain.lock().resize(&self.device, size);
    // }
}

// Adapter info
impl Gpu {
    pub fn get_adapter_name(&self) -> String {
        unsafe {
            let d = self.adapter.GetDesc().unwrap().Description;
            let len = d.iter().position(|&x| x == 0).unwrap();
            String::from_utf16_lossy(&d[..len])
        }
    }

    pub fn memory_stats(&self) -> MemoryStats {
        let d3d12_stats = self.d3d12_memory_stats();
        let resource_heap = self.resource_heap.lock();

        let mut r = MemoryStats {
            total_bytes_used: d3d12_stats.CurrentUsage,
            descriptor_heap_used: resource_heap.used(),
            descriptor_heap_capacity: resource_heap.capacity(),
            descriptor_ring_used: self.previous_frame().descriptors.used(),
            descriptor_ring_capacity: self.previous_frame().descriptors.capacity(),
            num_upload_ring_allocations: self.previous_frame().upload.num_allocations(),
            upload_ring_used: self.previous_frame().upload.used(),
            upload_ring_capacity: self.previous_frame().upload.capacity(),
            errors: MemoryReportCategories::empty(),
            warnings: MemoryReportCategories::empty(),
            high_water: false,
        };

        let descriptor_heap_used_ratio =
            r.descriptor_heap_used as f64 / r.descriptor_heap_capacity as f64;
        let descriptor_ring_used_ratio =
            r.descriptor_ring_used as f64 / r.descriptor_ring_capacity as f64;
        let upload_ring_used_ratio = r.upload_ring_used as f64 / r.upload_ring_capacity as f64;

        if descriptor_heap_used_ratio >= 1.0 {
            r.errors |= MemoryReportCategories::DESCRIPTOR_HEAP;
        } else if descriptor_heap_used_ratio >= 0.8 {
            r.warnings |= MemoryReportCategories::DESCRIPTOR_HEAP;
        }

        if descriptor_heap_used_ratio >= 0.9 {
            r.high_water = true;
        }

        if descriptor_ring_used_ratio >= 1.0 {
            r.errors |= MemoryReportCategories::DESCRIPTOR_RING;
        } else if descriptor_ring_used_ratio >= 0.8 {
            r.warnings |= MemoryReportCategories::DESCRIPTOR_RING;
        }

        if descriptor_ring_used_ratio >= 0.9 {
            r.high_water = true;
        }

        if upload_ring_used_ratio >= 1.0 {
            r.errors |= MemoryReportCategories::UPLOAD_RING;
        } else if upload_ring_used_ratio >= 0.8 {
            r.warnings |= MemoryReportCategories::UPLOAD_RING;
        }

        if upload_ring_used_ratio >= 0.9 {
            r.high_water = true;
        }

        r
    }

    pub fn d3d12_memory_stats(&self) -> DXGI_QUERY_VIDEO_MEMORY_INFO {
        unsafe {
            let mut memory_info = DXGI_QUERY_VIDEO_MEMORY_INFO::default();
            self.adapter
                .QueryVideoMemoryInfo(0, DXGI_MEMORY_SEGMENT_GROUP_LOCAL, &mut memory_info)
                .unwrap();

            memory_info
        }
    }
}

impl std::ops::Deref for Gpu {
    type Target = d3d12::Device;

    fn deref(&self) -> &Self::Target {
        &self.device
    }
}

pub struct MemoryStats {
    pub total_bytes_used: u64,

    pub descriptor_heap_used: usize,
    pub descriptor_heap_capacity: usize,

    pub descriptor_ring_used: usize,
    pub descriptor_ring_capacity: usize,

    pub num_upload_ring_allocations: usize,
    pub upload_ring_used: usize,
    pub upload_ring_capacity: usize,

    pub warnings: MemoryReportCategories,
    pub errors: MemoryReportCategories,
    pub high_water: bool,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy)]
    pub struct MemoryReportCategories: u8 {
        const ALLOCATOR = 1 << 0;
        const DESCRIPTOR_HEAP = 1 << 1;

        const DESCRIPTOR_RING = 1 << 2;
        const UPLOAD_RING = 1 << 3;
    }
}
