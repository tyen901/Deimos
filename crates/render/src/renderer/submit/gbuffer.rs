use deimos_core::convar::ConVars;
use deimos_data::tfx::{
    FeatureRendererSubscription, PipelineState, RenderStage, TfxFeatureRenderer,
};

use crate::{
    cmd_event_span,
    gpu::command_list::{CommandList, DepthMode},
};

use super::Renderer;

impl Renderer {
    pub(super) fn submit_gbuffer_generation(&self, cmd: &mut CommandList) {
        profiling::scope!("submit_gbuffer_generation");

        let gpu = &self.gpu;

        self.gbuffers.clear(cmd);
        self.gbuffers.bind(cmd, self);

        {
            cmd_event_span!(cmd, "generate_gbuffer");

            if ConVars::get_flag("render.vao_buffer") {
                // if let Some(ao_vb) = self.ao_buffer.lock().as_ref().and_then(|h| h.get()) {
                //     cmd.vertex_set_shader_resources(0, &[ao_vb.srv.clone()]);
                // }
            }

            cmd.state = PipelineState::new(Some(0), Some(2), Some(2), Some(0));

            self.submit_stage_multi(cmd, RenderStage::GenerateGbuffer, 12);
        }

        {
            cmd_event_span!(cmd, "decals");

            self.gbuffers
                .depth_proxy
                .lock()
                .update(cmd, self.surfaces.get(self.gbuffers.depth));

            self.surfaces
                .copy(cmd, self.gbuffers.normal, self.gbuffers.normal_read);
            cmd.state = PipelineState::new(Some(8), Some(15), Some(2), Some(1));
            self.submit_stage(
                cmd,
                RenderStage::Decals,
                FeatureRendererSubscription::all_but(TfxFeatureRenderer::DynamicDecals),
            );

            // TODO(cohae): We should only reverse the depth mode for decals that we are inside of
            cmd.state_override = PipelineState::new(None, None, Some(1), None);
            cmd.set_depth_mode(DepthMode::Forward);
            self.submit_stage(
                cmd,
                RenderStage::Decals,
                FeatureRendererSubscription::DYNAMIC_DECALS,
            );
            cmd.set_depth_mode(DepthMode::Reverse);
            cmd.state_override.reset();

            {
                profiling::scope!("prepare/submit immediate geometry");
                self.immediate.lock().prepare(gpu);
                self.immediate.lock().submit(cmd);
            }
        }

        self.gbuffers
            .third_proxy
            .lock()
            .update(&cmd, self.surfaces.get(self.gbuffers.third));

        // TODO(cohae): Can we reduce boilerplate for these kinds of pipelines?
        if ConVars::get_flag("render.vertex_ao_workaround") {
            cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
            cmd.flush_states();
            cmd.vertex_set_shader(Some(&self.clear_ao_vs));
            cmd.pixel_set_shader(Some(&self.clear_ao_ps));
            cmd.set_input_topology(deimos_data::tfx::PrimitiveType::TriangleStrip);
            cmd.pixel_set_shader_resources(
                0,
                &[Some(self.gbuffers.third_proxy.lock().srv.clone())],
            );
            cmd.draw(4, 0);
        }

        // {
        //     cmd.state = PipelineState::new(Some(0), Some(2), Some(0), Some(0));
        //     let depth_half_surf = self.surfaces.get(self.gbuffers.depth_half);
        //     depth_half_surf.clear_depth(cmd, 0.0, 0);
        //     depth_half_surf.bind_single(cmd);
        //     let depth_full_surf = self.surfaces.get(self.gbuffers.depth);

        //     {
        //         let hdao = &mut self.externs.get_mut().hdao;
        //         hdao.unk60_source = self.gbuffers.depth_proxy.lock().srv.clone().into();
        //         hdao.unk70_dest_res = depth_half_surf.resolution_with_recip();
        //         hdao.unk80_source_res = depth_full_surf.resolution_with_recip();
        //     }

        //     self.execute_global_pipeline(
        //         cmd,
        //         &self.globals.pipelines.downsample_depth_buffer,
        //         "downsample_depth_buffer",
        //     );
        // }

        // self.submit_uber_depth_generation(cmd);
    }

    // fn submit_uber_depth_generation(&self, cmd: &mut CommandList) {
    //     cmd_event_span!(cmd, "submit_uber_depth_generation");

    //     {
    //         cmd_event_span!(cmd, "[uber_depth_default]");

    //         self.globals.pipelines.uber_depth_default.bind(cmd).unwrap();
    //         let (width, height) = self.surfaces.get(self.gbuffers.depth).resolution();
    //         cmd.dispatch(width.div_ceil(16), height.div_ceil(16), 1);
    //         cmd.compute_set_unordered_access_views(0, &[None, None, None, None], None);
    //     }

    //     cmd_event_span!(cmd, "[downsample_max_min_avg_no_swizzle]");
    //     self.externs.get_mut().downsample_texture_generic = DownsampleTextureGeneric {
    //         source: self.gbuffers.uber_depth_quarter.into(),
    //         resolution_dest: self
    //             .surfaces
    //             .get(self.gbuffers.uber_depth_eighth)
    //             .resolution_with_recip(),
    //         resolution_source: self
    //             .surfaces
    //             .get(self.gbuffers.uber_depth_quarter)
    //             .resolution_with_recip(),
    //     };
    //     self.bind_surfaces(cmd, &[self.gbuffers.uber_depth_eighth], None);
    //     cmd.state = PipelineState::new(Some(0), Some(0), Some(0), Some(0));
    //     self.execute_global_pipeline(
    //         cmd,
    //         &self.globals.pipelines.downsample_max_min_avg_no_swizzle,
    //         "downsample_max_min_avg_no_swizzle",
    //     );
    // }
}
