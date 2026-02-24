use crate::gpu::alloc::resource::OwnedResource;

/// Simple upload buffer for a single GPU resource. Should be avoided for large/repeated uploads.
pub struct UploadBuffer {
    pub resource: OwnedResource,
}
