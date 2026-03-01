use std::{collections::hash_map::Entry, sync::Arc};

use ahash::HashMap;
use anyhow::Context;
use d3d12::{D3D12_DEPTH_STENCIL_DESC, DeviceChild, GraphicsPipelineStateDesc};
use deimos_data::tfx::FixedFunctionState;
use tiger_pkg::{TagHash, package_manager};

use crate::gpu::global_state::RenderStates;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PipelineKey {
    pub vertex_shader: TagHash,
    pub pixel_shader: TagHash,
    pub fixed_function_state: FixedFunctionState,
    pub input_layout: u8,
}

pub struct CachedPipeline {
    pub pso: d3d12::PipelineState,
}

pub struct PipelineCache {
    render_states: RenderStates,
    bytecode_cache: HashMap<TagHash, Arc<[u8]>>,
    storage: HashMap<PipelineKey, CachedPipeline>,

    device: d3d12::Device,
}

impl PipelineCache {
    pub fn new(device: d3d12::Device) -> Self {
        Self {
            render_states: RenderStates::new(&device).expect("Failed to load render states"),
            bytecode_cache: HashMap::default(),
            storage: HashMap::default(),
            device,
        }
    }

    pub fn get_or_create(
        &mut self,
        key: PipelineKey,
        root_signature: &d3d12::RootSignature,
        rtv_formats: &[d3d12::Format],
        dsv_format: Option<d3d12::Format>,
    ) -> anyhow::Result<&CachedPipeline> {
        if self.storage.contains_key(&key) {
            return Ok(self.storage.get(&key).expect("unreachable: just checked"));
        }

        let vs = self.get_or_load_bytecode(key.vertex_shader)?;
        let ps = self.get_or_load_bytecode(key.pixel_shader)?;

        let input_layout = self
            .render_states
            .input_layouts
            .get(key.input_layout as usize)
            .with_context(|| format!("invalid input layout {}", key.input_layout))?;

        let blend_state_index = key.fixed_function_state.blend_state().unwrap_or(0);
        let blend_state = self
            .render_states
            .blend_states
            .get(blend_state_index)
            .context("invalid blend state")?;

        let depth_state = D3D12_DEPTH_STENCIL_DESC {
            DepthEnable: true.into(),
            DepthWriteMask: d3d12::D3D12_DEPTH_WRITE_MASK_ALL,
            DepthFunc: d3d12::D3D12_COMPARISON_FUNC_GREATER_EQUAL,
            StencilEnable: false.into(),
            ..Default::default()
        };

        let pipeline = self
            .device
            .create_graphics_pipeline_state(
                &GraphicsPipelineStateDesc::new(root_signature)
                    .with_vs(&vs)
                    .with_ps(&ps)
                    .with_input_layout(input_layout.as_slice())
                    .with_primitive_topology(d3d12::PrimitiveTopology2::Triangle)
                    .with_blend_state(blend_state.clone())
                    .with_rtv_formats(rtv_formats)
                    .with_dsv_format(dsv_format.unwrap_or_default())
                    .with_depth_stencil_state(depth_state),
            )
            .context("create_graphics_pipeline_state")?;

        pipeline.set_debug_name(format!("vs_{}_ps_{}", key.vertex_shader, key.pixel_shader));

        self.storage
            .insert(key.clone(), CachedPipeline { pso: pipeline });
        Ok(self.storage.get(&key).expect("unreachable: just inserted"))
    }

    pub fn get_or_load_bytecode(&mut self, hash: TagHash) -> anyhow::Result<Arc<[u8]>> {
        match self.bytecode_cache.entry(hash) {
            Entry::Occupied(entry) => Ok(entry.into_mut().clone()),
            Entry::Vacant(entry) => {
                let tag_entry = package_manager()
                    .get_entry(hash)
                    .context("entry not found")?;
                let data = package_manager()
                    .read_tag(tag_entry.reference)
                    .context("failed to read shader bytecode")?;

                Ok(entry.insert(Arc::from(data)).clone())
            }
        }
    }
}
