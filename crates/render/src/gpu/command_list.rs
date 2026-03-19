use std::{ops::Deref, sync::Arc};

use d3d12::{CpuDescriptorHandle, GpuVirtualAddress};
use deimos_data::tfx::{FixedFunctionState, PrimitiveType, ShaderStage};
use smallvec::SmallVec;
use tiger_pkg::TagHash;

use crate::{
    gpu::{
        alloc::{descriptors::ResourceView, ring::UploadRing},
        native_command_list::NativeCommandList,
        render_target::{DepthBuffer, RenderTarget},
    },
    tfx::externs::LocalExternContainer,
};

use super::Gpu;

pub struct CommandList {
    parent: Arc<Gpu>,
    pub(crate) cmd: NativeCommandList,

    state: CommandListState,
    pub(super) current_blend_state: usize,
    pub(super) current_depth_state: usize,
    pub(super) current_rasterizer_state: usize,
    pub(super) current_depth_bias: usize,
    pub(super) current_input_layout: usize,
    // pub(super) current_input_topology: usize,
    pub(super) current_stencil_ref: u32,
    pub(super) bound_technique: TagHash,
    smart_rebind: bool,

    pub externs: LocalExternContainer,

    tag: Option<u64>,
}

impl Deref for CommandList {
    type Target = d3d12::GraphicsCommandList;
    fn deref(&self) -> &Self::Target {
        &self.cmd
    }
}

impl CommandList {
    // pub fn new(gpu: &Arc<Gpu>) -> Self {
    //     Self::from_native_command_list(
    //         gpu,
    //         NativeCommandList::new(&gpu.device).expect("Failed to create command list"),
    //     )
    // }

    pub fn from_native_command_list(gpu: &Arc<Gpu>, cmd: NativeCommandList) -> Self {
        cmd.set_descriptor_heaps(std::slice::from_ref(gpu.frame().descriptors.heap()));
        Self {
            externs: LocalExternContainer::new(gpu.extern_source.read().clone()),
            parent: gpu.clone(),
            cmd,
            state: CommandListState::default(),

            current_blend_state: usize::MAX,
            current_depth_state: usize::MAX,
            current_input_layout: usize::MAX,
            current_rasterizer_state: usize::MAX,
            current_depth_bias: usize::MAX,
            // current_input_topology: usize::MAX,
            current_stencil_ref: 0,
            smart_rebind: false,
            bound_technique: TagHash::NONE,

            tag: None,
        }
    }

    pub const fn with_tag(mut self, tag: u64) -> Self {
        self.tag = Some(tag);
        self
    }

    pub const fn tag(&self) -> Option<u64> {
        self.tag
    }

    pub fn into_inner(self) -> NativeCommandList {
        self.cmd
    }

    // pub fn new_sublist(&self) -> Self {
    //     let mut new = CommandList::from_device_context(
    //         &self.parent,
    //         self.parent
    //             .create_deferred_context()
    //             .expect("Failed to create deferred context"),
    //     );

    //     new.state = self.state;
    //     new.state_override = self.state_override;
    //     new.depth_mode = self.depth_mode;

    //     new
    // }

    pub const fn gpu(&self) -> &Arc<Gpu> {
        &self.parent
    }

    pub fn upload_ring(&self) -> &UploadRing {
        &self.parent.frame().upload
    }
}

// GPU state management
impl CommandList {
    const fn reset_states(&mut self) {
        // Reset current states
        self.current_blend_state = usize::MAX;
        self.current_depth_state = usize::MAX;
        self.current_input_layout = usize::MAX;
        self.current_rasterizer_state = usize::MAX;
        self.current_depth_bias = usize::MAX;
        // self.current_input_topology = usize::MAX;
        self.bound_technique = TagHash::NONE;
    }

    pub const fn flush_states(&mut self) {
        self.reset_states();
        if let Some(blend) = self.state.ffstate.blend_state() {
            self.set_blend_state(blend);
        }
        if let Some(depth_stencil) = self.state.ffstate.depth_stencil_state() {
            self.set_depth_stencil_state(depth_stencil);
        }
        if let Some(rasterizer) = self.state.ffstate.rasterizer_state() {
            self.set_rasterizer_state(rasterizer);
        }
        if let Some(depth_bias) = self.state.ffstate.depth_bias_state() {
            self.set_depth_bias(depth_bias);
        }
    }

    pub const fn set_blend_state(&mut self, index: usize) {
        self.current_blend_state = index;
    }

    pub fn set_depth_mode(&mut self, mode: DepthMode) {
        if self.state.depth_mode != mode {
            self.state.depth_mode = mode;
            let mut d = usize::MAX;
            std::mem::swap(&mut d, &mut self.current_depth_state);
            self.set_depth_stencil_state(d);
        }
    }

    pub const fn set_depth_stencil_state(&mut self, index: usize) {
        self.current_depth_state = index;
    }

