use bitflags::bitflags;
use windows::Win32::Graphics::Direct3D12::*;

use crate::verify_ffi_type;

#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct HeapProperties(pub(crate) D3D12_HEAP_PROPERTIES);
verify_ffi_type!(HeapProperties, D3D12_HEAP_PROPERTIES);

impl HeapProperties {
    pub fn new(heap_type: HeapType) -> Self {
        Self(D3D12_HEAP_PROPERTIES {
            Type: heap_type.into(),
            CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
            MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
            CreationNodeMask: 1,
            VisibleNodeMask: 1,
        })
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct HeapFlags : i32 {
        const ALLOW_ALL_BUFFERS_AND_TEXTURES = D3D12_HEAP_FLAG_ALLOW_ALL_BUFFERS_AND_TEXTURES.0;
        const ALLOW_DISPLAY = D3D12_HEAP_FLAG_ALLOW_DISPLAY.0;
        const ALLOW_ONLY_BUFFERS = D3D12_HEAP_FLAG_ALLOW_ONLY_BUFFERS.0;
        const ALLOW_ONLY_NON_RT_DS_TEXTURES = D3D12_HEAP_FLAG_ALLOW_ONLY_NON_RT_DS_TEXTURES.0;
        const ALLOW_ONLY_RT_DS_TEXTURES = D3D12_HEAP_FLAG_ALLOW_ONLY_RT_DS_TEXTURES.0;
        const ALLOW_SHADER_ATOMICS = D3D12_HEAP_FLAG_ALLOW_SHADER_ATOMICS.0;
        const ALLOW_WRITE_WATCH = D3D12_HEAP_FLAG_ALLOW_WRITE_WATCH.0;
        const CREATE_NOT_RESIDENT = D3D12_HEAP_FLAG_CREATE_NOT_RESIDENT.0;
        const CREATE_NOT_ZEROED = D3D12_HEAP_FLAG_CREATE_NOT_ZEROED.0;
        const DENY_BUFFERS = D3D12_HEAP_FLAG_DENY_BUFFERS.0;
        const DENY_NON_RT_DS_TEXTURES = D3D12_HEAP_FLAG_DENY_NON_RT_DS_TEXTURES.0;
        const DENY_RT_DS_TEXTURES = D3D12_HEAP_FLAG_DENY_RT_DS_TEXTURES.0;
        const HARDWARE_PROTECTED = D3D12_HEAP_FLAG_HARDWARE_PROTECTED.0;
        const SHARED = D3D12_HEAP_FLAG_SHARED.0;
        const SHARED_CROSS_ADAPTER = D3D12_HEAP_FLAG_SHARED_CROSS_ADAPTER.0;
        const TOOLS_USE_MANUAL_WRITE_TRACKING = D3D12_HEAP_FLAG_TOOLS_USE_MANUAL_WRITE_TRACKING.0;
    }
}
verify_ffi_type!(HeapFlags, D3D12_HEAP_FLAGS);

#[repr(i32)]
#[derive(Debug, Clone, Copy)]
pub enum HeapType {
    Custom = D3D12_HEAP_TYPE_CUSTOM.0,
    Default = D3D12_HEAP_TYPE_DEFAULT.0,
    GpuUpload = D3D12_HEAP_TYPE_GPU_UPLOAD.0,
    Readback = D3D12_HEAP_TYPE_READBACK.0,
    Upload = D3D12_HEAP_TYPE_UPLOAD.0,
}

verify_ffi_type!(HeapType, D3D12_HEAP_TYPE);
