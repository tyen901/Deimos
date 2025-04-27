use std::{ops::Deref, sync::Arc};

use deimos_data::tfx::{PipelineState, PrimitiveType, ShaderStage};
use tiger_pkg::TagHash;

use super::{global_state, Gpu};

pub struct CommandList {
    parent: Arc<Gpu>,
    pub(crate) context: d3d11::DeviceContext,
    annotation: d3d11::UserDefinedAnnotation,

    pub state: PipelineState,
    pub state_override: PipelineState,
    pub(super) current_blend_state: usize,
    pub(super) current_depth_state: usize,
    pub(super) current_rasterizer_state: usize,
    pub(super) current_depth_bias: usize,
    pub(super) current_input_layout: usize,
    pub(super) current_input_topology: usize,
    pub(super) current_stencil_ref: u32,
    pub(super) depth_mode: DepthMode,
    pub(super) bound_technique: TagHash,

    /// Uncommitted resources
    resources: [StageResources; 6],
}

impl Deref for CommandList {
    type Target = d3d11::DeviceContext;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl CommandList {
    pub fn new(gpu: &Arc<Gpu>) -> Self {
        let context = gpu
            .create_deferred_context()
            .expect("Failed to create deferred context");

        Self::from_device_context(gpu, context)
    }

    pub fn from_device_context(gpu: &Arc<Gpu>, context: d3d11::DeviceContext) -> Self {
        CommandList {
            parent: gpu.clone(),
            annotation: context.get_user_defined_annotation(),
            context,
            state: PipelineState::default(),
            state_override: PipelineState::default(),

            current_blend_state: usize::MAX,
            current_depth_state: usize::MAX,
            current_input_layout: usize::MAX,
            current_rasterizer_state: usize::MAX,
            current_depth_bias: usize::MAX,
            current_input_topology: usize::MAX,
            current_stencil_ref: 0,
            depth_mode: DepthMode::Reverse,
            bound_technique: TagHash::NONE,

            resources: std::array::from_fn(|_| StageResources::default()),
        }
    }

    pub fn gpu(&self) -> &Gpu {
        &self.parent
    }

    pub fn global_states(&self) -> &global_state::RenderStates {
        &self.gpu().global_states
    }
}

// GPU state management
impl CommandList {
    fn reset_states(&mut self) {
        // Reset current states
        self.current_blend_state = usize::MAX;
        self.current_depth_state = usize::MAX;
        self.current_input_layout = usize::MAX;
        self.current_rasterizer_state = usize::MAX;
        self.current_depth_bias = usize::MAX;
        self.current_input_topology = usize::MAX;
        self.bound_technique = TagHash::NONE;
    }

    pub fn flush_states(&mut self) {
        self.reset_states();
        if let Some(blend) = self.state.blend_state() {
            self.set_blend_state(blend);
        }
        if let Some(depth_stencil) = self.state.depth_stencil_state() {
            self.set_depth_stencil_state(depth_stencil);
        }
        if let Some(rasterizer) = self.state.rasterizer_state() {
            self.set_rasterizer_state(rasterizer);
        }
        if let Some(depth_bias) = self.state.depth_bias_state() {
            self.set_depth_bias(depth_bias);
        }
    }

    pub fn set_blend_state(&mut self, index: usize) {
        if self.current_blend_state != index {
            self.output_merger_set_blend_state(
                &self.global_states().blend_states[index],
                Some(&[1.0, 1.0, 1.0, 1.0]),
                0xFFFFFFFF,
            );
            self.current_blend_state = index;
        }
    }

    pub fn set_depth_mode(&mut self, mode: DepthMode) {
        if self.depth_mode != mode {
            self.depth_mode = mode;
            let mut d = usize::MAX;
            std::mem::swap(&mut d, &mut self.current_depth_state);
            self.set_depth_stencil_state(d);
        }
    }

    pub fn set_depth_stencil_state(&mut self, index: usize) {
        if self.current_depth_state != index {
            let states = &self.global_states().depth_stencil_states[index];
            self.output_merger_set_depth_stencil_state(
                match self.depth_mode {
                    DepthMode::Reverse => &states.0,
                    DepthMode::Forward => &states.1,
                },
                self.current_stencil_ref,
            );
            self.current_depth_state = index;
        }
    }

    pub fn set_stencil_ref(&mut self, ref_value: u32) {
        if self.current_stencil_ref != ref_value {
            self.current_stencil_ref = ref_value;
            let d = self.current_depth_state;
            self.current_depth_state = usize::MAX;
            self.set_depth_stencil_state(d);
        }
    }

    pub fn set_rasterizer_state(&mut self, index: usize) {
        if self.current_rasterizer_state != index {
            let depth_bias = self.current_depth_bias;
            if index < 9 && depth_bias < 9 {
                self.rasterizer_set_state(
                    &self.global_states().rasterizer_states[depth_bias][index],
                );
            }
            self.current_rasterizer_state = index;
        }
    }

