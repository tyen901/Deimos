use std::sync::Arc;

use deimos_data::tfx::{
    FixedFunctionState, PrimitiveType, RenderStage,
    features::{decals::SDecalCollection, dynamic::RenderStageSubscription},
    geometry::AxisAlignedBBox,
};

use crate::{
    asset::{AssetManager, Handle, vertex_buffer::VertexBuffer},
    features::FeatureRenderer,
    gpu::command_list::{CommandList, DepthMode},
    renderer::{Renderer, packet::SubmitNode},
    tfx::technique::Technique,
};

pub struct DecalCollectionRenderer {
    sets: Vec<DecalSet>,
    vb0: Handle<VertexBuffer>,
    vb1: Handle<VertexBuffer>,
    bounds: AxisAlignedBBox,
    render_stage: RenderStage,
}

pub struct DecalSet {
    pub bounds: AxisAlignedBBox,
    /// Result of the occlusion test
    pub visible: bool,
    pub technique: Handle<Technique>,
    pub start: u16,
    pub count: u16,
}

impl DecalCollectionRenderer {
    // #[profiling::function]
    pub fn load(renderer: &Renderer, collection: SDecalCollection) -> anyhow::Result<Box<Self>> {
        let vb0 = renderer.asset_manager.load(collection.vb0);
        let vb1 = renderer.asset_manager.load(collection.vb1);

        let sets = collection
            .decals
            .into_iter()
            .map(|set| {
                let r = (set.start as usize)..((set.start + set.count) as usize);
                DecalSet {
                    // TODO(cohae): Out of bounds error here
                    bounds: collection.bounds, //collection.decal_bounds.bounds[r].iter().map(|b| b.bb).sum(),
                    visible: true,
                    technique: renderer.asset_manager.load(set.technique),
                    start: set.start,
                    count: set.count,
                }
            })
            .collect();

        Ok(Box::new(Self {
            sets,
            vb0,
            vb1,
            bounds: collection.bounds,
            render_stage: collection.render_stage,
        }))
    }
}

// #[profiling::all_functions]
impl FeatureRenderer for DecalCollectionRenderer {
    fn populate_submit_node_blocks(
        &self,
        renderer: &crate::renderer::Renderer,
        view_node: usize,
        visibility: &crate::visibility::ViewVisibility,
        submit_node_blocks: &mut crate::renderer::packet::SubmitNodeContainer,
    ) {
        submit_node_blocks.broadcast(self.subscribed_stages(), SubmitNode { view_node, key: 0 });
    }

    // fn visibility_test(&mut self, _view_index: usize, view: &dyn OpaqueView) -> bool {
    //     if !view.is_visible(&self.bounds) {
    //         return false;
    //     }

    //     // let mut any_visible = false;
    //     // for set in &mut self.sets {
    //     //     let is_visible = camera.culling_frustum.aabb_intersecting(&set.bounds);
    //     //     if is_visible {
    //     //         set.visible = true;
    //     //         any_visible = true;
    //     //     } else {
    //     //         set.visible = false;
    //     //     }
    //     // }

    //     // any_visible
    //     true
    // }

    // fn prepare(
    //     &mut self,
    //     _renderer: &Renderer,
    //     _view_index: usize,
    //     _extracted_data: &dyn std::any::Any,
    // ) {
    // }

    // fn submit(
    //     &self,
    //     cmd: &mut CommandList,
    //     _view_index: usize,
    //     _stage: alkahest_data::tfx::RenderStage,
    // ) {
    //     let Some((vb0, vb1)) = self.vb0.get().zip(self.vb1.get()) else {
    //         return;
    //     };

    //     cmd.state_override = PipelineState::new(None, None, Some(1), None);
    //     cmd.set_depth_mode(DepthMode::Forward);
    //     Renderer::instance().globals.scopes.decal.bind(cmd).unwrap();

    //     cmd.input_assembler_set_vertex_buffers(
    //         0,
    //         &[Some(&vb1.buffer), Some(&vb0.buffer)],
    //         Some(&[vb1.stride, vb0.stride]),
    //         Some(&[0, 0]),
    //     )
    //     .unwrap();
    //     cmd.set_input_layout(17);
    //     cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::Triangles);
    //     for set in self.sets.iter().filter(|s| s.visible) {
    //         let Some(t) = set.technique.get() else {
    //             continue;
    //         };
    //         t.bind(cmd).unwrap();
    //         cmd.draw_instanced(36_u32, set.count as u32, 0, set.start as u32);
    //     }

