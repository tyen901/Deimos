use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use d3d12::{
    Format, RootSignatureBuilder, RootSignatureFlags,
    ext::{PsvResourceBinding, PsvResourceType},
};
use deimos_data::tfx::{STechnique, STechniqueStage, ShaderStage, TechniqueBindMode, TfxScopeBits};
use parking_lot::Mutex;
use smallvec::SmallVec;
use tiger_parse::PackageManagerExt;
use tiger_pkg::{TagHash, package_manager};

use crate::{
    asset::AssetManager,
    gpu::{
        Gpu, alloc::descriptors::DescriptorRange, command_list::CommandList,
        pipeline_cache::PipelineKey,
    },
    renderer::globals::get_scope_samplers,
    tfx::{
        dynamic_core::{DynamicCore, ResolvedTextureSource},
        expression_vm::opcodes::get_accessed_externs_from_bytecode,
    },
};

pub struct Technique {
    gpu: Arc<Gpu>,
    pub tag: TagHash,
    pub data: STechnique,
    root_signature: d3d12::RootSignature,
    stage_vertex: TechniqueStage,
    stage_pixel: TechniqueStage,
}

impl Technique {
    pub fn load(
        asset_manager: &Arc<AssetManager>,
        gpu: &Arc<Gpu>,
        hash: TagHash,
    ) -> anyhow::Result<Self> {
        let data: STechnique = package_manager()
            .read_tag_struct(hash)
            .context("Failed to read technique data")?;

        let mut rsb = RootSignatureBuilder::default()
            .flags(RootSignatureFlags::ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT);

        let mut stage_vertex = TechniqueStage::new(
            asset_manager,
            gpu,
            &mut rsb,
            &data,
            data.shader_vertex.clone(),
            ShaderStage::Vertex,
        )
        .context("while loading vertex stage")?;

        stage_vertex.srv_root_param_index = rsb.add_param(
            d3d12::RootParameter::DescriptorTable(stage_vertex.descriptor_ranges.as_slice()),
            stage_vertex.visibility,
        );

        let mut stage_pixel = TechniqueStage::new(
            asset_manager,
            gpu,
            &mut rsb,
            &data,
            data.shader_pixel.clone(),
            ShaderStage::Pixel,
        )
        .context("while loading pixel stage")?;

        stage_pixel.srv_root_param_index = rsb.add_param(
            d3d12::RootParameter::DescriptorTable(stage_pixel.descriptor_ranges.as_slice()),
            stage_pixel.visibility,
        );

        let root_signature_raw = rsb.serialize()?;

        let root_signature = gpu.create_root_signature(&root_signature_raw)?;

        let technique = Self {
            tag: hash,
            gpu: gpu.clone(),
            root_signature,
            stage_vertex,
            stage_pixel,
            data,
        };

        // if let Some(static_descriptor_heap) = &technique.static_descriptor_heap {
        //     for stage in technique.all_stages() {
        //         while !stage.core.all_textures_loaded() {
        //             rayon::yield_local();
        //             // potassium::yield_job();
        //         }

        //         stage
        //             .copy_static_descriptors(gpu, static_descriptor_heap)
        //             .context("copying static descriptors")?;
        //     }
        // }

        Ok(technique)
    }

