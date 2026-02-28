// pub mod cbuffer;
// pub mod command_list;
// pub mod debug_text;
mod global_state;
pub mod pipeline_cache;
// pub mod profiler;
// pub mod spinner;
// pub mod state;
pub mod alloc;
pub mod buffer;
pub mod command_list;
pub mod frame;
pub mod native_command_list;
pub mod render_target;
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
    ext::GpuFence,
};
use gpu_allocator::{
    AllocationSizes, AllocatorDebugSettings,
    d3d12::{ID3D12DeviceVersion, ResourceStateOrBarrierLayout},
};
use itertools::Itertools;
use parking_lot::Mutex;
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

use crate::gpu::{
    alloc::{descriptors::DescriptorHeapAllocator, resource::OwnedResource},
    frame::FrameContext,
    native_command_list::{CommandListRing, NativeCommandList},
    pipeline_cache::PipelineCache,
};

pub struct Gpu {
    adapter: IDXGIAdapter3,
    pub device: d3d12::Device,
    pub swapchain: Mutex<Swapchain>,
    // global_states: global_state::RenderStates,
    pub allocator: Mutex<gpu_allocator::d3d12::Allocator>,
    pub queue: d3d12::CommandQueue,

    pub pipeline_cache: Mutex<PipelineCache>,

    pub(crate) frames: [FrameContext; Self::FRAMES_IN_FLIGHT],
    pub(crate) frame_index: AtomicUsize,
    pub(crate) frame_fence: GpuFence,

    cmd_ring: CommandListRing,
    pub resource_heap: Mutex<DescriptorHeapAllocator>,

    /// List of resources to be destroyed after the frame is finished.
    bin: Mutex<Vec<(Box<dyn Any>, u8)>>,
}

unsafe impl Sync for Gpu {}
unsafe impl Send for Gpu {}

#[profiling::all_functions]
impl Gpu {
    pub const FRAMES_IN_FLIGHT: usize = 3;

    pub fn create(window: &Rc<sdl3::video::Window>) -> anyhow::Result<Self> {
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

        let allocator =
            gpu_allocator::d3d12::Allocator::new(&gpu_allocator::d3d12::AllocatorCreateDesc {
                device: ID3D12DeviceVersion::Device(device.as_windows().clone()),
                debug_settings: AllocatorDebugSettings::default(),
                allocation_sizes: AllocationSizes::default(),
            })
            .context("Failed to create allocator")?;

        let window_size = window.size();
        Ok(Self {
            cmd_ring: CommandListRing::new(&device, queue.clone(), 16)?,
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
                FrameContext::new(&device).expect("Failed to create frame context")
            }),
            frame_fence: GpuFence::new(&device)?,
            device,
            allocator: Mutex::new(allocator),
            frame_index: AtomicUsize::new(0),
            bin: Mutex::new(Vec::new()),
        })
    }

    pub fn allocate_resource(
        self: &Arc<Self>,
        desc: &gpu_allocator::d3d12::ResourceCreateDesc,
    ) -> anyhow::Result<OwnedResource> {
        let res = self.allocator.lock().create_resource(desc)?;

        let current_state =
            if let ResourceStateOrBarrierLayout::ResourceState(s) = desc.initial_state_or_layout {
                d3d12::ResourceStates::from_bits_truncate(s.0)
            } else {
                error!("Gpu::allocate_resource used with BarrierLayout, expected ResourceState");
                d3d12::ResourceStates::COMMON
            };

        Ok(OwnedResource::new(self.clone(), res, current_state))
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
        let frame_index = self.frame_index.load(std::sync::atomic::Ordering::Relaxed);
        let frame = &self.frames[frame_index % Self::FRAMES_IN_FLIGHT];
        frame.begin_frame();
        _ = frame.wait_for_completion(&self.frame_fence);

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

    /// Signal the frame fence and increments the frame index
    ///
    /// Should be called after submitting the current frame's commandlist
    pub fn end_frame(&self) {
        let frame_index = self.frame_index.load(std::sync::atomic::Ordering::Relaxed);
        let frame = &self.frames[frame_index % Self::FRAMES_IN_FLIGHT];
        frame.signal(&self.frame_fence, &self.queue);

        self.frame_index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn cmd_scope<F>(&self, func: F) -> anyhow::Result<()>
    where
        F: FnOnce(&NativeCommandList) -> anyhow::Result<()>,
    {
        self.cmd_ring.submit(func)
    }

    #[profiling::function]
    pub fn present(&self, vsync: bool) {
        self.swapchain.lock().present(vsync);
    }

    pub fn swapchain_resolution(&self) -> (u32, u32) {
        self.swapchain.lock().swapchain_resolution
    }

    pub fn wait_for_idle(&self) {
        let fence = GpuFence::new(&self.device).unwrap();
        let fence_value = fence.signal(&self.queue);
        _ = fence.wait(fence_value, None);
    }

    pub fn shutdown(&self) {
        self.wait_for_idle();
    }

    pub fn bin_resource<T: 'static>(&self, resource: T) {
        self.bin
            .try_lock_for(Duration::from_secs(5))
            .expect("Failed to acquire bin lock")
            .push((Box::new(resource), 4));
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
            String::from_utf16_lossy(&d[..len]).to_string()
        }
    }

    pub fn get_memory_stats(&self) -> DXGI_QUERY_VIDEO_MEMORY_INFO {
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