    pub const fn set_stencil_ref(&mut self, ref_value: u32) {
        if self.current_stencil_ref != ref_value {
            self.current_stencil_ref = ref_value;
            let d = self.current_depth_state;
            self.current_depth_state = usize::MAX;
            self.set_depth_stencil_state(d);
        }
    }

    pub const fn set_rasterizer_state(&mut self, index: usize) {
        self.current_rasterizer_state = index;
    }

    pub const fn set_depth_bias(&mut self, index: usize) {
        self.current_depth_bias = index;
    }

    pub const fn set_input_layout(&mut self, index: usize) {
        self.current_input_layout = index;
    }

    pub const fn get_input_layout(&self) -> usize {
        self.current_input_layout
    }

    /// Applies a one-time fixed function state override
    pub const fn apply_ffstate(&mut self, states: &FixedFunctionState) {
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

    pub fn set_input_topology(&mut self, topology: PrimitiveType) {
        // if self.current_input_topology != topology as usize {
        self.cmd.ia_set_primitive_topology(match topology {
            PrimitiveType::PointList => d3d12::PrimitiveTopology::PointList,
            PrimitiveType::LineList => d3d12::PrimitiveTopology::LineList,
            PrimitiveType::LineStrip => d3d12::PrimitiveTopology::LineStrip,
            PrimitiveType::Triangles => d3d12::PrimitiveTopology::TriangleList,
            PrimitiveType::TriangleStrip => d3d12::PrimitiveTopology::TriangleStrip,
        });
        //     self.current_input_topology = topology as usize;
        // }
    }

    #[deprecated(note = "This method bypasses the pipeline state, use set_input_topology instead")]
    pub fn input_assembler_set_primitive_topology(&self, topology: d3d12::PrimitiveTopology) {
        self.cmd.ia_set_primitive_topology(topology);
    }

    pub fn om_set_render_targets(
        &mut self,
        render_target_descriptors: &[(CpuDescriptorHandle, d3d12::Format)],
        depth_stencil_descriptor: Option<(CpuDescriptorHandle, d3d12::Format)>,
    ) {
        self.state.output.rtvs = render_target_descriptors
            .iter()
            .copied()
            .map(|(rtv, _)| rtv)
            .collect();
        self.state.output.rtv_formats = render_target_descriptors
            .iter()
            .copied()
            .map(|(_, format)| format)
            .collect();
        self.state.output.dsv = depth_stencil_descriptor.unzip().0;
        self.state.output.dsv_format = depth_stencil_descriptor.unzip().1;

        self.cmd
            .om_set_render_targets(&self.state.output.rtvs, false, self.state.output.dsv);
    }

    pub fn set_render_targets(
        &mut self,
        render_target_descriptors: &[&RenderTarget],
        depth_stencil_descriptor: Option<&DepthBuffer>,
    ) {
        self.state.output.rtvs = render_target_descriptors
            .iter()
            .copied()
            .map(|rt| rt.cpu_handle())
            .collect();
        self.state.output.rtv_formats = render_target_descriptors
            .iter()
            .copied()
            .map(|rt| rt.output_format())
            .collect();
        self.state.output.dsv = depth_stencil_descriptor.map(|db| db.cpu_handle());
        self.state.output.dsv_format = depth_stencil_descriptor.map(|db| db.output_format());

        self.cmd
            .om_set_render_targets(&self.state.output.rtvs, false, self.state.output.dsv);
    }

    pub fn set_scissor_rects(&mut self, rects: &[d3d12::Rect]) {
        self.state.output.scissor_rects = rects.iter().cloned().collect();
        self.cmd.set_scissor_rects(rects);
    }

    pub fn set_viewports(&mut self, viewports: &[d3d12::Viewport]) {
        self.state.output.viewports = viewports.iter().cloned().collect();
        self.cmd.set_viewports(viewports);
    }

    pub const fn resources(&self, stage: ShaderStage) -> &StageResources {
        &self.state.resources[stage as usize - 1]
    }

    pub const fn resources_mut(&mut self, stage: ShaderStage) -> &mut StageResources {
        &mut self.state.resources[stage as usize - 1]
    }

    pub fn get_shader_resource_view(
        &mut self,
        stage: ShaderStage,
        slot: u32,
    ) -> Option<ResourceView> {
        self.resources(stage)
            .srvs
            .get(slot as usize)
            .cloned()
            .flatten()
    }

    pub fn get_shader_constant_buffer_view(
        &mut self,
        stage: ShaderStage,
        slot: u32,
    ) -> Option<d3d12::GpuVirtualAddress> {
        self.resources(stage)
            .cbvs
            .get(slot as usize)
            .cloned()
            .flatten()
    }

    pub fn set_shader_resource_view(
        &mut self,
        stage: ShaderStage,
        slot: u32,
        resource: Option<ResourceView>,
    ) {
        if let Some(slot_mut) = self.resources_mut(stage).srvs.get_mut(slot as usize) {
            *slot_mut = resource;
        }
    }

    pub fn set_shader_constant_buffer_view(
        &mut self,
        stage: ShaderStage,
        slot: u32,
        mut buffer_address: Option<d3d12::GpuVirtualAddress>,
    ) {
        if buffer_address == Some(d3d12::GpuVirtualAddress::NULL) {
            buffer_address = None;
        }

        if let Some(slot_mut) = self.resources_mut(stage).cbvs.get_mut(slot as usize) {
            *slot_mut = buffer_address;
        }
    }

    /// Rebinds techniques even if they are already bound
    pub const fn disable_smart_technique_binding(&mut self) {
        self.smart_rebind = false;
    }

    /// Skips rebinding techniques that are already bound
    pub const fn enable_smart_technique_binding(&mut self) {
        self.smart_rebind = true;
    }

    /// Returns true if the given technique is already bound and should not be rebound
    pub fn is_technique_smart_bound(&mut self, index: TagHash) -> bool {
        if self.bound_technique != index {
            self.bound_technique = index;
            false
        } else {
            self.smart_rebind
        }
    }

    #[profiling::function]
    pub fn restore_cmd_state(&mut self, new_state: &CommandListState) {
        let om = &new_state.output;
        self.cmd.om_set_render_targets(&om.rtvs, false, om.dsv);
        self.cmd.set_viewports(&om.viewports);
        self.cmd.set_scissor_rects(&om.scissor_rects);

        self.state = new_state.clone();
    }

    pub const fn cmd_state(&self) -> &CommandListState {
        &self.state
    }

    pub const fn set_ffstate(&mut self, ffstate: FixedFunctionState) {
        self.state.ffstate = ffstate;
    }

    pub const fn set_ffstate_override(&mut self, ffstate: FixedFunctionState) {
        self.state.ffstate_override = ffstate;
    }

    pub fn reset_ffstate_override(&mut self) {
        self.state.ffstate_override = FixedFunctionState::default();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthMode {
    /// Commonly used for shadow maps and decals
    Forward,
    /// Used by default
    Reverse,
}

#[derive(Default, Clone)]
pub struct StageResources {
    pub srvs: [Option<ResourceView>; 32],
    pub cbvs: [Option<d3d12::GpuVirtualAddress>; 16],
}

impl StageResources {
    pub fn get_shader_resource_view(&self, slot: u32) -> Option<ResourceView> {
        self.srvs.get(slot as usize).cloned().flatten()
    }

    pub fn get_shader_constant_buffer_view(&self, slot: u32) -> Option<d3d12::GpuVirtualAddress> {
        let va = self.cbvs.get(slot as usize).cloned().flatten();
        if va == Some(GpuVirtualAddress::NULL) {
            None
        } else {
            va
        }
    }
}

#[macro_export]
macro_rules! cmd_event_span {
    ($cmd:ident, $name:expr) => {
        profiling::scope!(&format!("cmd-{}", $name));
        let _gpu_span = $cmd.begin_event_span($name);
    };
}

#[derive(Clone)]
pub struct CommandListState {
    pub ffstate: FixedFunctionState,
    pub ffstate_override: FixedFunctionState,
    pub output: OutputState,
    pub(super) depth_mode: DepthMode,

    resources: [StageResources; 6],
    // pub(crate) resources_vs: StageResources,
    // pub(crate) resources_ps: StageResources,
    // pub(crate) resources_cs: StageResources,
    // pub(crate) resources_ds: StageResources,
    // pub(crate) resources_hs: StageResources,
    // pub(crate) resources_gs: StageResources,
}

impl Default for CommandListState {
    fn default() -> Self {
        Self {
            ffstate: FixedFunctionState::default(),
            ffstate_override: FixedFunctionState::default(),

            output: OutputState::default(),
            depth_mode: DepthMode::Reverse,

            resources: std::array::from_fn(|_| StageResources::default()),
            // resources_vs: StageResources::default(),
            // resources_ps: StageResources::default(),
            // resources_cs: StageResources::default(),
            // resources_ds: StageResources::default(),
            // resources_hs: StageResources::default(),
            // resources_gs: StageResources::default(),
        }
    }
}

#[derive(Default, Clone)]
pub struct OutputState {
    pub rtvs: SmallVec<[CpuDescriptorHandle; 4]>,
    pub rtv_formats: SmallVec<[d3d12::Format; 4]>,
    pub dsv: Option<CpuDescriptorHandle>,
    pub dsv_format: Option<d3d12::Format>,

    pub viewports: SmallVec<[d3d12::Viewport; 4]>,
    pub scissor_rects: SmallVec<[d3d12::Rect; 4]>,
}
