// use std::sync::atomic::AtomicU64;

// use parking_lot::Mutex;

// use crate::gpu::{command_list::CommandList, native_command_list::NativeCommandList};

use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use parking_lot::Mutex;

use crate::gpu::{Gpu, command_list::CommandList, native_command_list::NativeCommandList};

pub struct FrameCommandStream {
    segments: Mutex<Vec<NativeCommandList>>,
    pool: CommandListPool,
}

impl FrameCommandStream {
    pub const fn new(device: d3d12::Device) -> Self {
        Self {
            segments: Mutex::new(vec![]),
            pool: CommandListPool::new(device),
        }
    }

    // fn close_active(&mut self) {
    //     if let Some(slot) = self.active_slot.take() {
    //         slot.cmd.close().unwrap();
    //         self.segments.push((slot.cmd.clone(), slot));
    //     }
    // }

    // pub fn begin_parallel(&mut self, n: usize) -> ParallelBlock {
    //     todo!()
    //     // self.close_active();

    //     // let workers = (0..n)
    //     //     .map(|_| {
    //     //         let slot = self.ring.acquire_open().unwrap();
    //     //         let mut cmd = CommandList::from_ring_slot(&slot);
    //     //         pass.apply(&mut cmd);
    //     //         (cmd, slot)
    //     //     })
    //     //     .collect();

    //     // ParallelBlock { workers }
    // }

    // pub fn end_parallel(&mut self, mut block: ParallelBlock) {
    //     todo!()
    //     // for (worker_cmd, slot) in block.workers.drain(..) {
    //     //     worker_cmd.cmd.close().unwrap();
    //     //     // Re-borrow the raw d3d12 list for the segment vec
    //     //     self.segments.push((slot.cmd.clone(), slot));
    //     // }

    //     // // Open a fresh active segment
    //     // let slot = self.ring.acquire_open().unwrap();
    //     // self.active_slot = Some(slot);
    // }

    pub fn submit(&self, queue: &d3d12::CommandQueue) {
        // self.close_active();

        let native_lists: Vec<d3d12::GraphicsCommandList> = self
            .segments
            .lock()
            .iter()
            .map(|c| c.command_list.clone())
            .collect();

        queue.execute_command_lists(&native_lists);

        for cmd in self.segments.lock().drain(..) {
            self.pool.release(cmd);
        }
    }

    pub fn acquire_cmd<'a>(&'a self, gpu: &Arc<Gpu>) -> StreamCommandListGuard<'a> {
        let c = self.pool.acquire();
        c.begin().expect("beginning cmd during acquire");

        let cmd = CommandList::from_native_command_list(gpu, c);
        StreamCommandListGuard {
            stream: self,
            cmd: ManuallyDrop::new(cmd),
        }
    }

    fn release_cmd(&self, cmd: CommandList) {
        let c = cmd.into_inner();
        c.end().expect("closing cmd during release");
        self.segments.lock().push(c);
    }
}

pub struct StreamCommandListGuard<'a> {
    stream: &'a FrameCommandStream,
    cmd: ManuallyDrop<CommandList>,
}

impl<'a> StreamCommandListGuard<'a> {
    pub fn scope<F>(mut self, f: F)
    where
        F: FnOnce(&mut CommandList),
    {
        f(&mut self.cmd);
    }
}

impl<'a> Drop for StreamCommandListGuard<'a> {
    fn drop(&mut self) {
        self.stream
            .release_cmd(unsafe { ManuallyDrop::take(&mut self.cmd) });
    }
}

impl<'a> Deref for StreamCommandListGuard<'a> {
    type Target = CommandList;

    fn deref(&self) -> &Self::Target {
        &self.cmd
    }
}

impl<'a> DerefMut for StreamCommandListGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cmd
    }
}

struct CommandListPool {
    device: d3d12::Device,
    command_lists: Mutex<Vec<NativeCommandList>>,
}

impl CommandListPool {
    const fn new(device: d3d12::Device) -> Self {
        Self {
            device,
            command_lists: Mutex::new(vec![]),
        }
    }

    fn acquire(&self) -> NativeCommandList {
        if let Some(cmd) = self.command_lists.lock().pop() {
            cmd
        } else {
            NativeCommandList::new(&self.device, d3d12::CommandListType::Direct)
                .expect("failed to create command lsit")
        }
    }

    fn release(&self, cmd: NativeCommandList) {
        assert!(
            !cmd.is_open(),
            "Command list must be closed before returning"
        );

        self.command_lists.lock().push(cmd);
    }
}
