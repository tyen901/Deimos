use windows::{
    core::{IUnknown, Interface},
    Win32::Graphics::{Direct3D::D3D_FEATURE_LEVEL_11_0, Direct3D12::*, Dxgi::IDXGIAdapter},
};

use crate::{
    error::Result, util::to_pcstr, CommandAllocator, CommandListType, CommandQueue,
    CommandQueueDesc, CopyableFootprints, CpuDescriptorHandle, DepthStencilViewDesc,
    DescriptorHeap, DescriptorHeapType, ElementOffset, Fence, FenceFlags, GraphicsCommandList,
    GraphicsPipelineStateDesc, HeapFlags, HeapProperties, PipelineState, RenderTargetViewDesc,
    Resource, ResourceDesc, ResourceStates, RootSignature, ShaderResourceViewDesc,
};

#[repr(transparent)]
#[derive(Clone)]
pub struct Device(pub(crate) ID3D12Device);

impl Device {
    pub fn create(adapter: Option<IDXGIAdapter>) -> Result<Self> {
        let mut device = None;
        unsafe {
            D3D12CreateDevice(
                adapter.map(|a| a.cast::<IUnknown>().unwrap()).as_ref(),
                D3D_FEATURE_LEVEL_11_0,
                &mut device,
            )?;
        }

        Ok(Self(device.expect("created device should not be null")))
    }

    pub fn create_graphics_pipeline_state(
        &self,
        desc: &GraphicsPipelineStateDesc,
    ) -> Result<PipelineState> {
        let mut cstrings = vec![];
        let mut input_layout_ffi = vec![];

        for d in &desc.input_layout_elements {
            let (cstring, pstring) = to_pcstr(&d.semantic_name);
            cstrings.push(cstring);

            input_layout_ffi.push(D3D12_INPUT_ELEMENT_DESC {
                SemanticName: pstring,
                SemanticIndex: d.semantic_index,
                Format: d.format.into(),
                InputSlot: d.input_slot,
                AlignedByteOffset: match d.aligned_byte_offset {
                    ElementOffset::Append => D3D12_APPEND_ALIGNED_ELEMENT,
                    ElementOffset::Absolute(offset) => offset,
                },
                InputSlotClass: d.input_slot_class.clone().into(),
                InstanceDataStepRate: d.instance_data_step_rate,
            });
        }

        let mut desc_raw = desc.inner.clone();
        desc_raw.InputLayout = D3D12_INPUT_LAYOUT_DESC {
            NumElements: input_layout_ffi.len() as u32,
            pInputElementDescs: input_layout_ffi.as_ptr(),
        };

        let pipeline_state: ID3D12PipelineState =
            unsafe { self.0.CreateGraphicsPipelineState(&raw const desc_raw)? };

        Ok(PipelineState(pipeline_state))
    }

    pub fn create_root_signature(&self, data: &[u8]) -> Result<RootSignature> {
        let root_signature: ID3D12RootSignature = unsafe { self.0.CreateRootSignature(0, data)? };

        Ok(RootSignature(root_signature))
    }

    pub fn create_command_queue(&self, desc: &CommandQueueDesc) -> Result<CommandQueue> {
        let command_queue: ID3D12CommandQueue =
            unsafe { self.0.CreateCommandQueue((&raw const *desc).cast())? };

        Ok(CommandQueue(command_queue))
    }

    pub fn create_command_allocator(&self, type_: CommandListType) -> Result<CommandAllocator> {
        let command_allocator: ID3D12CommandAllocator = unsafe {
            self.0
                .CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE(type_ as i32))?
        };

