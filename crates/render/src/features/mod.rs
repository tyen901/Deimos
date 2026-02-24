use std::any::Any;

use deimos_data::tfx::features::dynamic::RenderStageSubscription;

use crate::renderer::Renderer;

pub trait FeatureRenderer {
    fn extract(&mut self, renderer: &Renderer, data: &dyn Any);

    fn prepare(&mut self, renderer: &Renderer);

    // fn submit(&self, cmd: &mut CommandList, view_index: usize, stage: RenderStage);

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
