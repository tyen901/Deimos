use std::sync::Arc;

use anyhow::Context;
use d3d12::{
    Format, RootSignatureBuilder, RootSignatureFlags,
    ext::{PsvResourceBinding, PsvResourceType},
};
use deimos_data::tfx::{STechnique, STechniqueStage, ShaderStage, TechniqueBindMode};
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
    tfx::dynamic_core::{DynamicCore, ResolvedTextureSource},
};

pub struct Technique {
    gpu: Arc<Gpu>,
    pub tag: TagHash,
    pub data: STechnique,
    root_signature: d3d12::RootSignature,
    stage_vertex: TechniqueStage,
    stage_pixel: TechniqueStage,

    descriptor_table_parameters: SmallVec<[u32; 3]>,
    descriptor_count: usize,
    static_descriptor_heap: Option<d3d12::DescriptorHeap>,
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

        let mut stage_vertex = TechniqueStage::new(
            asset_manager,
            gpu,
            data.shader_vertex.clone(),
            ShaderStage::Vertex,
        )
        .context("while loading vertex stage")?;

        let mut stage_pixel = TechniqueStage::new(
            asset_manager,
            gpu,
            data.shader_pixel.clone(),
            ShaderStage::Pixel,
        )
        .context("while loading pixel stage")?;

        let mut rsb = RootSignatureBuilder::default()
            .flags(RootSignatureFlags::ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT);