        Ok(CommandAllocator(command_allocator))
    }

    // Only supports graphics command lists for now
    pub fn create_command_list(
        &self,
        node_mask: u32,
        type_: CommandListType,
        allocator: &CommandAllocator,
        initial_state: Option<&PipelineState>,
    ) -> Result<GraphicsCommandList> {
        let command_list: ID3D12GraphicsCommandList = unsafe {
            self.0.CreateCommandList(
                node_mask,
                D3D12_COMMAND_LIST_TYPE(type_ as i32),
                &allocator.0,
                initial_state.map(|s| &s.0),
            )?
        };

        Ok(GraphicsCommandList(command_list))
    }

    pub fn create_committed_resource(
        &self,
        properties: &HeapProperties,
        flags: HeapFlags,
        desc: &ResourceDesc,
        initial_state: ResourceStates,
    ) -> Result<Resource> {
        let mut resource = None;
        unsafe {
            self.0.CreateCommittedResource(
                properties.as_ffi(),
                flags.into(),
                desc.as_ffi(),
                initial_state.into(),
                None,
                &mut resource,
            )?;
        }

        Ok(Resource(resource.unwrap()))
    }

    pub fn create_fence(&self, initial_value: u64) -> Result<Fence> {
        self.create_fence_ex(initial_value, FenceFlags::empty())
    }

    pub fn create_fence_ex(&self, initial_value: u64, flags: FenceFlags) -> Result<Fence> {
        let fence: ID3D12Fence = unsafe {
            self.0
                .CreateFence(initial_value, D3D12_FENCE_FLAGS(flags.bits()))?
        };

        Ok(Fence(fence))
    }

    pub fn create_render_target_view(
        &self,
        resource: Option<&Resource>,
        desc: Option<&RenderTargetViewDesc>,
        dest_descriptor: CpuDescriptorHandle,
    ) {
        unsafe {
            self.0.CreateRenderTargetView(
                resource.map(|r| &r.0),
                desc.map(|d| d.as_ffi()),
                dest_descriptor.into(),
            );
        }
    }

    pub fn create_depth_stencil_view(
        &self,
        resource: Option<&Resource>,
        desc: Option<&DepthStencilViewDesc>,
        dest_descriptor: CpuDescriptorHandle,
    ) {
        unsafe {
            self.0.CreateDepthStencilView(
                resource.map(|r| &r.0),
                desc.map(|d| d.as_ffi()),
                dest_descriptor.into(),
            );
        }
    }

    pub fn create_shader_resource_view(
        &self,
        resource: Option<&Resource>,
        desc: Option<&ShaderResourceViewDesc>,
        dest_descriptor: CpuDescriptorHandle,
    ) {
        unsafe {
            self.0.CreateShaderResourceView(
                resource.map(|r| &r.0),
                desc.map(|d| &raw const d.0),
                dest_descriptor.into(),
            );
        }
    }

    pub fn get_copyable_footprints(
        &self,
        resource_desc: &ResourceDesc,
        first_subresource: u32,
        num_subresources: u32,
        base_offset: u64,
    ) -> Result<CopyableFootprints> {
        let mut f = CopyableFootprints {
            layouts: vec![unsafe { std::mem::zeroed() }; num_subresources as usize],
            num_rows: vec![0; num_subresources as usize],
            row_size_in_bytes: vec![0; num_subresources as usize],
            total_bytes: 0,
        };
        unsafe {
            self.0.GetCopyableFootprints(
                &resource_desc.0,
                first_subresource,
                num_subresources,
                base_offset,
                Some(f.layouts.as_mut_ptr().cast()),
                Some(f.num_rows.as_mut_ptr()),
                Some(f.row_size_in_bytes.as_mut_ptr()),
                Some(&mut f.total_bytes),
            );
        }
        if f.total_bytes == u64::MAX {
            return Err(crate::Error::Other("Invalid resource_desc".to_string()));
        }

        Ok(f)
    }

    pub fn create_descriptor_heap(
        &self,
        type_: DescriptorHeapType,
        num_descriptors: u32,
        shader_visible: bool,
        node_mask: u32,
    ) -> Result<DescriptorHeap> {
        let descriptor_heap: ID3D12DescriptorHeap = unsafe {
            self.0.CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
                Type: D3D12_DESCRIPTOR_HEAP_TYPE(type_ as i32),
                NumDescriptors: num_descriptors,
                Flags: if shader_visible {
                    D3D12_DESCRIPTOR_HEAP_FLAG_SHADER_VISIBLE
                } else {
                    D3D12_DESCRIPTOR_HEAP_FLAG_NONE
                },
                NodeMask: node_mask,
            })?
        };

        Ok(DescriptorHeap(descriptor_heap))
    }

    pub fn descriptor_handle_increment_size(&self, type_: DescriptorHeapType) -> u32 {
        unsafe {
            self.0
                .GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE(type_ as i32))
        }
    }

    pub fn copy_descriptors_simple(
        &self,
        num_descriptors: u32,
        src_descriptor_range_start: CpuDescriptorHandle,
        dst_descriptor_range_start: CpuDescriptorHandle,
        heap_type: DescriptorHeapType,
    ) {
        unsafe {
            self.0.CopyDescriptorsSimple(
                num_descriptors,
                dst_descriptor_range_start.into(),
                src_descriptor_range_start.into(),
                D3D12_DESCRIPTOR_HEAP_TYPE(heap_type as i32),
            );
        }
    }

    pub fn as_windows(&self) -> &ID3D12Device {
        &self.0
    }
}
