use std::marker::PhantomData;

use anyhow::Context;
use d3d11::{BindFlags, BufferDesc, CpuAccessFlags, SubresourceMapGuard, Usage};

use super::{Gpu, ShaderStage};

pub struct ConstantBuffer<T: Sized> {
    buffer: d3d11::Buffer,
    size: usize,
    _marker: PhantomData<T>,
}

impl<T> ConstantBuffer<T> {
    pub fn create(gpu: &Gpu, initial_data: Option<&T>) -> anyhow::Result<Self> {
        let size_aligned = (std::mem::size_of::<T>() + 15) & !15;

        let buffer = gpu.create_buffer(
            &BufferDesc::builder()
                .usage(Usage::Dynamic)
                .bind_flags(BindFlags::CONSTANT_BUFFER)
                .cpu_access_flags(CpuAccessFlags::WRITE)
                .byte_width(size_aligned as u32)
                .build(),
            initial_data.map(|d| unsafe {
                &*std::ptr::slice_from_raw_parts(
                    d as *const T as *const u8,
                    std::mem::size_of::<T>(),
                )
            }),
        )?;

        Ok(Self {
            buffer,
            size: size_aligned,
            _marker: Default::default(),
        })
    }

    pub fn create_array(
        gpu: &Gpu,
        count: usize,
        initial_data: Option<&[T]>,
    ) -> anyhow::Result<Self> {
        let size_aligned = ((std::mem::size_of::<T>() * count) + 15) & !15;
        let initial_data_ffi = if let Some(initial_data) = initial_data {
            anyhow::ensure!(
                initial_data.len() == count,
                "Initial data length does not match count"
            );

            Some(unsafe {
                &*std::ptr::slice_from_raw_parts(
                    initial_data.as_ptr() as *const u8,
                    std::mem::size_of::<T>() * count,
                )
            })
        } else {
            None
        };

        let buffer = gpu.create_buffer(
            &BufferDesc::builder()
                .usage(Usage::Dynamic)
                .bind_flags(BindFlags::CONSTANT_BUFFER)
                .cpu_access_flags(CpuAccessFlags::WRITE)
                .byte_width(size_aligned as u32)
                .build(),
            initial_data_ffi,
        )?;

        Ok(Self {
            buffer,
            size: size_aligned,
            _marker: Default::default(),
        })
    }

    pub fn write(&self, ctx: &d3d11::DeviceContext, data: &T) -> anyhow::Result<()> {
        self.map(ctx, d3d11::MapType::WriteDiscard, |map| unsafe {
            map.data
                .copy_from_nonoverlapping(data as *const T as _, std::mem::size_of::<T>());
        })
    }

    /// SAFETY: The caller must ensure that the length of the slice matches the size of the buffer.
    pub unsafe fn write_array(&self, ctx: &d3d11::DeviceContext, data: &[T]) -> anyhow::Result<()> {
        self.map(ctx, d3d11::MapType::WriteDiscard, |map| unsafe {
            map.data.copy_from_nonoverlapping(
                data.as_ptr() as *const T as _,
                std::mem::size_of_val(data),
            );
        })
    }

    pub fn map(
        &self,
        ctx: &d3d11::DeviceContext,
        mode: d3d11::MapType,
        f: impl FnOnce(SubresourceMapGuard<d3d11::Buffer>),
    ) -> anyhow::Result<()> {
        let ptr = ctx
            .map(&self.buffer, 0, mode, false)
            .context("Failed to map ConstantBuffer")?;

        f(ptr);

        Ok(())
    }

    pub fn map_slice(
        &self,
        ctx: &d3d11::DeviceContext,
        mode: d3d11::MapType,
        f: impl FnOnce(&mut [T]),
    ) -> anyhow::Result<()> {
        self.map(ctx, mode, |map| {
            let slice = unsafe {
                std::slice::from_raw_parts_mut(
                    map.data as *mut T,
                    self.size / std::mem::size_of::<T>(),
                )
            };
            f(slice);
        })
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn buffer(&self) -> &d3d11::Buffer {
        &self.buffer
    }

    pub fn bind(&self, context: &d3d11::DeviceContext, stage: ShaderStage, slot: u32) {
        match stage {
            ShaderStage::Vertex => {
                context.vertex_set_constant_buffers(slot, &[Some(self.buffer.clone())])
            }
            ShaderStage::Pixel => {
                context.pixel_set_constant_buffers(slot, &[Some(self.buffer.clone())])
            }
            ShaderStage::Domain => {
                context.domain_set_constant_buffers(slot, &[Some(self.buffer.clone())])
            }
            ShaderStage::Hull => {
                context.hull_set_constant_buffers(slot, &[Some(self.buffer.clone())])
            }
            ShaderStage::Geometry => {
                context.geometry_set_constant_buffers(slot, &[Some(self.buffer.clone())])
            }
            ShaderStage::Compute => {
                context.compute_set_constant_buffers(slot, &[Some(self.buffer.clone())])
            }
        }
    }
}