    pub fn set_depth_bias(&mut self, index: usize) {
        if self.current_depth_bias != index {
            let rasterizer_state = self.current_rasterizer_state;
            if index < 9 && rasterizer_state < 9 {
                self.rasterizer_set_state(
                    &self.global_states().rasterizer_states[index][rasterizer_state],
                );
            }
            self.current_depth_bias = index;
        }
    }

    /// Returns true if the given technique is already bound
    pub fn set_bound_technique(&mut self, index: TagHash) -> bool {
        if self.bound_technique != index {
            self.bound_technique = index;
            false
        } else {
            true
        }
    }

    pub fn set_input_layout(&mut self, index: usize) {
        if self.current_input_layout != index {
            self.context.input_assembler_set_input_layout(
                self.global_states().input_layouts[index].as_ref(),
            );
            self.current_input_layout = index;
        }
    }

    /// Applies a one-time state override
    pub fn apply_state(&mut self, states: &PipelineState) {
        if let Some(u) = states.blend_state() {
            self.set_blend_state(u);
        }
        if let Some(u) = states.depth_stencil_state() {
            self.set_depth_stencil_state(u);
        }
        if let Some(u) = states.rasterizer_state() {
            self.set_rasterizer_state(u);
        }
        if let Some(u) = states.depth_bias_state() {
            self.set_depth_bias(u);
        }
    }

    #[deprecated(note = "This method bypasses the pipeline state, use set_input_layout instead")]
    pub fn input_assembler_set_input_layout(&self, layout: &d3d11::InputLayout) {
        self.context.input_assembler_set_input_layout(layout);
    }

    pub fn set_input_topology(&mut self, topology: PrimitiveType) {
        if self.current_input_topology != topology as usize {
            self.context
                .input_assembler_set_primitive_topology(match topology {
                    PrimitiveType::PointList => d3d11::PrimitiveTopology::PointList,
                    PrimitiveType::LineList => d3d11::PrimitiveTopology::LineList,
                    PrimitiveType::LineStrip => d3d11::PrimitiveTopology::LineStrip,
                    PrimitiveType::Triangles => d3d11::PrimitiveTopology::TriangleList,
                    PrimitiveType::TriangleStrip => d3d11::PrimitiveTopology::TriangleStrip,
                });
            self.current_input_topology = topology as usize;
        }
    }

    #[deprecated(note = "This method bypasses the pipeline state, use set_input_topology instead")]
    pub fn input_assembler_set_primitive_topology(&self, topology: d3d11::PrimitiveTopology) {
        self.context
            .input_assembler_set_primitive_topology(topology);
    }

    // pub fn input_assembler_set_primitive_topology_tfx(&self, topology: PrimitiveType) {
    //     self.input_assembler_set_primitive_topology(match topology {
    //         PrimitiveType::PointList => d3d11::PrimitiveTopology::PointList,
    //         PrimitiveType::LineList => d3d11::PrimitiveTopology::LineList,
    //         PrimitiveType::LineStrip => d3d11::PrimitiveTopology::LineStrip,
    //         PrimitiveType::Triangles => d3d11::PrimitiveTopology::TriangleList,
    //         PrimitiveType::TriangleStrip => d3d11::PrimitiveTopology::TriangleStrip,
    //     });
    // }

    // pub fn set_input_layout(&self, id: usize) {
    //     if let Some(layout) = self
    //         .parent
    //         .global_states
    //         .input_layouts
    //         .get(id)
    //         .and_then(|l| l.clone())
    //     {
    //         self.input_assembler_set_input_layout(&layout);
    //     }
    // }
}

// Resource staging
impl CommandList {
    pub fn clear_resources(&mut self) {
        for r in &mut self.resources {
            r.clear();
        }
    }

    #[profiling::function]
    pub fn commit_resources(&mut self, stages: &[ShaderStage]) {
        for stage in stages {
            self.stage_resources(*stage).commit(self, *stage);
        }
    }

    #[inline(always)]
    pub(super) fn stage_resources(&self, stage: ShaderStage) -> &StageResources {
        &self.resources[stage as usize - 1]
    }

    #[inline(always)]
    pub(super) fn stage_resources_mut(&mut self, stage: ShaderStage) -> &mut StageResources {
        &mut self.resources[stage as usize - 1]
    }

    #[inline(always)]
    pub fn set_shader_resource(
        &mut self,
        stage: ShaderStage,
        slot: usize,
        srv: impl Into<Option<d3d11::ShaderResourceView>>,
    ) {
        if slot < d3d11::DeviceContext::SHADER_RESOURCE_SLOT_COUNT {
            self.stage_resources_mut(stage).srvs[slot] = srv.into();
        }
    }

    #[inline(always)]
    pub fn set_sampler(
        &mut self,
        stage: ShaderStage,
        slot: usize,
        sampler: impl Into<Option<d3d11::SamplerState>>,
    ) {
        if slot < d3d11::DeviceContext::SAMPLER_SLOT_COUNT {
            self.stage_resources_mut(stage).samplers[slot] = sampler.into();
        }
    }

