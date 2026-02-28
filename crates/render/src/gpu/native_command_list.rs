use std::{
    backtrace::Backtrace,
    ops::Deref,
    time::{Duration, Instant},
};

use d3d12::ext::GpuFence;
use parking_lot::Mutex;

pub struct NativeCommandList {
    command_allocator: d3d12::CommandAllocator,
    pub command_list: d3d12::GraphicsCommandList,
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
        })
    }

    pub fn scope<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&d3d12::GraphicsCommandList) -> anyhow::Result<()>,
    {
        self.begin()?;
        f(&self.command_list)?;
        self.end()
    }

    pub fn begin(&self) -> anyhow::Result<()> {
        self.command_allocator.reset()?;
        self.command_list.reset(&self.command_allocator, None)?;
        Ok(())
    }

    pub fn end(&self) -> anyhow::Result<()> {
        self.command_list.close()?;
        Ok(())
    }
}

impl Deref for NativeCommandList {
    type Target = d3d12::GraphicsCommandList;

    fn deref(&self) -> &Self::Target {
        &self.command_list
    }
}

// TODO(cohae): We should probably make the pool per-frame
pub struct NativeCommandListPool {
    device: d3d12::Device,
    queue: d3d12::CommandQueue,
    command_lists: Mutex<Vec<(NativeCommandList, GpuFence)>>,
}

impl NativeCommandListPool {
    pub fn new(device: d3d12::Device, queue: d3d12::CommandQueue) -> anyhow::Result<Self> {
        let command_lists = Mutex::new(Vec::new());
        Ok(Self {
            device,
            queue,
            command_lists,
        })
    }

    /// Acquires a command list from the pool, executes the given function, executes the command list on the queue, and returns it to the pool.
    ///
    /// This wait for the fence to signal before returning.
    pub fn scope_immediate<F>(&self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(&d3d12::GraphicsCommandList) -> anyhow::Result<()>,
    {
        let (cmd, fence) = if let Some(g) = self.command_lists.lock().pop() {
            g
        } else {
            (
                NativeCommandList::new(&self.device)?,
                GpuFence::new(&self.device)?,
            )
        };

        cmd.begin()?;
        f(&cmd)?;
        cmd.end()?;
        self.queue
            .execute_command_lists(std::slice::from_ref(&cmd.command_list));
        let fence_value = fence.signal(&self.queue);
        let start = Instant::now();
        fence.wait(fence_value, Some(Duration::from_secs(5)))?;

        self.command_lists.lock().push((cmd, fence));
        Ok(())
    }
}
