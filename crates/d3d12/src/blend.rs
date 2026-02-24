use bon::Builder;
use windows::{core::BOOL, Win32::Graphics::Direct3D12::*};

use crate::verify_ffi_type;

#[repr(C)]
#[derive(Builder, Clone, Default, Debug)]
pub struct BlendDesc {
    #[builder(into)]
    pub alpha_to_coverage_enable: BOOL,
    #[builder(into)]
    pub independent_blend_enable: BOOL,
    pub render_target: [RenderTargetBlendDesc; 8],
}
verify_ffi_type!(BlendDesc, D3D12_BLEND_DESC);

impl BlendDesc {
    pub const DISABLED: Self = Self {
        alpha_to_coverage_enable: BOOL(0),
        independent_blend_enable: BOOL(0),
        render_target: [RenderTargetBlendDesc::DISABLED; 8],
    };

    pub fn from_single_target(target: RenderTargetBlendDesc) -> Self {
        Self {
            alpha_to_coverage_enable: false.into(),
            independent_blend_enable: true.into(),
            render_target: std::array::from_fn(|_| target.clone()),
        }
    }
}

#[repr(C)]
#[derive(Builder, Clone, Debug)]
pub struct RenderTargetBlendDesc {
    #[builder(into)]
    pub blend_enable: BOOL,
    #[builder(into)]
    pub logic_op_enable: BOOL,
    pub src_blend: Blend,
    pub dest_blend: Blend,
    pub blend_op: BlendOp,
    pub src_blend_alpha: Blend,
    pub dest_blend_alpha: Blend,
    pub blend_op_alpha: BlendOp,
    pub logic_op: LogicOp,
    pub render_target_write_mask: u8, // TODO: Bitflags
}
verify_ffi_type!(RenderTargetBlendDesc, D3D12_RENDER_TARGET_BLEND_DESC);

impl RenderTargetBlendDesc {
    pub const DISABLED: Self = Self {
        blend_enable: BOOL(0),
        logic_op_enable: BOOL(0),
        src_blend: Blend::One,
        dest_blend: Blend::Zero,
        blend_op: BlendOp::Add,
        src_blend_alpha: Blend::One,
        dest_blend_alpha: Blend::Zero,
        blend_op_alpha: BlendOp::Add,
        logic_op: LogicOp::Noop,
        render_target_write_mask: 0,
    };
}

impl Default for RenderTargetBlendDesc {
    fn default() -> Self {
        Self::DISABLED
    }
}

#[repr(i32)]
#[derive(Clone, Debug)]
pub enum Blend {
    Zero = D3D12_BLEND_ZERO.0,
    One = D3D12_BLEND_ONE.0,
    SrcColor = D3D12_BLEND_SRC_COLOR.0,
    InvSrcColor = D3D12_BLEND_INV_SRC_COLOR.0,
    SrcAlpha = D3D12_BLEND_SRC_ALPHA.0,
    InvSrcAlpha = D3D12_BLEND_INV_SRC_ALPHA.0,
    DestAlpha = D3D12_BLEND_DEST_ALPHA.0,
    InvDestAlpha = D3D12_BLEND_INV_DEST_ALPHA.0,
    DestColor = D3D12_BLEND_DEST_COLOR.0,
    InvDestColor = D3D12_BLEND_INV_DEST_COLOR.0,
    SrcAlphaSat = D3D12_BLEND_SRC_ALPHA_SAT.0,
    BlendFactor = D3D12_BLEND_BLEND_FACTOR.0,
    InvBlendFactor = D3D12_BLEND_INV_BLEND_FACTOR.0,
    Src1Color = D3D12_BLEND_SRC1_COLOR.0,
    InvSrc1Color = D3D12_BLEND_INV_SRC1_COLOR.0,
    Src1Alpha = D3D12_BLEND_SRC1_ALPHA.0,
    InvSrc1Alpha = D3D12_BLEND_INV_SRC1_ALPHA.0,
}

#[repr(i32)]
#[derive(Clone, Debug)]
pub enum BlendOp {
    Add = D3D12_BLEND_OP_ADD.0,
    Subtract = D3D12_BLEND_OP_SUBTRACT.0,
    RevSubtract = D3D12_BLEND_OP_REV_SUBTRACT.0,
    Min = D3D12_BLEND_OP_MIN.0,
    Max = D3D12_BLEND_OP_MAX.0,
}

#[repr(i32)]
#[derive(Clone, Debug)]
pub enum LogicOp {
    And = D3D12_LOGIC_OP_AND.0,
    AndInverted = D3D12_LOGIC_OP_AND_INVERTED.0,
    AndReverse = D3D12_LOGIC_OP_AND_REVERSE.0,
    Clear = D3D12_LOGIC_OP_CLEAR.0,
    Copy = D3D12_LOGIC_OP_COPY.0,
    CopyInverted = D3D12_LOGIC_OP_COPY_INVERTED.0,
    Equiv = D3D12_LOGIC_OP_EQUIV.0,
    Invert = D3D12_LOGIC_OP_INVERT.0,
    Nand = D3D12_LOGIC_OP_NAND.0,
    Noop = D3D12_LOGIC_OP_NOOP.0,
    Nor = D3D12_LOGIC_OP_NOR.0,
    Or = D3D12_LOGIC_OP_OR.0,
    OrInverted = D3D12_LOGIC_OP_OR_INVERTED.0,
    OrReverse = D3D12_LOGIC_OP_OR_REVERSE.0,
    Set = D3D12_LOGIC_OP_SET.0,
    Xor = D3D12_LOGIC_OP_XOR.0,
}
