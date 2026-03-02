use std::{ops::Deref, sync::Arc};

use d3d12::GraphicsCommandList;
use deimos_data::tfx::{FixedFunctionState, PrimitiveType, ShaderStage};
use tiger_pkg::TagHash;

use crate::{
    gpu::alloc::{descriptors::ResourceView, ring::UploadRing},
    renderer::Renderer,
    tfx::externs::LocalExternContainer,
};

use super::Gpu;

pub struct CommandList {
    parent: Arc<Gpu>,
    pub(crate) cmd: GraphicsCommandList,

    pub state: FixedFunctionState,
    pub state_override: FixedFunctionState,
    pub(super) current_blend_state: usize,
    pub(super) current_depth_state: usize,
    pub(super) current_rasterizer_state: usize,
    pub(super) current_depth_bias: usize,
    pub(super) current_input_layout: usize,
    pub(super) current_input_topology: usize,
    pub(super) current_stencil_ref: u32,
    pub(super) depth_mode: DepthMode,
    pub(super) bound_technique: TagHash,
    pub(crate) resources_vs: StageResources,
    pub(crate) resources_ps: StageResources,
    pub(crate) resources_cs: StageResources,
    pub(crate) resources_ds: StageResources,
    pub(crate) resources_hs: StageResources,
    pub(crate) resources_gs: StageResources,

    pub externs: LocalExternContainer,
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

