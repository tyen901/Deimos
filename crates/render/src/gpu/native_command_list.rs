use d3d12::{Event, ext::GpuFence};
use parking_lot::Mutex;
use std::{
    ops::Deref,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Instant,
};

use crate::features::static_geometry::InstanceTransformBlock;

pub struct NativeCommandList {
    command_allocator: d3d12::CommandAllocator,
    pub command_list: d3d12::GraphicsCommandList,
    is_open: AtomicBool,
}

impl NativeCommandList {
    pub fn new(device: &d3d12::Device) -> anyhow::Result<Self> {
        let command_allocator = device.create_command_allocator(d3d12::CommandListType::Direct)?;
        let command_list = device.create_command_list(
            0,
            d3d12::CommandListType::Direct,
            &command_allocator,
            None,
        )?;
        command_list.close()?;

        Ok(Self {
            command_allocator,
            command_list,
            is_open: AtomicBool::new(false),
        })
    }

    pub fn is_open(&self) -> bool {
        self.is_open.load(Ordering::Acquire)
    }

    pub fn begin(&self) -> anyhow::Result<()> {
        assert!(!self.is_open(), "command list is already recording");
        self.command_allocator.reset()?;
        self.command_list.reset(&self.command_allocator, None)?;
        self.is_open.store(true, Ordering::Release);
        Ok(())
    }

    pub fn end(&self) -> anyhow::Result<()> {
        assert!(self.is_open(), "command list is not recording");
        self.command_list.close()?;
        self.is_open.store(false, Ordering::Release);
        Ok(())
    }

    #[deprecated(note = "use begin() instead")]
    pub fn reset(&self) -> anyhow::Result<()> {
        self.command_list.reset(&self.command_allocator, None)?;
        Ok(())
    }

    #[deprecated(note = "use end() instead")]
    pub fn close(&self) -> anyhow::Result<()> {
        self.command_list.close()?;
        Ok(())
    }

    pub fn scope<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&d3d12::GraphicsCommandList) -> anyhow::Result<()>,
    {
        self.begin()?;
        f(&self.command_list)?;
        self.end()
    }
}

impl Deref for NativeCommandList {
    type Target = d3d12::GraphicsCommandList;

    fn deref(&self) -> &Self::Target {
        &self.command_list
    }
}
struct CommandListSlot {
    command_list: NativeCommandList,
    fence_value: u64,
}

pub struct CommandListRing {
    queue: d3d12::CommandQueue,
    fence: d3d12::Fence,
    fence_event: d3d12::Event,

    next_fence_value: AtomicU64,
    slots: Mutex<Box<[CommandListSlot]>>,
    head: Mutex<usize>,
}

impl CommandListRing {
    pub fn new(
        device: &d3d12::Device,
        queue: d3d12::CommandQueue,
        capacity: usize,
    ) -> anyhow::Result<Self> {
        let fence = device.create_fence(0)?;
        let fence_event = Event::new(false, false)?;

        let slots = (0..capacity)
            .map(|_| {
                let command_list = NativeCommandList::new(device)?;
                Ok(CommandListSlot {
                    command_list,
                    fence_value: 0,
                })
            })
            .collect::<anyhow::Result<Box<[_]>>>()?;

        Ok(Self {
            queue,
            fence,
            fence_event,
            next_fence_value: AtomicU64::new(1),
            slots: Mutex::new(slots),
            head: Mutex::new(0),
        })
    }

    pub fn submit<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&NativeCommandList) -> anyhow::Result<()>,
    {
        let mut head = self.head.lock();
        let mut slots = self.slots.lock();
        let slot = &mut slots[*head];

        let start = Instant::now();
        self.wait_for(slot.fence_value)?;

        slot.command_list.begin()?;

        f(&slot.command_list)?;

        slot.command_list.end()?;
        self.queue
            .execute_command_lists(std::slice::from_ref(&slot.command_list));

        let fence_value = self.next_fence_value.fetch_add(1, Ordering::Relaxed);
        self.queue.signal(&self.fence, fence_value)?;
        slot.fence_value = fence_value;

        *head = (*head + 1) % slots.len();

        Ok(())
    }

    /// Wait for a previously returned fence value.
    /// Useful if a caller needs to know uploads are done before proceeding.
    pub fn wait_for(&self, fence_value: u64) -> anyhow::Result<()> {
        if self.fence.get_completed_value() < fence_value {
            self.fence
                .set_event_on_completion(&self.fence_event, fence_value)?;
            self.fence_event.wait(None);
        }
        Ok(())
    }
}