    /// Bind the technique to the command list.
    ///
    /// Make sure to set any necessary states (input layout, etc.) before calling this method, as these need to be compiled into the PSO
    #[profiling::function]
    pub fn bind(&self, cmd: &mut CommandList) {
        let full_rebind = !cmd.is_technique_smart_bound(self.tag);

        if !matches!(
            self.data.bind_mode,
            TechniqueBindMode::VertexOnly | TechniqueBindMode::VertexPixel
        ) {
            error!("{:?} bind mode not implemented", self.data.bind_mode);
            return;
        }

        if full_rebind {
            let fixed_function_state = cmd
                .cmd_state()
                .ffstate
                .select(&self.data.states)
                .select(&cmd.cmd_state().ffstate_override);

            let pipeline_key = PipelineKey {
                vertex_shader: self.data.shader_vertex.shader,
                pixel_shader: self.data.shader_pixel.shader,
                fixed_function_state,
                input_layout: cmd.get_input_layout() as u8,
            };

            match self.gpu.pipeline_cache.lock().get_or_create(
                pipeline_key,
                &self.root_signature,
                &cmd.cmd_state().output.rtv_formats,
                cmd.cmd_state().output.dsv_format,
            ) {
                Ok(pipeline) => {
                    cmd.set_pipeline_state(&pipeline.pso);
                    cmd.set_root_signature(&self.root_signature);
                }
                Err(err) => {
                    error!("Failed to create pipeline: {}", err);
                    return;
                }
            }
        }

        for stage in self.all_stages() {
            let skip_full_stage_rebind = !full_rebind
                && !stage.core.has_dynamic_textures()
                && !stage.has_manual_textures
                && stage.core.stage != ShaderStage::Vertex;

            if skip_full_stage_rebind {
                stage.bind_cbvs(cmd);
            } else {
                stage.bind(cmd, full_rebind);
            }
        }
    }

    pub const fn all_stages(&self) -> [&TechniqueStage; 2] {
        [&self.stage_vertex, &self.stage_pixel]
    }

    pub fn is_loaded(&self) -> bool {
        self.all_stages()
            .iter()
            .all(|s| s.core.all_textures_loaded())
    }
}

pub struct TechniqueStage {
    pub core: DynamicCore,
    pub visibility: d3d12::ShaderVisibility,

    pub root_cbuffer_slots: SmallVec<[TechniqueCbufferSlot; 16]>,
    pub root_texture_slots: SmallVec<[TechniqueResourceSlot; 16]>,

    pub resources: Vec<PsvResourceBinding>,

    /// Does this stage have any textures bound outside of the technique's expressions?
    pub has_manual_textures: bool,

    pub srv_root_param_index: u32,
    pub descriptor_count: usize,

    descriptor_ranges: Vec<d3d12::DescriptorRange>,
    static_staging_heap: Option<d3d12::DescriptorHeap>,
}

pub struct TechniqueCbufferSlot {
    pub register: u8,
    pub rs_slot: u8,
}

pub struct TechniqueResourceSlot {
    pub register: u8,
    pub descriptor_offset: u8,
    is_static: bool,
}

