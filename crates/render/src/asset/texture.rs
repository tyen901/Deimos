use std::sync::Arc;

use anyhow::Context;
use d3d12::{
    DeviceChild, ResourceBarrier, ResourceStates, ShaderResourceViewDesc, TextureCopyLocation,
};
use deimos_data::{
    tag::WideHash,
    tfx::{ShaderStage, texture::STextureHeader},
};
use gpu_allocator::{MemoryLocation, d3d12::ResourceCreateDesc};
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;

use crate::gpu::{
    Gpu,
    alloc::{descriptors::ResourceView, resource::OwnedResource},
    command_list::CommandList,
};

pub struct Texture {
    pub resource: OwnedResource,
    pub srv: ResourceView,
    gpu: Arc<Gpu>,
}

impl Texture {
    #[profiling::function]
    pub fn load_data(
        hash: WideHash,
        load_full_mip: bool,
    ) -> anyhow::Result<(STextureHeader, Vec<u8>)> {
        let texture_header_ref = package_manager()
            .get_entry(hash)
            .with_context(|| format!("Texture header entry for {hash} not found"))?
            .reference;

        let texture: STextureHeader = package_manager().read_tag_struct(hash)?;
        let mut texture_data = if texture.large_buffer.is_some() {
            package_manager()
                .read_tag(texture.large_buffer)
                .context("Failed to read texture data")?
        } else {
            package_manager()
                .read_tag(texture_header_ref)
                .context("Failed to read texture data")?
        };

        if load_full_mip && texture.large_buffer.is_some() {
            let ab = package_manager()
                .read_tag(texture_header_ref)
                .context("Failed to read large texture buffer")?;

            texture_data.extend(ab);
        }

        Ok((texture, texture_data))
    }

    pub fn load(gpu: &Arc<Gpu>, hash: tiger_pkg::TagHash) -> anyhow::Result<Self> {
        let hash = hash.into();
        let _span = debug_span!("Load texture", ?hash).entered();
        let (header, texture_data) = Self::load_data(hash, true)?;

        let resource_desc = d3d12::ResourceDesc::new(header.dimension())
            .width(header.width as u64)
            .height(header.height as u32)
            .depth_or_array_size(header.depth.max(header.array_size))
            .format(header.format.into())
            .mip_levels(header.mip_count());

        let resource = gpu.allocate_resource(&ResourceCreateDesc {
            name: "texture",
            memory_location: MemoryLocation::GpuOnly,
            resource_category: gpu_allocator::d3d12::ResourceCategory::OtherTexture,
            resource_desc: resource_desc.as_ref(),
            castable_formats: &[],
            clear_value: None,
            initial_state_or_layout:
                gpu_allocator::d3d12::ResourceStateOrBarrierLayout::ResourceState(
                    d3d12::D3D12_RESOURCE_STATE_COMMON,
                ),
            resource_type: &gpu_allocator::d3d12::ResourceType::Placed,
        })?;

        let num_subresources = header.mip_count() as u32 * header.array_size as u32;
        let footprint = gpu.get_copyable_footprints(&resource_desc, 0, num_subresources, 0)?;

        let upload_buffer = gpu.allocate_upload_buffer(footprint.total_bytes)?;
        unsafe {
            let dst_base = upload_buffer.resource().map(0)?;
            // create a mutable slice covering the entire upload buffer
            let dst = std::slice::from_raw_parts_mut(dst_base, footprint.total_bytes as usize);

            let mut src_offset: usize = 0;

            for (subresource_idx, layout) in footprint.layouts.iter().enumerate() {
                let mip = subresource_idx as u32 % header.mip_count() as u32;

                let mip_width = (header.width >> mip).max(1) as u32;
                let mip_height = (header.height >> mip).max(1) as u32;
                let mip_depth = (header.depth >> mip).max(1) as u32;

                let (src_row_pitch, _) = header.format.calculate_pitch(mip_width, mip_height);

                let block_height: usize = if header.format.is_compressed() {
                    mip_height.div_ceil(4)
                } else {
                    mip_height
                } as usize;

                let dst_slice_pitch = layout.footprint.row_pitch as usize * block_height;

                for depth_slice in 0..mip_depth as usize {
                    for row in 0..block_height {
                        let src_start = src_offset
                            + depth_slice * src_row_pitch * block_height
                            + row * src_row_pitch;
                        let src_end = src_start + src_row_pitch;

                        let dst_start = layout.offset as usize
                            + depth_slice * dst_slice_pitch
                            + row * layout.footprint.row_pitch as usize;
                        let dst_end = dst_start + src_row_pitch;

                        dst[dst_start..dst_end].copy_from_slice(
                            texture_data
                                .get(src_start..src_end)
                                .context("Source slice out of range")?,
                        );
                    }
                }

                src_offset += src_row_pitch * block_height * mip_depth as usize;
            }

            upload_buffer.resource().unmap(0);
        }

        gpu.cmd_scope(|cmd| {
            cmd.resource_barriers(&[ResourceBarrier::transition(
                resource.resource(),
                0,
                ResourceStates::COMMON,
                ResourceStates::COPY_DEST,
            )]);

            for (subresource, layout) in footprint.layouts.into_iter().enumerate() {
                let src = TextureCopyLocation::placed_footprint(upload_buffer.resource(), layout);
                let dst = TextureCopyLocation::subresource(resource.resource(), subresource as u32);

                cmd.copy_texture_region(&src, None, &dst, (0, 0, 0));
            }

            cmd.resource_barriers(&[ResourceBarrier::transition(
                resource.resource(),
                0,
                ResourceStates::COPY_DEST,
                ResourceStates::COMMON,
            )]);

            Ok(())
        })?;

        resource
            .resource()
            .set_debug_name(format!("texture {hash}"));

        let srv_desc = if header.depth > 1 {
            ShaderResourceViewDesc::texture_3d(
                header.format.into(),
                0,
                header.mip_count() as u32,
                0.0,
            )
        } else if header.array_size > 1 {
            ShaderResourceViewDesc::texture_2d_array(
                header.format.into(),
                0,
                header.mip_count() as u32,
                0.0,
                0,
                0..header.array_size as u32,
            )
        } else {
            ShaderResourceViewDesc::texture_2d(
                header.format.into(),
                0,
                header.mip_count() as u32,
                0.0,
                0,
            )
        };

        let srv = gpu.resource_heap.lock().allocate_srv(
            format!("texture_srv {}", hash),
            resource.resource(),
            &srv_desc,
        );

        Ok(Self {
            gpu: gpu.clone(),
            resource,
            srv,
        })
    }

    pub fn bind(&self, cmd: &mut CommandList, slot: u32, stage: ShaderStage) {
        cmd.set_shader_resource_view(stage, slot, Some(self.srv));
    }
}

impl Drop for Texture {
    fn drop(&mut self) {
        self.gpu.resource_heap.lock().free_srv(self.srv);
    }
}