    pub fn from_native_command_list(renderer: &Arc<Renderer>, cmd: GraphicsCommandList) -> Self {
        Self {
            parent: renderer.gpu.clone(),
            cmd,
            state: FixedFunctionState::default(),
            state_override: FixedFunctionState::default(),

            current_blend_state: usize::MAX,
            current_depth_state: usize::MAX,
            current_input_layout: usize::MAX,
            current_rasterizer_state: usize::MAX,
            current_depth_bias: usize::MAX,
            current_input_topology: usize::MAX,
            current_stencil_ref: 0,
            depth_mode: DepthMode::Reverse,
            bound_technique: TagHash::NONE,
            resources_vs: StageResources::default(),
            resources_ps: StageResources::default(),
            resources_cs: StageResources::default(),
            resources_ds: StageResources::default(),
            resources_hs: StageResources::default(),
            resources_gs: StageResources::default(),

            externs: LocalExternContainer::new(renderer.clone()),
        }
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

    pub fn gpu(&self) -> &Gpu {
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
        self.current_input_topology = usize::MAX;
        self.bound_technique = TagHash::NONE;
    }

    pub const fn flush_states(&mut self) {
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

    pub const fn set_blend_state(&mut self, index: usize) {
        self.current_blend_state = index;
    }

    pub fn set_depth_mode(&mut self, mode: DepthMode) {
        if self.depth_mode != mode {
            self.depth_mode = mode;
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

    /// Returns true if the given technique is already bound
    pub fn set_bound_technique(&mut self, index: TagHash) -> bool {
        if self.bound_technique != index {
            self.bound_technique = index;
            false
        } else {
            true
        }
    }

    pub const fn set_input_layout(&mut self, index: usize) {
        self.current_input_layout = index;
    }

    pub const fn get_input_layout(&self) -> usize {
        self.current_input_layout
    }

    /// Applies a one-time state override
    pub const fn apply_state(&mut self, states: &FixedFunctionState) {
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
        if self.current_input_topology != topology as usize {
            self.cmd.ia_set_primitive_topology(match topology {
                PrimitiveType::PointList => d3d12::PrimitiveTopology::PointList,
                PrimitiveType::LineList => d3d12::PrimitiveTopology::LineList,
                PrimitiveType::LineStrip => d3d12::PrimitiveTopology::LineStrip,
                PrimitiveType::Triangles => d3d12::PrimitiveTopology::TriangleList,
                PrimitiveType::TriangleStrip => d3d12::PrimitiveTopology::TriangleStrip,
            });
            self.current_input_topology = topology as usize;
        }
    }

    #[deprecated(note = "This method bypasses the pipeline state, use set_input_topology instead")]
    pub fn input_assembler_set_primitive_topology(&self, topology: d3d12::PrimitiveTopology) {
        self.cmd.ia_set_primitive_topology(topology);
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

    pub const fn resources(&mut self, stage: ShaderStage) -> &mut StageResources {
        match stage {
            ShaderStage::Vertex => &mut self.resources_vs,
            ShaderStage::Pixel => &mut self.resources_ps,
            ShaderStage::Compute => &mut self.resources_cs,
            ShaderStage::Domain => &mut self.resources_ds,
            ShaderStage::Hull => &mut self.resources_hs,
            ShaderStage::Geometry => &mut self.resources_gs,
        }
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
        if let Some(slot_mut) = self.resources(stage).srvs.get_mut(slot as usize) {
            *slot_mut = resource;
        }
    }

    pub fn set_shader_constant_buffer_view(
        &mut self,
        stage: ShaderStage,
        slot: u32,
        buffer_address: Option<d3d12::GpuVirtualAddress>,
    ) {
        if let Some(slot_mut) = self.resources(stage).cbvs.get_mut(slot as usize) {
            *slot_mut = buffer_address;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthMode {
    /// Commonly used for shadow maps and decals
    Forward,
    /// Used by default
    Reverse,
}

#[derive(Default)]
pub struct StageResources {
    pub srvs: [Option<ResourceView>; 32],
    pub cbvs: [Option<d3d12::GpuVirtualAddress>; 16],
}

#[macro_export]
macro_rules! cmd_event_span {
    ($cmd:ident, $name:expr) => {
        profiling::scope!(&format!("cmd-{}", $name));
        let _gpu_span = $cmd.begin_event_span($name);
    };
}

// pub trait ContextExt {
//     fn set_shader_resource<'a>(
//         &self,
//         stage: ShaderStage,
//         slot: u32,
//         srv: impl Into<Option<&'a d3d11::ShaderResourceView>>,
//     );
//     fn set_sampler<'a>(
//         &self,
//         stage: ShaderStage,
//         slot: u32,
//         sampler: impl Into<Option<&'a d3d11::SamplerState>>,
//     );

//     fn set_constant_buffer<'a>(
//         &self,
//         stage: ShaderStage,
//         slot: u32,
//         cbuffer: impl Into<Option<&'a d3d11::Buffer>>,
//     );
// }

// // Convenience methods for setting resources by stage
// impl ContextExt for d3d11::DeviceContext {
//     #[inline(always)]
//     fn set_shader_resource<'a>(
//         &self,
//         stage: ShaderStage,
//         slot: u32,
//         srv: impl Into<Option<&'a d3d11::ShaderResourceView>>,
//     ) {
//         (match stage {
//             ShaderStage::Pixel => DeviceContext::pixel_set_shader_resources,
//             ShaderStage::Vertex => DeviceContext::vertex_set_shader_resources,
//             ShaderStage::Geometry => DeviceContext::geometry_set_shader_resources,
//             ShaderStage::Hull => DeviceContext::hull_set_shader_resources,
//             ShaderStage::Compute => DeviceContext::compute_set_shader_resources,
//             ShaderStage::Domain => DeviceContext::domain_set_shader_resources,
//         })(self, slot, &[srv.into()]);
//     }

//     #[inline(always)]
//     fn set_sampler<'a>(
//         &self,
//         stage: ShaderStage,
//         slot: u32,
//         sampler: impl Into<Option<&'a d3d11::SamplerState>>,
//     ) {
//         (match stage {
//             ShaderStage::Pixel => DeviceContext::pixel_set_samplers,
//             ShaderStage::Vertex => DeviceContext::vertex_set_samplers,
//             ShaderStage::Geometry => DeviceContext::geometry_set_samplers,
//             ShaderStage::Hull => DeviceContext::hull_set_samplers,
//             ShaderStage::Compute => DeviceContext::compute_set_samplers,
//             ShaderStage::Domain => DeviceContext::domain_set_samplers,
//         })(self, slot, &[sampler.into()]);
//     }

//     #[inline(always)]
//     fn set_constant_buffer<'a>(
//         &self,
//         stage: ShaderStage,
//         slot: u32,
//         cbuffer: impl Into<Option<&'a d3d11::Buffer>>,
//     ) {
//         (match stage {
//             ShaderStage::Pixel => DeviceContext::pixel_set_constant_buffers,
//             ShaderStage::Vertex => DeviceContext::vertex_set_constant_buffers,
//             ShaderStage::Geometry => DeviceContext::geometry_set_constant_buffers,
//             ShaderStage::Hull => DeviceContext::hull_set_constant_buffers,
//             ShaderStage::Compute => DeviceContext::compute_set_constant_buffers,
//             ShaderStage::Domain => DeviceContext::domain_set_constant_buffers,
//         })(self, slot, &[cbuffer.into()]);
//     }
// }
