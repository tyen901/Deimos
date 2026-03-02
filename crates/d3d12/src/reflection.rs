use std::{ffi::c_void, ptr::null_mut};

use windows::{
    core::Interface,
    Win32::Graphics::{
        Direct3D::{
            Fxc::D3DReflect, D3D_RESOURCE_RETURN_TYPE, D3D_SHADER_INPUT_TYPE, D3D_SRV_DIMENSION,
        },
        Direct3D12::{ID3D12ShaderReflection, D3D12_SHADER_DESC, D3D12_SHADER_INPUT_BIND_DESC},
    },
};

use crate::Result;

#[repr(transparent)]
#[derive(Clone)]
pub struct Reflection(pub(crate) ID3D12ShaderReflection);

impl Reflection {
    pub fn new(bytecode: &[u8]) -> Result<Self> {
        unsafe {
            let mut ppv = null_mut::<c_void>();
            D3DReflect(
                bytecode.as_ptr().cast(),
                bytecode.len(),
                &ID3D12ShaderReflection::IID,
                &mut ppv,
            )?;
            Ok(Self(ID3D12ShaderReflection::from_raw(ppv)))
        }
    }

    pub fn desc(&self) -> Result<D3D12_SHADER_DESC> {
        let mut desc = D3D12_SHADER_DESC::default();
        unsafe { self.0.GetDesc(&mut desc)? };
        Ok(desc)
    }

    pub fn resource_binding_desc(&self, index: u32) -> Option<ShaderInputBindDesc> {
        unsafe {
            let mut desc = D3D12_SHADER_INPUT_BIND_DESC::default();
            self.0.GetResourceBindingDesc(index, &mut desc).ok()?;
            let name = desc.Name.display().to_string();
            Some(ShaderInputBindDesc {
                name,
                type_: desc.Type,
                bind_point: desc.BindPoint,
                bind_count: desc.BindCount,
                return_type: desc.ReturnType,
                dimension: desc.Dimension,
                num_samples: desc.NumSamples,
                space: desc.Space,
                uid: desc.uID,
            })
        }
    }

    pub const fn iter_resource_binding_desc(&self) -> ResourceBindingDescIter<'_> {
        ResourceBindingDescIter {
            reflection: self,
            index: 0,
        }
    }
}

pub struct ResourceBindingDescIter<'a> {
    reflection: &'a Reflection,
    index: u32,
}

impl<'a> Iterator for ResourceBindingDescIter<'a> {
    type Item = ShaderInputBindDesc;

    fn next(&mut self) -> Option<Self::Item> {
        self.index
            .checked_add(1)
            .and_then(|i| self.reflection.resource_binding_desc(i))
    }
}

#[derive(Debug, Clone)]
pub struct ShaderInputBindDesc {
    pub name: String,
    pub type_: D3D_SHADER_INPUT_TYPE,
    pub bind_point: u32,
    pub bind_count: u32,
    pub return_type: D3D_RESOURCE_RETURN_TYPE,
    pub dimension: D3D_SRV_DIMENSION,
    pub num_samples: u32,
    pub space: u32,
    pub uid: u32,
}