    #[inline(always)]
    pub fn set_constant_buffer(
        &mut self,
        stage: ShaderStage,
        slot: usize,
        cbuffer: impl Into<Option<d3d11::Buffer>>,
    ) {
        if slot < d3d11::DeviceContext::CONSTANT_BUFFER_SLOT_COUNT {
            self.stage_resources_mut(stage).cbuffers[slot] = cbuffer.into();
        }
    }
}

impl CommandList {
    pub fn set_marker(&self, name: impl AsRef<str>) {
        self.annotation.set_marker(name.as_ref());
    }

    pub fn begin_event_span(&self, name: impl AsRef<str>) -> GpuEventGuard {
        self.annotation.begin_event(name.as_ref());
        GpuEventGuard {
            annotation: self.annotation.clone(),
        }
    }
}

pub struct GpuEventGuard {
    annotation: d3d11::UserDefinedAnnotation,
}

impl GpuEventGuard {
    pub fn scoped<F: FnOnce()>(self, f: F) {
        f();
    }
}

impl Drop for GpuEventGuard {
    fn drop(&mut self) {
        self.annotation.end_event();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthMode {
    /// Commonly used for shadow maps and decals
    Forward,
    /// Used by default
    Reverse,
}

#[macro_export]
macro_rules! cmd_event_span {
    ($cmd:ident, $name:expr) => {
        profiling::scope!(&format!("cmd-{}", $name));
        let _gpu_span = $cmd.begin_event_span($name);
    };
}

pub(super) struct StageResources {
    pub cbuffers: [Option<d3d11::Buffer>; d3d11::DeviceContext::CONSTANT_BUFFER_SLOT_COUNT],
    pub samplers: [Option<d3d11::SamplerState>; d3d11::DeviceContext::SAMPLER_SLOT_COUNT],
    pub srvs: [Option<d3d11::ShaderResourceView>; d3d11::DeviceContext::SHADER_RESOURCE_SLOT_COUNT],
}

impl Default for StageResources {
    fn default() -> Self {
        Self {
            cbuffers: Default::default(),
            samplers: Default::default(),
            srvs: [const { None }; d3d11::DeviceContext::SHADER_RESOURCE_SLOT_COUNT],
        }
    }
}

impl StageResources {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn commit(&self, ctx: &d3d11::DeviceContext, stage: ShaderStage) {
        let bind_srvs = match stage {
            ShaderStage::Pixel => d3d11::DeviceContext::pixel_set_shader_resources,
            ShaderStage::Vertex => d3d11::DeviceContext::vertex_set_shader_resources,
            ShaderStage::Geometry => d3d11::DeviceContext::geometry_set_shader_resources,
            ShaderStage::Hull => d3d11::DeviceContext::hull_set_shader_resources,
            ShaderStage::Compute => d3d11::DeviceContext::compute_set_shader_resources,
            ShaderStage::Domain => d3d11::DeviceContext::domain_set_shader_resources,
        };
        let bind_samplers = match stage {
            ShaderStage::Pixel => d3d11::DeviceContext::pixel_set_samplers,
            ShaderStage::Vertex => d3d11::DeviceContext::vertex_set_samplers,
            ShaderStage::Geometry => d3d11::DeviceContext::geometry_set_samplers,
            ShaderStage::Hull => d3d11::DeviceContext::hull_set_samplers,
            ShaderStage::Compute => d3d11::DeviceContext::compute_set_samplers,
            ShaderStage::Domain => d3d11::DeviceContext::domain_set_samplers,
        };
        let bind_cbuffers = match stage {
            ShaderStage::Pixel => d3d11::DeviceContext::pixel_set_constant_buffers,
            ShaderStage::Vertex => d3d11::DeviceContext::vertex_set_constant_buffers,
            ShaderStage::Geometry => d3d11::DeviceContext::geometry_set_constant_buffers,
            ShaderStage::Hull => d3d11::DeviceContext::hull_set_constant_buffers,
            ShaderStage::Compute => d3d11::DeviceContext::compute_set_constant_buffers,
            ShaderStage::Domain => d3d11::DeviceContext::domain_set_constant_buffers,
        };

        // Sparsely bind SRVs and samplers. Most of the time only a small portion of the slots are used, so we can optimize the binding process by skipping the unused slots.
        if let Some(srv_start) = self.srvs.iter().position(|s| s.is_some()) {
            let srv_end = self.srvs.iter().rposition(|s| s.is_some()).unwrap();
            bind_srvs(ctx, srv_start as u32, &self.srvs[srv_start..=srv_end]);
        }
        if let Some(sampler_start) = self.samplers.iter().position(|s| s.is_some()) {
            let sampler_end = self.samplers.iter().rposition(|s| s.is_some()).unwrap();
            bind_samplers(
                ctx,
                sampler_start as u32,
                &self.samplers[sampler_start..=sampler_end],
            );
        }
        // cohae: cb0, cb12 and cb13 are practically always bound, so we don't gain anything from doing a sparse bind for cbuffers
        bind_cbuffers(ctx, 0, &self.cbuffers);
    }
}
