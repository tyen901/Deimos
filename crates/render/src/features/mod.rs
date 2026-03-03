use std::any::Any;

// pub mod rigid_model;
mod shared;
pub mod static_geometry;
pub mod terrain_patches;

use deimos_data::tfx::{RenderStage, features::dynamic::RenderStageSubscription};

use crate::{
    gpu::command_list::CommandList,
    renderer::{
        Renderer,
        packet::{RenderPerViewNode, SubmitNode, SubmitNodeContainer},
    },
    visibility::{ViewVisibility, frustum::Frustum},
};

pub trait FeatureRenderer: Send {
    fn extract(&mut self, renderer: &Renderer, view_node: &RenderPerViewNode);

    fn prepare(&mut self, renderer: &Renderer, view_node: &RenderPerViewNode);

    fn submit(
        &self,
        cmd: &mut CommandList,
        stage: RenderStage,
        view_node: &RenderPerViewNode,
        submit_key: u64,
    );

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

    fn populate_submit_node_blocks(
        &self,
        renderer: &Renderer,
        view_node: usize,
        visibility: &ViewVisibility,
        submit_node_blocks: &mut SubmitNodeContainer,
    );

    fn subscribed_stages(&self) -> RenderStageSubscription;

    /// Returns true if the feature renderer has finished loading any dependencies (techniques, buffers, etc)
    fn is_loaded(&self) -> bool {
        true
    }
}
