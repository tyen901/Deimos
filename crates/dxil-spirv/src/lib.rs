#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::mem::zeroed;

use anyhow::Context;
use spirv_cross2::{
    compile::hlsl::{CompilerOptions, HlslShaderModel},
    targets::Hlsl,
    Module,
};

mod private {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

pub fn convert_dxil_to_hlsl(dxil: &[u8]) -> anyhow::Result<String> {
    use private::*;
    unsafe {
        let mut dxil_blob = std::ptr::null_mut();
        if dxil_spv_parse_dxil_blob(dxil.as_ptr().cast(), dxil.len(), &mut dxil_blob) < 0 {
            anyhow::bail!("Failed to parse DXIL blob");
        }

        let mut converter = std::ptr::null_mut();
        if dxil_spv_create_converter(dxil_blob, &mut converter) < 0 {
            anyhow::bail!("Failed to create converter");
        }

        if dxil_spv_converter_run(converter) < 0 {
            anyhow::bail!("Failed to convert DXIL to SPIR-V");
        }

        let mut compiled_spirv = zeroed();
        if dxil_spv_converter_get_compiled_spirv(converter, &mut compiled_spirv) < 0 {
            anyhow::bail!("Failed to get compiled SPIR-V");
        }

        let words =
            std::slice::from_raw_parts(compiled_spirv.data.cast::<u32>(), compiled_spirv.size / 4);

        let compiler = spirv_cross2::Compiler::<Hlsl>::new(Module::from_words(words))
            .context("Failed to create spirv-cross session")?;

        let mut options = CompilerOptions::default();
        options.shader_model = HlslShaderModel::ShaderModel5_1;

        let res = compiler.compile(&options).context("Failed to compile")?;

        dxil_spv_converter_free(converter);
        dxil_spv_parsed_blob_free(dxil_blob);

        Ok(res.to_string())
    }
}

pub fn convert_dxil_to_dxbc(
    dxil: &[u8],
    target: d3d11::fxc::ShaderTarget,
) -> anyhow::Result<Vec<u8>> {
    let hlsl = convert_dxil_to_hlsl(dxil).expect("Failed to convert DXIL to HLSL");
    let dxbc = d3d11::fxc::compile(hlsl.as_bytes(), None, &[], "main", target)?;
    Ok(dxbc)
}
