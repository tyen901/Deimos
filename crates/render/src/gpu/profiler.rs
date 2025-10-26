use std::sync::atomic::AtomicUsize;

use d3d11::{
    query::{QueryType, D3D11_QUERY_DATA_TIMESTAMP_DISJOINT},
    GetDataResult,
};
use parking_lot::Mutex;

use crate::renderer::Renderer;

pub struct TimestampManager {
    tracy_context: tracy_client::GpuContext,
    context: d3d11::DeviceContext,

    queries: Vec<d3d11::Query>,
    pending_frames: Mutex<Vec<PendingFrame>>,
    disjoint_query: d3d11::Query,

    query_counter: AtomicUsize,
}

impl TimestampManager {
    const MAX_FRAMES: usize = 5;
    const MAX_QUERIES: usize = Self::MAX_FRAMES * 8 * 1024;

    pub fn new(device: &d3d11::Device) -> anyhow::Result<Self> {
        let queries = (0..Self::MAX_QUERIES)
            .map(|_| create_query(device, QueryType::Timestamp))
            .collect::<Result<Vec<_>, _>>()?;
        let disjoint_query = create_query(device, QueryType::TimestampDisjoint)?;

        let mut t_gpu = 0;
        let context = device.get_immediate_context();
        for _attempt in 0..50 {
            context.begin(&disjoint_query);
            context.end(&queries[0]);
            context.end(&disjoint_query);

            wait_for_query(&context, &disjoint_query);

            let r = unsafe {
                context.get_data::<D3D11_QUERY_DATA_TIMESTAMP_DISJOINT>(&disjoint_query, false)
            };
            let GetDataResult::Ok(disjoint) = r else {
                continue;
            };

            if disjoint.Disjoint.as_bool() {
                continue;
            }

            let t = unsafe { context.get_data::<u64>(&queries[0], false) };
            let GetDataResult::Ok(timestamp) = t else {
                continue;
            };
            if timestamp == 0 {
                continue;
            }

            t_gpu = timestamp * (1_000_000_000 / disjoint.Frequency);
            break;
        }

        context.begin(&disjoint_query);

        Ok(Self {
            tracy_context: tracy_client::Client::running()
                .expect("Tracy client not running")
                .new_gpu_context(
                    None,
                    tracy_client::GpuContextType::Direct3D11,
                    t_gpu as i64,
                    1.0,
                )?,
            context,
            queries,
            disjoint_query,
            pending_frames: Mutex::new(Vec::new()),
            query_counter: AtomicUsize::new(0),
            // previous_checkpoint: AtomicUsize::new(0),
            // next_checkpoint: AtomicUsize::new(0),
        })
    }

    fn get_index(&self) -> usize {
        let index = self
            .query_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if (index + 1) >= Self::MAX_QUERIES {
            self.query_counter
                .store(0, std::sync::atomic::Ordering::Relaxed);
        }

        index
    }

    pub fn begin_span(&self, location: &'static SpanLocation) -> GpuSpanGuard {
        let tracy_span = self
            .tracy_context
            .span(location)
            .expect("Failed to create Tracy GPU span");

        let start_query = self.get_index();
        let end_query = self.get_index();

        self.context.end(&self.queries[start_query]);

        GpuSpanGuard {
            tracy_span: Some(tracy_span),
            start_query,
            end_query,
        }
    }

    fn end_span(&self, mut span: PendingGpuSpan) {
        self.context.end(&self.queries[span.end_query]);
        span.tracy_span.end_zone();
        self.pending_frames.lock().last_mut().unwrap().push(span);
    }

    pub fn begin_frame(&self) {
        if !cfg!(feature = "tracy") {
            return;
        }
        self.pending_frames.lock().push(PendingFrame::default());
    }

