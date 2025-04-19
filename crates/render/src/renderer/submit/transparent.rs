use deimos_data::tfx::{
    FeatureRendererSubscription, PipelineState, RenderStage, TfxFeatureRenderer,
};

use crate::{cmd_event_span, gpu::command_list::CommandList};

use super::Renderer;

impl Renderer {
    pub(super) fn submit_transparent(&self, cmd: &mut CommandList) {
        {
            {
                let ext = &mut self.externs.get_mut();
                ext.deferred.deferred_depth = self.gbuffers.depth_proxy.lock().srv.clone().into();
                ext.view
                    .derive_matrices(self.surfaces.get(self.shading_result).resolution());
            }
            self.globals.scopes.view.bind(cmd).unwrap();
            self.globals.scopes.transparent.bind(cmd).unwrap();
            self.globals.scopes.transparent_advanced.bind(cmd).unwrap();

            cmd_event_span!(cmd, "decals_additive");
            self.bind_surfaces(cmd, &[self.shading_result], Some(self.gbuffers.depth));

            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(1));
            cmd.flush_states();
            self.submit_stage(
                cmd,
                RenderStage::DecalsAdditive,
                FeatureRendererSubscription::all(),
            );
        }
        {
            cmd_event_span!(cmd, "transparents");

            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(1));
            self.submit_stage(
                cmd,
                RenderStage::Transparents,
                FeatureRendererSubscription::all_but(TfxFeatureRenderer::Water),
            );
        }

        {
            cmd_event_span!(cmd, "distortion");

            {
                let distortion = self.surfaces.get(self.lighting.distortion);
                let externs = &mut self.externs.get_mut();
                externs.view.derive_matrices(distortion.resolution());
                externs.deferred.deferred_depth = self.gbuffers.uber_depth_half.into();
            }
            self.globals.scopes.view.bind(cmd).unwrap();

            self.clear_surface(cmd, self.lighting.distortion, [0., 0., 0., 0.]);
            self.bind_surfaces(
                cmd,
                &[self.lighting.distortion],
                Some(self.gbuffers.depth_half),
            );

            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(1));
            cmd.flush_states();
            self.submit_stage(
                cmd,
                RenderStage::Distortion,
                FeatureRendererSubscription::all(),
            );
        }

        // Rebind full resolution depth buffer
        {
            let shading_result = self.surfaces.get(self.shading_result);
            let externs = &mut self.externs.get_mut();
            externs.view.derive_matrices(shading_result.resolution());
            externs.deferred.deferred_depth = self.gbuffers.depth.into();

            self.shading_result_read.lock().update(&cmd, shading_result);
        }
        self.globals.scopes.view.bind(cmd).unwrap();
    }
}