    //     cmd.set_depth_mode(DepthMode::Reverse);
    //     cmd.state_override.reset();
    // }

    // fn submit_parallel(
    //     &self,
    //     renderer: &Arc<Renderer>,
    //     _view_index: usize,
    //     set: CommandListSetId,
    //     _stage: alkahest_data::tfx::RenderStage,
    //     jobs: &mut Vec<alkahest_core::job::potassium::JobHandle>,
    // ) {
    //     let renderer = renderer.clone();

    //     let self_p = &raw const *self as u64;
    //     let pool = renderer.cmd_pool.clone();
    //     // TODO(cohae): There's opportunity for optimization here. These jobs are currently quite coarse
    //     let job = alkahest_core::job::SCHEDULER
    //         .job_builder("decals_render")
    //         .spawn(move || {
    //             let self_ref = unsafe { &*(self_p as *const Self) };
    //             let cmd = pool.get_command_list(set);

    //             cmd.state_override = PipelineState::new(None, None, Some(1), None);
    //             cmd.set_depth_mode(DepthMode::Forward);

    //             let Some((vb0, vb1)) = self_ref.vb0.get().zip(self_ref.vb1.get()) else {
    //                 return;
    //             };

    //             renderer.globals.scopes.decal.bind(cmd).unwrap();

    //             cmd.input_assembler_set_vertex_buffers(
    //                 0,
    //                 &[Some(&vb1.buffer), Some(&vb0.buffer)],
    //                 Some(&[vb1.stride, vb0.stride]),
    //                 Some(&[0, 0]),
    //             )
    //             .unwrap();
    //             cmd.set_input_layout(17);
    //             cmd.set_input_topology(alkahest_data::tfx::PrimitiveType::Triangles);
    //             for set in self_ref.sets.iter().filter(|s| s.visible) {
    //                 let Some(t) = set.technique.get() else {
    //                     continue;
    //                 };
    //                 t.bind(cmd).unwrap();
    //                 cmd.draw_instanced(36_u32, set.count as u32, 0, set.start as u32);
    //             }

    //             cmd.set_depth_mode(DepthMode::Reverse);
    //             cmd.state_override.reset();
    //         });
    //     jobs.push(job);
    // }

    fn submit(
        &self,
        cmd: &mut CommandList,
        stage: deimos_data::tfx::RenderStage,
        frame_node: &crate::renderer::packet::RenderPerFrameNode,
        view_node: &crate::renderer::packet::RenderPerViewNode,
        submit_key: u64,
    ) {
        let Some((vb0, vb1)) = self.vb0.get().zip(self.vb1.get()) else {
            return;
        };

        cmd.set_ffstate_override(FixedFunctionState::new(None, None, Some(1), None));
        cmd.set_depth_mode(DepthMode::Forward);

        // cmd.input_assembler_set_vertex_buffers(
        //     0,
        //     &[Some(&vb1.buffer), Some(&vb0.buffer)],
        //     Some(&[vb1.stride, vb0.stride]),
        //     Some(&[0, 0]),
        // )
        // .unwrap();
        cmd.ia_set_vertex_buffers(0, &[vb1.view(), vb0.view()]);

        cmd.set_input_layout(17);
        cmd.set_input_topology(PrimitiveType::Triangles);
        for set in self.sets.iter().filter(|s| s.visible) {
            let Some(t) = set.technique.get() else {
                continue;
            };
            t.bind(cmd);
            cmd.draw_instanced(0..36, (set.start as u32..(set.start + set.count) as u32));
        }

        cmd.set_depth_mode(DepthMode::Reverse);
        cmd.reset_ffstate_override();
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        // TODO(cohae): Dynamic decals never actually use the decals stage, they should actually use GenerateGbuffer instead
        if self.render_stage == RenderStage::GenerateGbuffer {
            RenderStage::Decals.into()
        } else {
            self.render_stage.into()
        }
    }
}
