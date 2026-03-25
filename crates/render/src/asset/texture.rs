use std::{borrow::Cow, f32::consts::PI, sync::Arc};

use anyhow::Context;
use d3d12::{
    DeviceChild, ResourceBarrier, ResourceStates, ShaderResourceViewDesc, TextureCopyLocation,
};
use deimos_data::{
    tag::WideHash,
    tfx::{ShaderStage, texture::STextureHeader},
};
use gpu_allocator::{MemoryLocation, d3d12::ResourceCreateDesc};
use image::{DynamicImage, GenericImageView, Rgba, Rgba32FImage};
use itertools::Itertools;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;

use crate::{
    gpu::{
        Gpu,
        alloc::{descriptors::ResourceView, resource::OwnedResource},
        command_list::CommandList,
    },
    util::filtering::{
        NUM_SAMPLES, d_ggx, hammersley, importance_sample_ggx, sample_base_bilinear,
        sample_mip_bilinear, uv_to_dir,
    },
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

    pub fn load_tag(gpu: &Arc<Gpu>, hash: tiger_pkg::TagHash) -> anyhow::Result<Self> {
        let hash = hash.into();
        let _span = debug_span!("Load texture", ?hash).entered();
        let (header, texture_data) = Self::load_data(hash, true)?;

        Self::load(
            gpu,
            &TextureDesc {
                dimension: header.dimension(),
                name: format!("texture_tag {hash}").into(),
                format: header.format.into(),
                width: header.width as u32,
                height: header.height as u32,
                depth: header.depth,
                array_size: header.array_size,
                num_mips: header.mip_count(),
            },
            &texture_data,
        )
    }

    pub fn load(gpu: &Arc<Gpu>, desc: &TextureDesc<'_>, data: &[u8]) -> anyhow::Result<Self> {
        let resource_desc = d3d12::ResourceDesc::new(desc.dimension)
            .width(desc.width as u64)
            .height(desc.height)
            .depth_or_array_size(desc.depth.max(desc.array_size))
            .format(desc.format)
            .mip_levels(desc.num_mips);

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

        let num_subresources = desc.num_mips as u32 * desc.array_size as u32;
        let footprint = gpu.get_copyable_footprints(&resource_desc, 0, num_subresources, 0)?;

        let upload_buffer = gpu.allocate_upload_buffer(footprint.total_bytes)?;
        unsafe {
            let dst_base = upload_buffer.resource().map(0)?;
            // create a mutable slice covering the entire upload buffer
            let dst = std::slice::from_raw_parts_mut(dst_base, footprint.total_bytes as usize);

            let mut src_offset: usize = 0;

            for (subresource_idx, layout) in footprint.layouts.iter().enumerate() {
                let mip = subresource_idx as u32 % desc.num_mips as u32;

                let mip_width = (desc.width >> mip).max(1);
                let mip_height = (desc.height >> mip).max(1);
                let mip_depth = (desc.depth >> mip).max(1) as u32;

                let (src_row_pitch, _) = desc.format.calculate_pitch(mip_width, mip_height);

                let block_height: usize = if desc.format.is_compressed() {
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
                            data.get(src_start..src_end)
                                .context("Source slice out of range")?,
                        );
                    }
                }

                src_offset += src_row_pitch * block_height * mip_depth as usize;
            }

            upload_buffer.resource().unmap(0);
        }

        // let upload_fence =
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

        let srv_desc = if desc.depth > 1 {
            ShaderResourceViewDesc::texture_3d(desc.format, 0, desc.num_mips as u32, 0.0)
        } else if desc.array_size > 1 {
            ShaderResourceViewDesc::texture_2d_array(
                desc.format,
                0,
                desc.num_mips as u32,
                0.0,
                0,
                0..desc.array_size as u32,
            )
        } else {
            ShaderResourceViewDesc::texture_2d(desc.format, 0, desc.num_mips as u32, 0.0, 0)
        };

        let srv = gpu.resource_heap.lock().allocate_srv(
            desc.name.to_string(),
            resource.resource(),
            &srv_desc,
        );

        resource.resource().set_debug_name(&desc.name);

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

pub struct TextureDesc<'a> {
    pub dimension: d3d12::ResourceDimension,
    pub name: Cow<'a, str>,
    pub format: d3d12::Format,
    pub width: u32,
    pub height: u32,
    pub depth: u16,
    pub array_size: u16,
    pub num_mips: u16,
}

impl<'a> TextureDesc<'a> {
    pub fn texture_2d(
        name: impl AsRef<str>,
        format: d3d12::Format,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            dimension: d3d12::ResourceDimension::Texture2D,
            name: Cow::Owned(name.as_ref().to_string()),
            format,
            width,
            height,
            depth: 1,
            array_size: 1,
            num_mips: 1,
        }
    }
}

