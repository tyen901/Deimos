use std::{ffi::c_void, sync::Arc, time::Instant};

use anyhow::Context;
use parking_lot::Mutex;

use crate::gpu::{Gpu, stream::FrameCommandStream};

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
    query_index: u32,
    scope_depth: usize,

    pending_spans: Vec<PendingProfilerSpan>,
    resolved_frames: Vec<Vec<ProfilerSpan>>,
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
    /// The maximum number of frames to keep for averaging results.
    pub const MAX_RESOLVED_FRAMES: usize = 15;

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
        let FrameProfilerState {
            timestamp_frequency,
            pending_spans,
            resolved_frames,
            ..
        } = &mut *self.state.lock();

        let readback_data = unsafe {
            std::slice::from_raw_parts(
                self.readback_ptr.cast::<u64>(),
                Self::MAX_PROFILER_QUERIES as usize,
            )
        };
        while resolved_frames.len() >= Self::MAX_RESOLVED_FRAMES {
            resolved_frames.remove(0);
        }
        let mut resolved_spans = Vec::with_capacity(pending_spans.len());
        for span in pending_spans {
            let gpu_start_ticks = readback_data[span.start_query_index as usize];
            let gpu_end_ticks = readback_data[span.end_query_index as usize];

            let gpu_duration_ticks = gpu_end_ticks.saturating_sub(gpu_start_ticks);
            let gpu_duration_us =
                gpu_duration_ticks as f64 / *timestamp_frequency as f64 * 1_000_000.0;

            let cpu_duration_us =
                span.end_time.duration_since(span.start_time).as_secs_f64() * 1_000_000.0;

            resolved_spans.push(ProfilerSpan {
                name: span.name,
                cpu_duration_us,
                gpu_duration_us,
                depth: span.depth,
            });
        }
        resolved_frames.push(resolved_spans);
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

    pub fn get_results_string(&self) -> String {
        let state = self.state.lock();
        if state.resolved_frames.is_empty() {
            return "No profiling data available yet".to_string();
        }

        let mut results = Vec::new();
        for scope in state
            .resolved_frames
            .last()
            .expect("resolved_frames cannot be empty")
        {
            // Average over last 10 frames
            let mut total_cpu = 0.0;
            let mut total_gpu = 0.0;
            let mut count = 0;
            for frame in state.resolved_frames.iter().rev() {
                if let Some(s) = frame
                    .iter()
                    .find(|s| s.name == scope.name && s.depth == scope.depth)
                {
                    total_cpu += s.cpu_duration_us;
                    total_gpu += s.gpu_duration_us;
                    count += 1;
                }
            }

            results.push(ProfilerSpan {
                name: scope.name,
                depth: scope.depth,
                cpu_duration_us: total_cpu / count as f64,
                gpu_duration_us: total_gpu / count as f64,
            });
        }

        let mut output = String::new();
        output.push_str(&format!(
            "{:<30} {:>12} {:>12}\n",
            "Scope", "CPU (ms)", "GPU (ms)"
        ));
        output.push_str(&"-".repeat(56));
        output.push('\n');

        let mut longest_duration_cpu = 0f64;
        let mut longest_duration_gpu = 0f64;
        for span in results {
            let scope_name = if span.depth > 0 {
                format!("{}{}", "  ".repeat(span.depth), span.name)
            } else {
                span.name.to_string()
            };
            output.push_str(&format!(
                "{:<30} {:>12.3} {:>12.3}\n",
                scope_name,
                span.cpu_duration_us / 1000.0,
                span.gpu_duration_us / 1000.0
            ));
            longest_duration_cpu = longest_duration_cpu.max(span.cpu_duration_us);
            longest_duration_gpu = longest_duration_gpu.max(span.gpu_duration_us);
        }

        output.push_str(&"-".repeat(56));
        output.push('\n');
        output.push_str(&format!(
            "{:<30} {:>12.1} {:>12.1}\n",
            "Potential FPS",
            if longest_duration_cpu > 0.0 {
                1_000_000.0 / longest_duration_cpu
            } else {
                f64::INFINITY
            },
            if longest_duration_gpu > 0.0 {
                1_000_000.0 / longest_duration_gpu
            } else {
                f64::INFINITY
            }
        ));

        output
    }

    pub fn scope<'a>(
        &'a self,
        gpu: &Arc<Gpu>,
        stream: &'a FrameCommandStream,
        name: &'static str,
    ) -> ScopeGuard<'a> {
        let mut state = self.state.lock();
        // let Some(cmd) = stream.active_linear_cmd() else {
        //     error!("FrameProfiler::scope({name:?}) needs an active linear CMD!");
        //     return ScopeGuard {
        //         profiler: self,
        //         stream,
        //         pending_span_index: usize::MAX,
        //     };
        // };

        if state.pending_spans.len() as u32 >= Self::MAX_PROFILER_SPANS {
            warn!("Exceeded maximum number of profiler spans");
            return ScopeGuard {
                profiler: self,
                stream,
                gpu: gpu.clone(),
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

        if let Some(cmd) = stream.active_linear_cmd() {
            cmd.end_query(&self.heap, d3d12::QueryType::Timestamp, start_query_index);
        } else {
            stream.acquire_cmd(gpu).end_query(
                &self.heap,
                d3d12::QueryType::Timestamp,
                start_query_index,
            );
        }

        ScopeGuard {
            profiler: self,
            stream,
            gpu: gpu.clone(),
            pending_span_index,
        }
    }
}

pub struct ScopeGuard<'a> {
    profiler: &'a FrameProfiler,
    gpu: Arc<Gpu>,
    stream: &'a FrameCommandStream,
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
        if let Some(cmd) = self.stream.active_linear_cmd() {
            cmd.end_query(
                &self.profiler.heap,
                d3d12::QueryType::Timestamp,
                pending_span.end_query_index,
            );
        } else {
            self.stream.acquire_cmd(&self.gpu).end_query(
                &self.profiler.heap,
                d3d12::QueryType::Timestamp,
                pending_span.end_query_index,
            );
        }
        state.scope_depth = state.scope_depth.saturating_sub(1);
    }
}
