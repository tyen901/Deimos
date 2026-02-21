// pub mod cbuffer;
// pub mod command_list;
// pub mod debug_text;
// mod global_state;
// pub mod profiler;
// pub mod spinner;
// pub mod state;
pub mod buffer;
pub mod frame;
pub mod swapchain;

use std::rc::Rc;

use anyhow::Context;
use d3d12::{
    CommandQueueDesc, D3D12GetDebugInterface, DxgiUsage, ID3D12Debug, SwapChainDesc, SwapEffect,
};
use gpu_allocator::{d3d12::ID3D12DeviceVersion, AllocationSizes, AllocatorDebugSettings};
use parking_lot::Mutex;
use swapchain::Swapchain;
use windows::{
    core::Interface,
    Win32::{
        Foundation::HWND,
        Graphics::Dxgi::{
            CreateDXGIFactory2, IDXGIAdapter3, IDXGIFactory4, DXGI_CREATE_FACTORY_DEBUG,
            DXGI_CREATE_FACTORY_FLAGS, DXGI_MEMORY_SEGMENT_GROUP_LOCAL,
            DXGI_QUERY_VIDEO_MEMORY_INFO,
        },
    },
};

use crate::gpu::frame::FrameContext;

pub struct Gpu {
    adapter: IDXGIAdapter3,
    pub device: d3d12::Device,
    pub swapchain: Mutex<Swapchain>,
    // global_states: global_state::RenderStates,
    pub allocator: Mutex<gpu_allocator::d3d12::Allocator>,
    pub queue: d3d12::CommandQueue,

    pub(crate) frames: [FrameContext; Self::FRAMES_IN_FLIGHT],
    pub(crate) frame_index: std::sync::atomic::AtomicUsize,
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
            queue,
            adapter: adapter3,
            swapchain: Mutex::new(Swapchain::new(swap_chain, &device, window_size)?),
            // global_states: global_state::RenderStates::new(&device)?,
            frames: std::array::from_fn(|_| {
                FrameContext::new(&device).expect("Failed to create frame context")
            }),
            device,
            allocator: Mutex::new(allocator),
            frame_index: std::sync::atomic::AtomicUsize::new(0),
        })
    }
}

// Frame management
impl Gpu {
    pub fn current_frame(&self) -> &FrameContext {
        let frame_index = self.frame_index.load(std::sync::atomic::Ordering::Relaxed);
        &self.frames[frame_index % Self::FRAMES_IN_FLIGHT]
    }

    pub fn frame_index(&self) -> usize {
        self.frame_index.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn increment_frame(&self) {
        self.frame_index
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    #[profiling::function]
    pub fn present(&self, vsync: bool) {
        self.swapchain.lock().present(vsync);
    }

    // pub fn acquire_rtv(&self) -> d3d12::RenderTargetView {
    //     self.swapchain
    //         .lock()
    //         .swapchain_target
    //         .as_ref()
    //         .unwrap()
    //         .clone()
    // }

    pub fn swapchain_resolution(&self) -> (u32, u32) {
        self.swapchain.lock().swapchain_resolution
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