impl TechniqueStage {
    pub fn new(
        asset_manager: &Arc<AssetManager>,
        gpu: &Arc<Gpu>,
        rsb: &mut d3d12::RootSignatureBuilder,
        technique_data: &STechnique,
        stage: STechniqueStage,
        shader_stage: ShaderStage,
    ) -> anyhow::Result<Self> {
        let resources = if let Ok(bytecode) =
            gpu.pipeline_cache.lock().get_or_load_bytecode(stage.shader)
            && let Some(parsed_resources) = d3d12::ext::parse_psv0_resources(&bytecode)
        {
            parsed_resources
        } else {
            vec![]
        };

        let core = DynamicCore::new(asset_manager, stage.core, shader_stage)?;

        let mut has_manual_textures = false;
        for res in &resources {
            if matches!(
                res.res_type,
                PsvResourceType::SRVTyped
                    | PsvResourceType::SRVRaw
                    | PsvResourceType::SRVStructured,
            ) && !core
                .textures
                .iter()
                .any(|(slot, _)| *slot == res.lower_bound)
            {
                has_manual_textures = true;
                break;
            }
        }

        for scopes in technique_data.used_scopes.iter() {
            let index = scopes.bits().ilog2();
            let scope = get_scope_samplers(index);
            if let Some(scope_stage) = scope.stage_by_visibility(shader_stage.shader_visibility()) {
                for sampler in &scope_stage.core.samplers {
                    rsb.add_sampler(sampler.clone());
                }
            }
        }

        for sampler in &core.samplers {
            rsb.add_sampler(sampler.clone());
        }

        let mut root_cbuffer_slots = SmallVec::new();
        let mut root_texture_slots = SmallVec::new();

        let mut descriptor_ranges = vec![];
        let mut descriptor_offset = 0;
        for resource in &resources {
            match resource.res_type {
                d3d12::ext::PsvResourceType::SRVTyped
                | d3d12::ext::PsvResourceType::SRVRaw
                | d3d12::ext::PsvResourceType::SRVStructured => {
                    let range = d3d12::DescriptorRange {
                        range_type: d3d12::DescriptorRangeType::Srv,
                        num_descriptors: 1,
                        base_shader_register: resource.lower_bound,
                        register_space: 0,
                        offset_in_descriptors_from_table_start: descriptor_offset,
                    };
                    debug_assert!(descriptor_offset <= 0xff);
                    debug_assert!(resource.lower_bound <= 0xff);

                    let is_static = core.textures.iter().any(|(slot, src)| {
                        *slot == resource.lower_bound
                            && matches!(src, ResolvedTextureSource::Static(_))
                    });
                    root_texture_slots.push(TechniqueResourceSlot {
                        descriptor_offset: descriptor_offset as u8,
                        register: resource.lower_bound as u8,
                        is_static,
                    });
                    descriptor_offset += 1;
                    descriptor_ranges.push(range);
                }
                d3d12::ext::PsvResourceType::CBV => {
                    let rs_index = rsb.add_param(
                        d3d12::RootParameter::CbvDescriptor {
                            shader_register: resource.lower_bound,
                            register_space: 0,
                        },
                        shader_stage.shader_visibility(),
                    );
                    debug_assert!(rs_index <= 0xff);
                    debug_assert!(resource.lower_bound <= 0xff);
                    root_cbuffer_slots.push(TechniqueCbufferSlot {
                        register: resource.lower_bound as u8,
                        rs_slot: rs_index as u8,
                    });
                }
                _ => {}
            }
        }

        let mut stage = Self {
            core,
            visibility: shader_stage.shader_visibility(),
            root_cbuffer_slots,
            root_texture_slots,
            resources,
            has_manual_textures,
            srv_root_param_index: u32::MAX,
            descriptor_ranges,
            descriptor_count: descriptor_offset as usize,
            static_staging_heap: None,
        };

        stage.prebuild_static_staging(gpu)?;

        Ok(stage)
    }

    #[profiling::function]
    pub fn bind(&self, cmd: &mut CommandList, full_rebind: bool) {
        if full_rebind && let Err(e) = self.core.prepare(cmd) {
            error!("Failed to prepare technique: {}", e);
            return;
        }

        let descriptor_range = cmd
            .gpu()
            .frame()
            .descriptors
            .allocate(self.descriptor_count);
        cmd.set_graphics_root_descriptor_table(
            self.srv_root_param_index,
            descriptor_range.gpu_handle(0),
        );

        let resources = cmd.resources(self.core.stage);

        if let Some(staging) = &self.static_staging_heap {
            cmd.gpu().copy_descriptors_simple(
                self.descriptor_count as u32,
                staging.cpu_descriptor_handle_for_heap_start(),
                descriptor_range.cpu_handle(0),
                d3d12::DescriptorHeapType::CbvSrvUav,
            );
        } else {
            let null = cmd.gpu().resource_heap.lock().null().cpu_handle();
            let resources = cmd.resources(self.core.stage);
            for slot in &self.root_texture_slots {
                let src = resources
                    .get_shader_resource_view(slot.register as u32)
                    .map(|t| t.cpu_handle())
                    .unwrap_or(null);
                cmd.gpu().copy_descriptors_simple(
                    1,
                    src,
                    descriptor_range.cpu_handle(slot.descriptor_offset as usize),
                    d3d12::DescriptorHeapType::CbvSrvUav,
                );
            }
        }

        self.bind_cbvs(cmd);
    }