pub fn generate_hdri_mips_rgbaf32(data: &[f32], width: usize, height: usize) -> Vec<Vec<f32>> {
    let num_mips = (width.min(height).ilog2() + 1) as usize;

    let mut src_chain: Vec<Vec<f32>> = vec![data.to_vec()];
    let mut src_widths = vec![width];
    let mut src_heights = vec![height];
    {
        let mut cur_w = width;
        let mut cur_h = height;
        while cur_w > 1 || cur_h > 1 {
            let prev = src_chain.last().unwrap();
            let next_w = (cur_w >> 1).max(1);
            let next_h = (cur_h >> 1).max(1);
            let mut next = vec![0f32; next_w * next_h * 4];
            for y in 0..next_h {
                for x in 0..next_w {
                    let fetch = |sx: usize, sy: usize| {
                        let i = (sy.min(cur_h - 1) * cur_w + sx.min(cur_w - 1)) * 4;
                        [prev[i], prev[i + 1], prev[i + 2], prev[i + 3]]
                    };
                    let p00 = fetch(x * 2, y * 2);
                    let p01 = fetch(x * 2 + 1, y * 2);
                    let p10 = fetch(x * 2, y * 2 + 1);
                    let p11 = fetch(x * 2 + 1, y * 2 + 1);
                    let i = (y * next_w + x) * 4;
                    for c in 0..4 {
                        next[i + c] = (p00[c] + p01[c] + p10[c] + p11[c]) * 0.25;
                    }
                }
            }
            src_chain.push(next);
            src_widths.push(next_w);
            src_heights.push(next_h);
            cur_w = next_w;
            cur_h = next_h;
        }
    }

    let texel_solid_angle = 4.0 * PI / (width * height) as f32;

    let mut mips: Vec<Vec<f32>> = Vec::with_capacity(num_mips);
    mips.push(data.to_vec());

    for mip in 1..num_mips {
        let mip_w = (width >> mip).max(1);
        let mip_h = (height >> mip).max(1);
        let roughness = mip as f32 / (num_mips - 1) as f32;
        let alpha = roughness * roughness;

        let pixel_rows: Vec<Vec<f32>> = (0..mip_h)
            .into_par_iter()
            .map(|y| {
                let mut row = vec![0.0; mip_w * 4];

                for x in 0..mip_w {
                    let u = (x as f32 + 0.5) / mip_w as f32;
                    let v = (y as f32 + 0.5) / mip_h as f32;
                    let r = uv_to_dir(u, v);

                    let mut accum = [0f32; 3];
                    let mut total_weight = 0f32;

                    for i in 0..NUM_SAMPLES {
                        let xi = hammersley(i, NUM_SAMPLES);
                        let h = importance_sample_ggx(xi, alpha, r);

                        let v_dot_h = r.dot(h).max(0.0);
                        let n_dot_h = v_dot_h;
                        let l = (2.0 * v_dot_h * h - r).normalize();
                        let n_o_l = r.dot(l).max(0.0);

                        if n_o_l > 0.0 {
                            let d = d_ggx(n_dot_h, alpha.max(0.001));
                            let pdf = (d * n_dot_h / 4.0f32.mul_add(v_dot_h, 1e-5)).max(1e-5);
                            let omega_s = 1.0 / (NUM_SAMPLES as f32 * pdf);
                            let src_mip = (0.5 * (omega_s / texel_solid_angle).log2()).max(0.0);

                            let s = sample_mip_bilinear(
                                &src_chain,
                                &src_widths,
                                &src_heights,
                                l,
                                src_mip,
                            );

                            accum[0] += s[0] * n_o_l;
                            accum[1] += s[1] * n_o_l;
                            accum[2] += s[2] * n_o_l;
                            total_weight += n_o_l;
                        }
                    }

                    let idx = x * 4;
                    if total_weight > 0.0 {
                        row[idx] = accum[0] / total_weight;
                        row[idx + 1] = accum[1] / total_weight;
                        row[idx + 2] = accum[2] / total_weight;
                    }
                    row[idx + 3] = 1.0;
                }
                row
            })
            .collect();

        mips.push(pixel_rows.into_iter().flatten().collect());
    }

    mips
}

pub fn generate_hdri_mips_rgbaf32_raw(data: &[u8], width: usize, height: usize) -> Vec<u8> {
    let data_f32 = bytemuck::cast_slice(data);
    let mips = generate_hdri_mips_rgbaf32(data_f32, width, height);
    let mut result = Vec::with_capacity(data.len() * 2);
    for mip in mips {
        result.extend_from_slice(bytemuck::cast_slice(&mip));
    }
    result
}
