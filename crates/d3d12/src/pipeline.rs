use std::{marker::PhantomData, mem::transmute_copy};

use windows::Win32::Graphics::{Direct3D12::*, Dxgi::Common::DXGI_SAMPLE_DESC};

use crate::{impl_device_child, BlendDesc, Format, InputElementDesc, RootSignature};

#[repr(transparent)]
#[derive(Clone)]
pub struct PipelineState(pub(crate) ID3D12PipelineState);
impl_device_child!(PipelineState);

impl PipelineState {
    pub fn get_cached_blob(&self) -> Vec<u8> {
        unsafe {
            let blob = self.0.GetCachedBlob().expect("Failed to get cached blob");
            let slice = std::slice::from_raw_parts(
                blob.GetBufferPointer() as *const u8,
                blob.GetBufferSize(),
            );

            slice.to_vec()
        }
    }
}

pub struct GraphicsPipelineStateDesc<'a> {
    pub(crate) inner: D3D12_GRAPHICS_PIPELINE_STATE_DESC,

    pub(crate) input_layout_elements: Vec<InputElementDesc>,

    _marker: PhantomData<&'a ()>,
}

impl<'a> GraphicsPipelineStateDesc<'a> {
    pub fn new(root_signature: &'a RootSignature) -> Self {
        Self {
            inner: D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                pRootSignature: unsafe { transmute_copy(&root_signature.0) },
                SampleMask: u32::MAX,
                SampleDesc: DXGI_SAMPLE_DESC {
                    Count: 1,
                    Quality: 0,
                },
                RasterizerState: D3D12_RASTERIZER_DESC {
                    FillMode: D3D12_FILL_MODE_SOLID,
                    CullMode: D3D12_CULL_MODE_NONE,
                    ..Default::default()
                },
                BlendState: D3D12_BLEND_DESC {
                    RenderTarget: [D3D12_RENDER_TARGET_BLEND_DESC {
                        RenderTargetWriteMask: D3D12_COLOR_WRITE_ENABLE_ALL.0 as u8,
                        ..Default::default()
                    }; 8],
                    ..Default::default()
                },
                IBStripCutValue: D3D12_INDEX_BUFFER_STRIP_CUT_VALUE_0xFFFF,
                PrimitiveTopologyType: D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE,
                ..Default::default()
            },
            input_layout_elements: vec![],
            _marker: PhantomData,
        }
    }

    pub const fn with_vs(mut self, bytecode: &'a [u8]) -> Self {
        self.inner.VS = D3D12_SHADER_BYTECODE {
            pShaderBytecode: bytecode.as_ptr().cast(),
            BytecodeLength: bytecode.len(),
        };
        self
    }

    pub const fn with_ps(mut self, bytecode: &'a [u8]) -> Self {
        self.inner.PS = D3D12_SHADER_BYTECODE {
            pShaderBytecode: bytecode.as_ptr().cast(),
            BytecodeLength: bytecode.len(),
        };
        self
    }

    pub const fn with_ds(mut self, bytecode: &'a [u8]) -> Self {
        self.inner.DS = D3D12_SHADER_BYTECODE {
            pShaderBytecode: bytecode.as_ptr().cast(),
            BytecodeLength: bytecode.len(),
        };
        self
    }

    pub const fn with_hs(mut self, bytecode: &'a [u8]) -> Self {
        self.inner.HS = D3D12_SHADER_BYTECODE {
            pShaderBytecode: bytecode.as_ptr().cast(),
            BytecodeLength: bytecode.len(),
        };
        self
    }

    pub const fn with_gs(mut self, bytecode: &'a [u8]) -> Self {
        self.inner.GS = D3D12_SHADER_BYTECODE {
            pShaderBytecode: bytecode.as_ptr().cast(),
            BytecodeLength: bytecode.len(),
        };
        self
    }

    pub fn with_input_layout(mut self, layout: &[InputElementDesc]) -> Self {
        self.input_layout_elements = layout.to_vec();

        self
    }

    pub const fn with_primitive_topology(mut self, topology: PrimitiveTopology2) -> Self {
        self.inner.PrimitiveTopologyType = D3D12_PRIMITIVE_TOPOLOGY_TYPE(topology as i32);
        self
    }

    /// Sets the render target view formats (up to 8).
    ///
    /// Only the first 8 formats are used, and any additional formats are ignored.
    pub fn with_rtv_formats(mut self, formats: &[Format]) -> Self {
        self.inner.RTVFormats =
            std::array::from_fn(|i| formats.get(i).copied().unwrap_or_default().into());
        self.inner.NumRenderTargets = formats.len().min(8) as u32;

        self
    }

    pub const fn with_depth_stencil_state(mut self, desc: D3D12_DEPTH_STENCIL_DESC) -> Self {
        self.inner.DepthStencilState = desc;
        self
    }

    pub fn with_dsv_format(mut self, format: Format) -> Self {
        self.inner.DSVFormat = format.into();
        self
    }

    pub const fn with_blend_state(mut self, desc: BlendDesc) -> Self {
        self.inner.BlendState = unsafe { desc.as_ffi().read() };
        self
    }

    pub const fn with_rasterizer_state(mut self, desc: D3D12_RASTERIZER_DESC) -> Self {
        self.inner.RasterizerState = desc;
        self
    }
}

#[repr(i32)]
pub enum PrimitiveTopology2 {
    Line = D3D12_PRIMITIVE_TOPOLOGY_TYPE_LINE.0,
    Patch = D3D12_PRIMITIVE_TOPOLOGY_TYPE_PATCH.0,
    Point = D3D12_PRIMITIVE_TOPOLOGY_TYPE_POINT.0,
    Triangle = D3D12_PRIMITIVE_TOPOLOGY_TYPE_TRIANGLE.0,
    Undefined = D3D12_PRIMITIVE_TOPOLOGY_TYPE_UNDEFINED.0,
}
