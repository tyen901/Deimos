use std::sync::Arc;

use anyhow::Context;
use d3d12::{DeviceChild, Format, ResourceBarrier, ResourceStates, TextureCopyLocation};
use deimos_data::{tag::WideHash, tfx::texture::STextureHeader};
use gpu_allocator::{MemoryLocation, d3d12::ResourceCreateDesc};
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

use crate::gpu::{Gpu, alloc::resource::OwnedResource};

pub struct Texture {
    pub resource: OwnedResource,
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
                .to_vec()
        };

        if load_full_mip && texture.large_buffer.is_some() {
            let ab = package_manager()
                .read_tag(texture_header_ref)
                .context("Failed to read large texture buffer")?
                .to_vec();

            texture_data.extend(ab);
        }

        Ok((texture, texture_data))
    }

    pub fn load(gpu: &Arc<Gpu>, hash: tiger_pkg::TagHash) -> anyhow::Result<Self> {
        let hash = hash.into();
        let _span = debug_span!("Load texture", ?hash).entered();
        let (texture, texture_data) = Self::load_data(hash, true)?;

        let dimension = if texture.depth > 1 {
            d3d12::ResourceDimension::Texture3D
        } else {
            d3d12::ResourceDimension::Texture2D
        };
        let resource_desc = d3d12::ResourceDesc::new(dimension)
            .width(texture.width as u64)
            .height(texture.height as u32)
            .depth_or_array_size(texture.depth.max(texture.array_size))
            .format(texture.format.into())
            .mip_levels(texture.mip_count as u16);

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

        let num_subresources = texture.mip_count as u32 * texture.array_size as u32;
        let footprint = gpu.get_copyable_footprints(&resource_desc, 0, num_subresources, 0)?;

        let upload_buffer = gpu.allocate_upload_buffer(footprint.total_bytes)?;
        unsafe {
            let dst_base = upload_buffer.resource().map(0)?;

            let mut src_offset: usize = 0;

            for (subresource_idx, layout) in footprint.layouts.iter().enumerate() {
                let mip = subresource_idx as u32 % texture.mip_count as u32;

                let mip_width = (texture.width >> mip).max(1) as u32;
                let mip_height = (texture.height >> mip).max(1) as u32;
                let mip_depth = (texture.depth >> mip).max(1) as u32;

                let (src_row_pitch, _) = texture.format.calculate_pitch(mip_width, mip_height);

                let block_height: u32 = if texture.format.is_compressed() {
                    mip_height.div_ceil(4)
                } else {
                    mip_height
                };

                let dst_slice_pitch = layout.footprint.row_pitch as usize * block_height as usize;

                for depth_slice in 0..mip_depth as usize {
                    for row in 0..block_height as usize {
                        let src_ptr = texture_data.as_ptr().add(
                            src_offset
                                + depth_slice * src_row_pitch * block_height as usize
                                + row * src_row_pitch,
                        );

                        let dst_ptr = dst_base.add(
                            layout.offset as usize
                                + depth_slice * dst_slice_pitch
                                + row * layout.footprint.row_pitch as usize,
                        );

                        std::ptr::copy_nonoverlapping(src_ptr, dst_ptr, src_row_pitch);
                    }
                }

                src_offset += src_row_pitch * block_height as usize * mip_depth as usize;
            }

            upload_buffer.resource().unmap(0);
        }
        gpu.immediate_pool.scope_immediate(|cmd| {
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

        Ok(Self { resource })
    }
}
