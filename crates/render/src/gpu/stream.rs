use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    sync::Arc,
};

use deimos_core::job::SCHEDULER;
use parking_lot::Mutex;

use crate::gpu::{
    Gpu,
    command_list::{CommandList, CommandListState},
    native_command_list::NativeCommandList,
};

pub struct FrameCommandStream {
    segments: Mutex<Vec<Option<NativeCommandList>>>,
    pool: CommandListPool,

    active_linear: Mutex<Option<d3d12::GraphicsCommandList>>,
    cached_cmd_state: Mutex<CommandListState>,
}

impl FrameCommandStream {
    pub fn new(device: d3d12::Device) -> Self {
        Self {
            segments: Mutex::new(vec![]),
            pool: CommandListPool::new(device),
            active_linear: Mutex::new(None),
            cached_cmd_state: Mutex::new(CommandListState::default()),
        }
    }

    pub fn begin_parallel(&self, cmd: &mut CommandList) -> Arc<ParallelCommandBlock> {
        let num_workers = SCHEDULER.num_workers();

        let mut cmd_state = self.cached_cmd_state.lock();
        *cmd_state = cmd.cmd_state().clone();

        let first_slot;
        {
            let mut segments = self.segments.lock();
            first_slot = segments.len();
            segments.extend((0..num_workers).map(|_| None));
        }
        let workers = (0..num_workers)
            .map(|_| {
                let c = self.pool.acquire();
                c.begin().expect("beginning cmd during begin_parallel");
                let mut cmd = CommandList::from_native_command_list(cmd.gpu(), c);
                cmd.restore_cmd_state(&cmd_state);
                Some(cmd)
            })
            .collect();

        let mut new_cmd = self.acquire_cmd_inner(cmd.gpu());
        new_cmd.restore_cmd_state(&cmd_state);
        std::mem::swap(cmd, &mut new_cmd);
        let old_cmd = new_cmd;

        let slot = old_cmd
            .tag()
            .expect("begin_parallel linear command list needs a slot tag");
        self.release_cmd(old_cmd, slot as usize);
        *self.active_linear.lock() = Some(cmd.cmd.command_list.clone());

        Arc::new(ParallelCommandBlock {
            workers: Mutex::new(workers),
            first_slot,
        })
    }

    pub fn end_parallel(&self, block: Arc<ParallelCommandBlock>) {
        let mut segments = self.segments.lock();
        for (i, worker_cmd) in block.workers.lock().drain(..).enumerate() {
            let worker_cmd = worker_cmd.expect("parallel block is missing a command list");
            worker_cmd
                .cmd
                .end()
                .expect("end_parallel: end command list");
            segments[block.first_slot + i] = Some(worker_cmd.into_inner());
        }
    }

    pub fn submit(&self, queue: &d3d12::CommandQueue) {
        *self.active_linear.lock() = None;
        let native_lists: Vec<d3d12::GraphicsCommandList> = self
            .segments
            .lock()
            .iter()
            .enumerate()
            .map(|(slot, c)| {
                c.as_ref().map_or_else(
                    || panic!("encountered an unclosed command list in command stream slot {slot}"),
                    |c| c.command_list.clone(),
                )
            })
            .collect();

        queue.execute_command_lists(&native_lists);

        for cmd in self.segments.lock().drain(..).flatten() {
            self.pool.release(cmd);
        }
    }

    pub fn acquire_cmd<'a>(&'a self, gpu: &Arc<Gpu>) -> StreamCommandListGuard<'a> {
        let cmd = self.acquire_cmd_inner(gpu);
        *self.active_linear.lock() = Some(cmd.cmd.command_list.clone());
        StreamCommandListGuard {
            stream: self,
            cmd: ManuallyDrop::new(cmd),
        }
    }

    fn acquire_cmd_inner(&self, gpu: &Arc<Gpu>) -> CommandList {
        let c = self.pool.acquire();
        c.begin().expect("beginning cmd during acquire");

        let mut segments = self.segments.lock();
        let slot = segments.len();
        segments.push(None);

        CommandList::from_native_command_list(gpu, c).with_tag(slot as u64)
    }

    fn release_cmd(&self, cmd: CommandList, slot: usize) {
        let c = cmd.into_inner();
        c.end().expect("closing cmd during release");
        self.segments.lock()[slot] = Some(c);
        *self.active_linear.lock() = None;
    }

    pub fn active_linear_cmd(&self) -> Option<d3d12::GraphicsCommandList> {
        self.active_linear.lock().clone()
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
        let cmd = unsafe { ManuallyDrop::take(&mut self.cmd) };
        let slot = cmd
            .tag()
            .expect("StreamCommandListGuard command list needs a slot tag");
        self.stream.release_cmd(cmd, slot as usize);
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

pub struct ParallelCommandBlock {
    workers: Mutex<Vec<Option<CommandList>>>,
    first_slot: usize,
}

impl ParallelCommandBlock {
    pub fn cmd<'a>(&'a self) -> ParallelCommandGuard<'a> {
        let worker = potassium::current_worker_index()
            .expect("ParallelCommandBlock::cmd can only be called from a potassium job!");

        let cmd = self.workers.lock()[worker]
            .take()
            .expect("cmd is missing from worker slot?");

        ParallelCommandGuard {
            block: self,
            cmd: ManuallyDrop::new(cmd),
        }
    }

    fn release(&self, cmd: CommandList) {
        let worker = potassium::current_worker_index()
            .expect("ParallelCommandBlock::release can only be called from a potassium job!");

        let slot = &mut self.workers.lock()[worker];
        assert!(slot.is_none(), "released slot already has a cmd");
        *slot = Some(cmd);
    }
}

pub struct ParallelCommandGuard<'a> {
    block: &'a ParallelCommandBlock,
    cmd: ManuallyDrop<CommandList>,
}

impl<'a> Drop for ParallelCommandGuard<'a> {
    fn drop(&mut self) {
        let cmd = unsafe { ManuallyDrop::take(&mut self.cmd) };
        self.block.release(cmd);
    }
}

impl<'a> Deref for ParallelCommandGuard<'a> {
    type Target = CommandList;

    fn deref(&self) -> &Self::Target {
        &self.cmd
    }
}

impl<'a> DerefMut for ParallelCommandGuard<'a> {
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
