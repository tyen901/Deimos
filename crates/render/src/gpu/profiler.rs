use std::{ffi::c_void, sync::Arc, time::Instant};

use anyhow::Context;
use parking_lot::Mutex;

use crate::gpu::{Gpu, alloc::resource::OwnedResource};

struct PendingProfilerSpan {
    name: &'static str,
    start_time: Instant,
    end_time: Instant,
    start_query_index: u32,
    end_query_index: u32,
    depth: usize,
}

struct ProfilerSpan {
    name: &'static str,
    cpu_duration_us: f64,
    gpu_duration_us: f64,
    depth: usize,
}

#[derive(Default)]
struct FrameProfilerState {
    timestamp_frequency: u64,
    pending_spans: Vec<PendingProfilerSpan>,
    query_index: u32,
    scope_depth: usize,
}

pub struct FrameProfiler {
    heap: d3d12::QueryHeap,
    readback: gpu_allocator::d3d12::Resource,
    readback_ptr: *mut c_void,

    state: Mutex<FrameProfilerState>,
}

impl FrameProfiler {
    pub const MAX_PROFILER_SPANS: u32 = 64;
    pub const MAX_PROFILER_QUERIES: u32 = Self::MAX_PROFILER_SPANS * 2;

    pub fn new(
        device: &d3d12::Device,
        allocator: &mut gpu_allocator::d3d12::Allocator,
    ) -> anyhow::Result<Self> {
        let heap = device
            .create_query_heap(
                d3d12::QueryHeapType::Timestamp,
                Self::MAX_PROFILER_QUERIES,
                0,
            )
            .context("creating query heap")?;

        let readback = allocator
            .create_resource(&gpu_allocator::d3d12::ResourceCreateDesc {
                name: "Profiler readback buffer",
                memory_location: gpu_allocator::MemoryLocation::GpuToCpu,
                resource_category: gpu_allocator::d3d12::ResourceCategory::Buffer,
                resource_desc: d3d12::ResourceDesc::buffer(Self::MAX_PROFILER_QUERIES as u64 * 8)
                    .as_ref(),
                castable_formats: &[],
                clear_value: None,
                initial_state_or_layout:
                    gpu_allocator::d3d12::ResourceStateOrBarrierLayout::ResourceState(
                        d3d12::D3D12_RESOURCE_STATE_COPY_DEST,
                    ),
                resource_type: &gpu_allocator::d3d12::ResourceType::Placed,
            })
            .context("allocating readback buffer")?;

        Ok(Self {
            heap,
            readback_ptr: readback.resource().as_ref().map(0)?.cast(),
            readback,
            state: Mutex::new(FrameProfilerState::default()),
        })
    }

    pub(super) fn begin_frame(&self, queue: &d3d12::CommandQueue) {
        self.resolve_pending_queries();

        let mut state = self.state.lock();
        state.query_index = 0;
        state.pending_spans.clear();
        state.scope_depth = 0;
        state.timestamp_frequency = queue
            .get_timestamp_frequency()
            .inspect_err(|e| {
                warn!("Failed to obtain timestamp frequency: {e:?}");
            })
            .unwrap_or(1_000_000);
    }

    fn resolve_pending_queries(&self) {
        let mut state = self.state.lock();

        let readback_data = unsafe {
            std::slice::from_raw_parts(
                self.readback_ptr.cast::<u64>(),
                Self::MAX_PROFILER_QUERIES as usize,
            )
        };

        for span in &state.pending_spans {
            let gpu_start_ticks = readback_data[span.start_query_index as usize];
            let gpu_end_ticks = readback_data[span.end_query_index as usize];

            let gpu_duration_ticks = gpu_end_ticks.saturating_sub(gpu_start_ticks);
            let gpu_duration_us =
                gpu_duration_ticks as f64 / state.timestamp_frequency as f64 * 1_000_000.0;

            let cpu_duration_us =
                span.end_time.duration_since(span.start_time).as_secs_f64() * 1_000_000.0;

            info!(
                "Profiler span '{}' - CPU: {:.3} ms, GPU: {:.3} ms",
                span.name,
                cpu_duration_us / 1000.0,
                gpu_duration_us / 1000.0,
            );
        }
    }

    pub(super) fn resolve_query_data(&self, command_list: &d3d12::GraphicsCommandList) {
        let state = self.state.lock();
        if state.pending_spans.is_empty() {
            return;
        }

        command_list.resolve_query_data(
            &self.heap,
            d3d12::QueryType::Timestamp,
            0,
            state.pending_spans.len() as u32 * 2,
            self.readback.resource().as_ref(),
            0,
        );
    }

    pub fn scope(
        &self,
        command_list: &d3d12::GraphicsCommandList,
        name: &'static str,
    ) -> ScopeGuard<'_> {
        let mut state = self.state.lock();
        if state.pending_spans.len() as u32 >= Self::MAX_PROFILER_SPANS {
            warn!("Exceeded maximum number of profiler spans");
            return ScopeGuard {
                profiler: self,
                cmd: command_list.clone(),
                pending_span_index: usize::MAX,
            };
        }

        let start_query_index = state.query_index;
        let end_query_index = state.query_index + 1;
        state.query_index += 2;
        let pending_span_index = state.pending_spans.len();
        let depth = state.scope_depth;
        state.scope_depth += 1;
        state.pending_spans.push(PendingProfilerSpan {
            name,
            start_time: Instant::now(),
            end_time: Instant::now(),
            start_query_index,
            end_query_index,
            depth,
        });

        command_list.end_query(&self.heap, d3d12::QueryType::Timestamp, start_query_index);
        ScopeGuard {
            profiler: self,
            cmd: command_list.clone(),
            pending_span_index,
        }
    }
}

pub struct ScopeGuard<'a> {
    profiler: &'a FrameProfiler,
    cmd: d3d12::GraphicsCommandList,
    pending_span_index: usize,
}

impl Drop for ScopeGuard<'_> {
    fn drop(&mut self) {
        let mut state = self.profiler.state.lock();
        if self.pending_span_index >= state.pending_spans.len() {
            warn!("Invalid profiler scope guard index");
            return;
        }

        let pending_span = &mut state.pending_spans[self.pending_span_index];
        pending_span.end_time = Instant::now();
        self.cmd.end_query(
            &self.profiler.heap,
            d3d12::QueryType::Timestamp,
            pending_span.end_query_index,
        );
        state.scope_depth = state.scope_depth.saturating_sub(1);
    }
}