    pub fn collect(&self) {
        if !cfg!(feature = "tracy") {
            return;
        }

        let span = tracy_client::span!();
        span.emit_color(0xff0000);

        self.context.end(&self.disjoint_query);

        // Start by culling old frames, we only need to keep the last MAX_FRAMES frames
        let mut pending_frames = self.pending_frames.lock();
        while pending_frames.len() > Self::MAX_FRAMES {
            pending_frames.remove(0);
        }

        if pending_frames.is_empty() {
            return;
        }

        wait_for_query(&self.context, &self.disjoint_query);

        let r = unsafe {
            self.context
                .get_data::<D3D11_QUERY_DATA_TIMESTAMP_DISJOINT>(&self.disjoint_query, true)
        };
        let GetDataResult::Ok(disjoint) = r else {
            return;
        };

        // If the disjoint flag is set, we can't trust the timestamps, so we throw away the current frame
        if disjoint.Disjoint.as_bool() {
            self.pending_frames.lock().pop();
            return;
        }

        if pending_frames.len() == Self::MAX_FRAMES {
            // for frame in pending_frames.extract_if(|p| p.is_ready(self)) {
            let frame = pending_frames.remove(0);
            // We've already collected the disjoint query, so we can immediately collect all the other timestamps without waiting
            for span in frame.0 {
                // let QueryResult::Ok(mut t_start) = query_get_data::<u64>(
                //     &self.context,
                //     &self.queries[span.start_query],
                //     false,
                // ) else {
                //     error!("Query data not available yet, but it should be");
                //     break;
                // };

                // let QueryResult::Ok(mut t_end) =
                //     query_get_data::<u64>(&self.context, &self.queries[span.end_query], false)
                // else {
                //     error!("Query data not available yet, but it should be");
                //     break;
                // };

                let mut t_start =
                    query_get_data_blocking::<u64>(&self.context, &self.queries[span.start_query]);
                let mut t_end =
                    query_get_data_blocking::<u64>(&self.context, &self.queries[span.end_query]);

                t_start *= 1_000_000_000 / disjoint.Frequency;
                t_end *= 1_000_000_000 / disjoint.Frequency;

                span.tracy_span
                    .upload_timestamp(t_start as i64, t_end as i64);
                // }
            }
        }

        // Restart the disjoint query for the next frame
        self.context.begin(&self.disjoint_query);
    }
}

#[derive(Default)]
pub struct PendingFrame(Vec<PendingGpuSpan>);
impl PendingFrame {
    fn push(&mut self, span: PendingGpuSpan) {
        self.0.push(span);
    }

    // fn is_ready(&self, manager: &TimestampManager) -> bool {
    //     if let Some(last_span) = self.0.last() {
    //         if let QueryResult::Ok(_) = query_get_data::<u64>(
    //             &manager.context,
    //             &manager.queries[last_span.end_query],
    //             false,
    //         ) {
    //             return true;
    //         } else {
    //             return false;
    //         }
    //     }

    //     true
    // }
}

pub struct GpuSpanGuard {
    tracy_span: Option<tracy_client::GpuSpan>,
    start_query: usize,
    end_query: usize,
}

impl GpuSpanGuard {
    fn to_pending(&mut self) -> PendingGpuSpan {
        PendingGpuSpan {
            tracy_span: self.tracy_span.take().unwrap(),
            start_query: self.start_query,
            end_query: self.end_query,
        }
    }
}

struct PendingGpuSpan {
    tracy_span: tracy_client::GpuSpan,
    start_query: usize,
    end_query: usize,
}

impl Drop for GpuSpanGuard {
    fn drop(&mut self) {
        Renderer::instance().timestamps.end_span(self.to_pending());
    }
}

#[profiling::function]
fn wait_for_query(context: &d3d11::DeviceContext, query: &d3d11::Query) {
    context.flush();
    while !context.is_query_ready(query) {
        std::thread::yield_now();
    }
}

fn create_query(device: &d3d11::Device, ty: QueryType) -> anyhow::Result<d3d11::Query> {
    Ok(device.create_query(&d3d11::query::QueryDesc {
        query: ty,
        misc_flags: 0,
    })?)
}

#[profiling::function]
fn query_get_data_blocking<T: Sized + Default>(
    context: &d3d11::DeviceContext,
    query: &d3d11::Query,
) -> T {
    loop {
        if let GetDataResult::Ok(data) = unsafe { context.get_data(query, false) } {
            return data;
        } else {
            std::thread::yield_now();
        }
    }
}

#[macro_export]
macro_rules! gpu_span {
    () => {
        #[cfg(feature = "tracy")]
        let _gpu_timespan = $crate::Renderer::instance()
            .timestamps
            .begin_span(tracy_client::span_location!());
    };
    ($name:expr) => {
        #[cfg(feature = "tracy")]
        let _gpu_timespan = $crate::Renderer::instance()
            .timestamps
            .begin_span(tracy_client::span_location!($name));
    };
}