    pub fn bind_cbvs(&self, cmd: &mut CommandList) {
        let resources = cmd.resources(self.core.stage);

        for slot in &self.root_cbuffer_slots {
            if let Some(va) = resources.get_shader_constant_buffer_view(slot.register as u32) {
                cmd.set_graphics_root_constant_buffer_view(slot.rs_slot as u32, va);
            } else {
                // error!(
                //     "Missing constant buffer view for register {} ({:?})",
                //     slot.register, self.core.stage
                // );
                cmd.set_graphics_root_constant_buffer_view(
                    slot.rs_slot as u32,
                    cmd.gpu().frame().upload.null(),
                );
            }
        }
    }

    pub fn copy_static_descriptors(
        &self,
        gpu: &Gpu,
        descriptor_heap: &d3d12::DescriptorHeap,
    ) -> anyhow::Result<()> {
        let increment_size =
            gpu.descriptor_handle_increment_size(d3d12::DescriptorHeapType::CbvSrvUav);
        for slot in &self.root_texture_slots {
            let Some((_, source)) = self
                .core
                .textures
                .iter()
                .find(|(core_slot, _)| *core_slot == slot.register as u32)
            else {
                continue;
            };

            let handle = match source {
                ResolvedTextureSource::Static(handle) => handle
                    .get()
                    .context("texture hasn't been loaded yet")?
                    .srv
                    .cpu_handle(),
                ResolvedTextureSource::Dynamic { .. } => gpu.resource_heap.lock().null_texture2d,
            };

            gpu.copy_descriptors_simple(
                1,
                handle,
                descriptor_heap
                    .cpu_descriptor_handle_for_heap_start()
                    .offset(slot.descriptor_offset as usize, increment_size),
                d3d12::DescriptorHeapType::CbvSrvUav,
            );
        }

        Ok(())
    }

    fn prebuild_static_staging(&mut self, gpu: &Gpu) -> anyhow::Result<()> {
        if self.core.has_dynamic_textures()
            || self.has_manual_textures
            || self.descriptor_count == 0
        {
            return Ok(());
        }

        let mut start = Instant::now();
        while !self.core.all_textures_loaded() {
            rayon::yield_local();
            // potassium::yield_job();
            if start.elapsed() > Duration::from_secs(5) {
                warn!(
                    "Timed out waiting for textures to load before building static descriptor staging heap"
                );
            }
        }

        let staging = gpu
            .create_descriptor_heap(
                d3d12::DescriptorHeapType::CbvSrvUav,
                self.descriptor_count as u32,
                false, // CPU-only, not shader visible
                0,
            )
            .expect("failed to create staging heap");

        self.copy_static_descriptors(gpu, &staging)
            .context("copying static descriptors")?;

        // let null = gpu.resource_heap.lock().null().cpu_handle();
        // for slot in &self.root_texture_slots {
        //     let src = self
        //         .core
        //         .textures
        //         .iter()
        //         .find(|(s, _)| *s == slot.register as u32)
        //         .and_then(|(_, src)| match src {
        //             ResolvedTextureSource::Static(h) => h.get().map(|t| t.srv.cpu_handle()),
        //             _ => None,
        //         })
        //         .unwrap_or(null);

        //     gpu.copy_descriptors_simple(
        //         1,
        //         src,
        //         staging.cpu_descriptor_handle_for_heap_start().offset(
        //             slot.descriptor_offset as usize,
        //             gpu.descriptor_handle_increment_size(d3d12::DescriptorHeapType::CbvSrvUav),
        //         ),
        //         d3d12::DescriptorHeapType::CbvSrvUav,
        //     );
        // }

        self.static_staging_heap = Some(staging);

        return Ok(());
    }
}
