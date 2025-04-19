use deimos_data::tfx::PipelineState;

use super::command_list::CommandList;

pub struct GpuState {
    tfx_state: PipelineState,
    current_blend_state: usize,
    current_depth_state: usize,
    current_rasterizer_state: usize,
    current_depth_bias: usize,
    current_input_layout: usize,
    current_input_topology: usize,

    state_vs: GpuStageState,
    state_ps: GpuStageState,
    state_cs: GpuStageState,
    viewports: [d3d11::Viewport; 8],
    rtvs: [Option<d3d11::RenderTargetView>; 8],
    dsv: Option<d3d11::DepthStencilView>,
}

/// Represents the state of a GPU stage (vertex, fragment, compute, etc.)
struct GpuStageState {
    cbuffers: [Option<d3d11::Buffer>; d3d11::DeviceContext::CONSTANT_BUFFER_SLOT_COUNT],
    samplers: [Option<d3d11::SamplerState>; d3d11::DeviceContext::SAMPLER_SLOT_COUNT],
    srvs: [Option<d3d11::ShaderResourceView>; d3d11::DeviceContext::SHADER_RESOURCE_SLOT_COUNT],
}

impl GpuState {
    #[profiling::function]
    pub fn backup(cmd: &CommandList) -> Self {
        let (rtvs, dsv) = cmd.output_merger_get_render_targets();
        GpuState {
            tfx_state: cmd.state,
            current_blend_state: cmd.current_blend_state,
            current_depth_state: cmd.current_depth_state,
            current_rasterizer_state: cmd.current_rasterizer_state,
            current_depth_bias: cmd.current_depth_bias,
            current_input_layout: cmd.current_input_layout,
            current_input_topology: cmd.current_input_topology,
            state_vs: GpuStageState {
                cbuffers: cmd.vertex_get_constant_buffers(),
                samplers: cmd.vertex_get_samplers(),
                srvs: cmd.vertex_get_shader_resources(),
            },
            state_ps: GpuStageState {
                cbuffers: cmd.pixel_get_constant_buffers(),
                samplers: cmd.pixel_get_samplers(),
                srvs: cmd.pixel_get_shader_resources(),
            },
            state_cs: GpuStageState {
                cbuffers: cmd.compute_get_constant_buffers(),
                samplers: cmd.compute_get_samplers(),
                srvs: cmd.compute_get_shader_resources(),
            },
            viewports: cmd.rasterizer_get_viewports(),
            rtvs,
            dsv,
        }
    }

    #[profiling::function]
    pub fn restore(&self, cmd: &mut CommandList) {
        cmd.state = self.tfx_state;
        cmd.current_blend_state = self.current_blend_state;
        cmd.current_depth_state = self.current_depth_state;
        cmd.current_rasterizer_state = self.current_rasterizer_state;
        cmd.current_depth_bias = self.current_depth_bias;
        cmd.current_input_layout = self.current_input_layout;
        cmd.current_input_topology = self.current_input_topology;
        cmd.vertex_set_constant_buffers(0, &self.state_vs.cbuffers);
        cmd.vertex_set_samplers(0, &self.state_vs.samplers);
        cmd.vertex_set_shader_resources(0, &self.state_vs.srvs);
        cmd.pixel_set_constant_buffers(0, &self.state_ps.cbuffers);
        cmd.pixel_set_samplers(0, &self.state_ps.samplers);
        cmd.pixel_set_shader_resources(0, &self.state_ps.srvs);
        cmd.compute_set_constant_buffers(0, &self.state_cs.cbuffers);
        cmd.compute_set_samplers(0, &self.state_cs.samplers);
        cmd.compute_set_shader_resources(0, &self.state_cs.srvs);
        cmd.rasterizer_set_viewports(&self.viewports);
        cmd.output_merger_set_render_targets(&self.rtvs, self.dsv.as_ref());
        cmd.flush_states();
    }
}
