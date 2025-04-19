use std::time::Duration;
use std::{sync::Arc, thread::JoinHandle};

use crossbeam::channel::{unbounded, Sender};
use deimos_core::convar::ConVars;
use deimos_data::tfx::{FeatureRendererSubscription, RenderStage};
use parking_lot::{Condvar, Mutex};

use crate::gpu::command_list::CommandList;
use crate::gpu::state::GpuState;

use super::Renderer;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

impl Renderer {
    pub fn submit_stage_range(
        &self,
        cmd: &mut CommandList,
        frame_node_range: std::ops::Range<usize>,
        stage: RenderStage,
        mut features: FeatureRendererSubscription,
    ) {
        features = self.active_feature_renderers.load().intersection(features);
        if features.is_empty() {
            return;
        }
        profiling::scope!("submit_stage", &format!("stage={stage:?}"));

        for obj in self.frame_packet.read().frame_nodes[frame_node_range].iter() {
            if let Some(render_object) = self
                .objects
                .read()
                .get(obj.render_object_handle.into())
                .filter(|p| p.stages.is_subscribed(stage) && features.is_subscribed(p.feature_type))
            {
                render_object.submit(cmd, stage);
            }
        }
    }
    pub fn submit_stage(
        &self,
        cmd: &mut CommandList,
        stage: RenderStage,
        features: FeatureRendererSubscription,
    ) {
        profiling::scope!(
            "submit_stage",
            &format!("stage={stage:?}, features={features:?}")
        );
        self.submit_stage_range(
            cmd,
            0..self.frame_packet.read().frame_nodes.len(),
            stage,
            features,
        );
    }

    pub fn submit_stage_multi(&self, cmd: &mut CommandList, stage: RenderStage, job_count: usize) {
        if !ConVars::get_flag("render.threaded_submit") {
            self.submit_stage(cmd, stage, FeatureRendererSubscription::all());
            return;
        }

        let job_count = job_count.max(1);
        profiling::scope!(
            "submit_stage_multi",
            &format!("stage={stage:?} jobs={job_count}")
        );

        let gpu_state = GpuState::backup(cmd);

        let mut jobs = Vec::new();

        let node_count = self.frame_packet.read().frame_nodes.len();
        for i in 0..job_count {
            let node_start = (i * node_count) / job_count;
            let node_end = ((i + 1) * node_count) / job_count;

            let mut job_cmd = self.gpu.create_command_list();
            gpu_state.restore(&mut job_cmd);
            let j = self.submit_jobs.submit_job(SubmitJobDesc {
                node_range: node_start..node_end,
                stage,
                cmd: job_cmd,
                index: i,
            });
            jobs.push(j);
        }

        for j in jobs {
            if let Some(command_list) = self.submit_jobs.await_job(j) {
                // We only need to restore state on the last job
                profiling::scope!("execute_command_list", &format!("job={:?}", j));
                cmd.execute_command_list(&command_list.finish_command_list(false).unwrap(), false);
            } else {
                error!("Submit job {j:?} got dropped?");
            }
        }

        gpu_state.restore(cmd);
    }
}

pub struct SubmitJobDesc {
    pub node_range: std::ops::Range<usize>,
    pub stage: RenderStage,
    pub cmd: CommandList,
    pub index: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SubmitJobId(usize);

pub struct JobResult {
    pub cmd: CommandList,
}

pub struct SubmitJobManager {
    next_job_id: AtomicUsize,
    jobs: Arc<Mutex<HashMap<usize, Option<JobResult>>>>,
    condvar: Arc<Condvar>,
    sender: Sender<(usize, SubmitJobDesc)>,
    thread_handles: Vec<JoinHandle<()>>,
}

impl SubmitJobManager {
    pub fn new(thread_count: usize) -> Self {
        let (sender, receiver) = unbounded();
        let jobs = Arc::new(Mutex::new(HashMap::new()));
        let condvar = Arc::new(Condvar::new());
        let mut thread_handles = Vec::new();

        for i in 0..thread_count {
            let receiver = receiver.clone();
            let jobs = jobs.clone();
            let condvar = condvar.clone();

            let handle = std::thread::Builder::new()
                .name(format!("render_submit_{i}"))
                .spawn(move || {
                    while let Ok((job_id, job_desc)) = receiver.recv() {
                        let SubmitJobDesc {
                            node_range,
                            stage,
                            mut cmd,
                            index,
                        } = job_desc;
                        let _ = index;
                        profiling::scope!(
                            "threaded_submit_job",
                            &format!("job_id={job_id} index={index} stage={stage:?}")
                        );

                        let renderer = Renderer::instance();
                        renderer.submit_stage_range(
                            &mut cmd,
                            node_range,
                            stage,
                            FeatureRendererSubscription::all(),
                        );

                        let mut jobs = jobs.lock();
                        jobs.insert(job_id, Some(JobResult { cmd }));
                        condvar.notify_all();
                    }
                })
                .expect("Failed to spawn submit thread");

            thread_handles.push(handle);
        }

        Self {
            next_job_id: AtomicUsize::new(0),
            jobs,
            condvar,
            sender,
            thread_handles,
        }
    }

    pub fn submit_job(&self, job_desc: SubmitJobDesc) -> SubmitJobId {
        let job_id = self.next_job_id.fetch_add(1, Ordering::SeqCst);
        self.jobs.lock().insert(job_id, None);
        self.sender.send((job_id, job_desc)).unwrap();
        SubmitJobId(job_id)
    }

    pub fn await_job(&self, job_id: SubmitJobId) -> Option<CommandList> {
        let mut jobs = self.jobs.lock();
        if !jobs.contains_key(&job_id.0) {
            // Job id doesn't exist or has already been awaited
            return None;
        }

        loop {
            if self.thread_handles.iter().any(|c| c.is_finished()) {
                panic!("Submission thread died");
            }

            // Check if the job has been completed, if not, skip to the next iteration
            if jobs.get(&job_id.0).unwrap().is_some() {
                let res = jobs.remove(&job_id.0).unwrap().unwrap();
                return Some(res.cmd);
            }
            self.condvar.wait_for(&mut jobs, Duration::from_millis(100));
        }
    }
}

impl Drop for SubmitJobManager {
    fn drop(&mut self) {
        // Replace the sender with a dummy sender to signal the threads to exit
        self.sender = unbounded().0;
        for handle in self.thread_handles.drain(..) {
            handle.join().expect("Failed to join thread");
        }
    }
}
