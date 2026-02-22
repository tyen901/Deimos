use std::time::Duration;

use anyhow::Context;
use d3d12::{self, DescriptorHeap, PresentFlags, SwapChainFlags, SwapChainStatus};
use itertools::Itertools;

pub struct Swapchain {
    device: d3d12::Device,
    pub swapchain: d3d12::SwapChain,
    // pub swapchain_target: Option<d3d12::RenderTargetView>,
    pub(crate) swapchain_resolution: (u32, u32),
    present_parameters: PresentFlags,

    rtv_desc_heap: DescriptorHeap,
    back_buffers: Vec<d3d12::Resource>,
}

impl Swapchain {
    const NUM_BUFFERS: u32 = 2;

    pub fn new(
        swapchain: d3d12::SwapChain,
        device: &d3d12::Device,
        size: (u32, u32),
    ) -> anyhow::Result<Self> {
        let rtv_desc_heap = device
            .create_descriptor_heap(d3d12::DescriptorHeapType::Rtv, Self::NUM_BUFFERS, false, 0)
            .context("creating descriptor heap")?;

        let back_buffers = (0..Self::NUM_BUFFERS)
            .map(|i| {
                let res = swapchain.get_buffer(i).unwrap();
                device.create_render_target_view(
                    Some(&res),
                    None,
                    rtv_desc_heap.cpu_descriptor_handle_for_heap_start().offset(
                        i as usize,
                        device.descriptor_handle_increment_size(d3d12::DescriptorHeapType::Rtv),
                    ),
                );

                res
            })
            .collect_vec();

        Ok(Self {
            device: device.clone(),
            swapchain,
            // swapchain_target: None,
            swapchain_resolution: size,
            present_parameters: PresentFlags::empty(),
            rtv_desc_heap,
            back_buffers,
        })
    }

    pub fn get_back_buffer(&self, index: usize) -> (d3d12::CpuDescriptorHandle, d3d12::Resource) {
        let handle = self
            .rtv_desc_heap
            .cpu_descriptor_handle_for_heap_start()
            .offset(
                index % Self::NUM_BUFFERS as usize,
                self.device
                    .descriptor_handle_increment_size(d3d12::DescriptorHeapType::Rtv),
            );
        let resource = &self.back_buffers[index % Self::NUM_BUFFERS as usize];
        (handle, resource.clone())
    }

    // pub fn get_buffer(&self) -> d3d12::Texture2D {
    //     self.swapchain.get_buffer(0).unwrap()
    // }

    // ⚠ The calling function MUST ensure that the RTV is not held/in use.
    pub fn resize(&mut self, new_size: (u32, u32)) {
        // drop(self.swapchain_target.take());

        self.swapchain
            .resize_buffers(
                Self::NUM_BUFFERS,
                new_size.0,
                new_size.1,
                d3d12::Format::B8g8r8a8Unorm,
                SwapChainFlags::empty(),
            )
            .unwrap();

        // let bb: d3d12::Texture2D = self.swapchain.get_buffer(0).unwrap();

        // let new_rtv = device.create_render_target_view(&bb, None).unwrap();

        // self.swapchain_target = Some(new_rtv);
        self.swapchain_resolution = new_size;
    }

    pub(crate) fn present(&mut self, vsync: bool) {
        if self
            .swapchain
            .present(vsync as u32, self.present_parameters)
            == Some(SwapChainStatus::Occluded)
        {
            self.present_parameters = PresentFlags::TEST;
            std::thread::sleep(Duration::from_millis(20));
        } else {
            self.present_parameters = PresentFlags::empty();
        }
    }
}