        let mut descriptor_ranges = Vec::new();
        let mut descriptor_offset = 0;
        for stage in [&mut stage_vertex, &mut stage_pixel] {
            for scopes in data.used_scopes.iter() {
                let index = scopes.bits().ilog2();
                let scope = get_scope_samplers(index);
                if let Some(scope_stage) = scope.stage_by_visibility(stage.visibility) {
                    for sampler in &scope_stage.core.samplers {
                        rsb.add_sampler(sampler.clone());
                    }
                }
            }

            for sampler in &stage.core.samplers {
                rsb.add_sampler(sampler.clone());
            }

            let mut local_ranges = vec![];
            for resource in &stage.resources {
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

                        let is_static = stage.core.textures.iter().any(|(slot, src)| {
                            *slot == resource.lower_bound
                                && matches!(src, ResolvedTextureSource::Static(_))
                        });
                        stage.root_texture_slots.push(TechniqueResourceSlot {
                            descriptor_offset: descriptor_offset as u8,
                            register: resource.lower_bound as u8,
                            is_static,
                        });
                        descriptor_offset += 1;
                        local_ranges.push(range);
                    }
                    d3d12::ext::PsvResourceType::CBV => {
                        let rs_index = rsb.add_param(
                            d3d12::RootParameter::CbvDescriptor {
                                shader_register: resource.lower_bound,
                                register_space: 0,
                            },
                            stage.visibility,
                        );
                        debug_assert!(rs_index <= 0xff);
                        debug_assert!(resource.lower_bound <= 0xff);
                        stage.root_cbuffer_slots.push(TechniqueCbufferSlot {
                            register: resource.lower_bound as u8,
                            rs_slot: rs_index as u8,
                        });
                    }
                    _ => {}
                }
            }
            if !local_ranges.is_empty() {
                descriptor_ranges.push((local_ranges, stage.visibility));
            }
        }
        let mut descriptor_table_parameters = SmallVec::new();
        for (ranges, visibility) in descriptor_ranges.iter() {
            let index = rsb.add_param(
                d3d12::RootParameter::DescriptorTable(ranges.as_slice()),
                *visibility,
            );
            descriptor_table_parameters.push(index as u32);
        }

        let descriptor_count = descriptor_offset as usize;

        let root_signature_raw = rsb.serialize()?;

        let root_signature = gpu.create_root_signature(&root_signature_raw)?;

        let technique = Self {
            tag: hash,
            gpu: gpu.clone(),
            root_signature,
            stage_vertex,
            stage_pixel,
            data,
            descriptor_table_parameters,
            descriptor_count,
            static_descriptor_heap: None,
            // TODO(cohae): Weirdly enough this causes an OOM issue when loading a lot of entities in the entity view
            // static_descriptor_heap: (descriptor_count > 0).then(|| {
            //     gpu.create_descriptor_heap(
            //         d3d12::DescriptorHeapType::CbvSrvUav,
            //         descriptor_count as u32,
            //         false,
            //         0,
            //     )
            //     .expect("failed to create static descriptor heap")
            // }),
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
                &[
                    Format::R8g8b8a8Unorm,
                    Format::R10g10b10a2Unorm,
                    Format::R8g8b8a8Unorm,
                    Format::R32g32Float,
                ],
                Some(Format::D32FloatS8x24Uint),
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

        cmd.set_descriptor_heaps(std::slice::from_ref(cmd.gpu().frame().descriptors.heap()));

        let descriptor_range = cmd
            .gpu()
            .frame()
            .descriptors
            .allocate(self.descriptor_count);
        if let Some(static_descriptor_heap) = &self.static_descriptor_heap {
            cmd.gpu().copy_descriptor_range(
                self.descriptor_count as u32,
                static_descriptor_heap.cpu_descriptor_handle_for_heap_start(),
                descriptor_range.cpu_handle(0),
                d3d12::DescriptorHeapType::CbvSrvUav,
            );
        }

        for &param in self.descriptor_table_parameters.iter() {
            cmd.set_graphics_root_descriptor_table(param, descriptor_range.gpu_handle(0));
        }

        for stage in self.all_stages() {
            stage.bind(cmd, &descriptor_range, full_rebind);
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
            if res.res_type == PsvResourceType::SRVTyped
                && !core
                    .textures
                    .iter()
                    .any(|(slot, _)| *slot == res.lower_bound)
            {
                has_manual_textures = true;
                break;
            }
        }

        Ok(Self {
            core,
            visibility: shader_stage.shader_visibility(),
            root_cbuffer_slots: SmallVec::new(),
            root_texture_slots: SmallVec::new(),
            resources,
            has_manual_textures,
        })
    }

    #[profiling::function]
    pub fn bind(
        &self,
        cmd: &mut CommandList,
        descriptor_range: &DescriptorRange,
        full_rebind: bool,
    ) {
        if full_rebind && let Err(e) = self.core.prepare(cmd) {
            error!("Failed to prepare technique: {}", e);
            return;
        }

        let resources = cmd.resources(self.core.stage);

        for slot in &self.root_cbuffer_slots {
            if let Some(va) = resources.get_shader_constant_buffer_view(slot.register as u32) {
                cmd.set_graphics_root_constant_buffer_view(slot.rs_slot as u32, va);
            } else {
                // error!(
                //     "Missing constant buffer view for register {} ({}:{:?})",
                //     slot.register, technique.tag, self.core.stage
                // );
                cmd.set_graphics_root_constant_buffer_view(
                    slot.rs_slot as u32,
                    d3d12::GpuVirtualAddress::NULL,
                );
            }
        }

        // Filter out static texture slots, as we already copied those from the static descriptor heap
        for slot in self.root_texture_slots.iter() {
            // .filter(|s| !s.is_static) {
            if let Some(tex) = resources.get_shader_resource_view(slot.register as u32) {
                cmd.gpu().copy_descriptors_simple(
                    1,
                    tex.cpu_handle(),
                    descriptor_range.cpu_handle(slot.descriptor_offset as usize),
                    d3d12::DescriptorHeapType::CbvSrvUav,
                );
            } else {
                // error!(
                //     "Missing texture view for register {} ({}:{:?})",
                //     slot.register, technique.tag, self.core.stage
                // );
                cmd.gpu().copy_descriptors_simple(
                    1,
                    cmd.gpu().resource_heap.lock().null().cpu_handle(),
                    descriptor_range.cpu_handle(slot.descriptor_offset as usize),
                    d3d12::DescriptorHeapType::CbvSrvUav,
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
}
