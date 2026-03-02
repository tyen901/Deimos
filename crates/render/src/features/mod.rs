use std::any::Any;

pub mod rigid_model;
mod shared;
pub mod static_geometry;
pub mod terrain_patches;

use deimos_data::tfx::{RenderStage, features::dynamic::RenderStageSubscription};

use crate::{gpu::command_list::CommandList, renderer::Renderer};

pub trait FeatureRenderer {
    fn extract(&mut self, renderer: &Renderer, data: &dyn Any);

    fn prepare(&mut self, renderer: &Renderer);

    fn submit(&self, cmd: &mut CommandList, stage: RenderStage);

    // fn submit_parallel(
    //     &self,
    //     renderer: &Arc<Renderer>,
    //     view_index: usize,
    //     set: CommandListSetId,
    //     stage: RenderStage,
    //     jobs: &mut Vec<JobHandle>,
    // ) {
    //     _ = (renderer, view_index, set, stage, jobs);
    // }

    fn dyn_clone(&self) -> Option<Box<dyn FeatureRenderer>> {
        None
    }

    fn subscribed_stages(&self) -> RenderStageSubscription;

    /// Returns true if the feature renderer has finished loading any dependencies (techniques, buffers, etc)
    fn is_loaded(&self) -> bool {
        true
    }
}
