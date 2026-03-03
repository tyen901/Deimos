use windows::Win32::Graphics::Direct3D12::*;

use crate::verify_ffi_type;

#[repr(transparent)]
#[derive(Clone)]
pub struct QueryHeap(pub(crate) ID3D12QueryHeap);

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryHeapType {
    CopyQueueTimestamp = D3D12_QUERY_HEAP_TYPE_COPY_QUEUE_TIMESTAMP.0,
    Occlusion = D3D12_QUERY_HEAP_TYPE_OCCLUSION.0,
    PipelineStatistics = D3D12_QUERY_HEAP_TYPE_PIPELINE_STATISTICS.0,
    PipelineStatistics1 = D3D12_QUERY_HEAP_TYPE_PIPELINE_STATISTICS1.0,
    SoStatistics = D3D12_QUERY_HEAP_TYPE_SO_STATISTICS.0,
    Timestamp = D3D12_QUERY_HEAP_TYPE_TIMESTAMP.0,
    VideoDecodeStatistics = D3D12_QUERY_HEAP_TYPE_VIDEO_DECODE_STATISTICS.0,
}
verify_ffi_type!(QueryHeapType, D3D12_QUERY_HEAP_TYPE);

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QueryType {
    BinaryOcclusion = D3D12_QUERY_TYPE_BINARY_OCCLUSION.0,
    Occlusion = D3D12_QUERY_TYPE_OCCLUSION.0,
    PipelineStatistics = D3D12_QUERY_TYPE_PIPELINE_STATISTICS.0,
    PipelineStatistics1 = D3D12_QUERY_TYPE_PIPELINE_STATISTICS1.0,
    SoStatisticsStream0 = D3D12_QUERY_TYPE_SO_STATISTICS_STREAM0.0,
    SoStatisticsStream1 = D3D12_QUERY_TYPE_SO_STATISTICS_STREAM1.0,
    SoStatisticsStream2 = D3D12_QUERY_TYPE_SO_STATISTICS_STREAM2.0,
    SoStatisticsStream3 = D3D12_QUERY_TYPE_SO_STATISTICS_STREAM3.0,
    Timestamp = D3D12_QUERY_TYPE_TIMESTAMP.0,
    VideoDecodeStatistics = D3D12_QUERY_TYPE_VIDEO_DECODE_STATISTICS.0,
}
verify_ffi_type!(QueryType, D3D12_QUERY_TYPE);
