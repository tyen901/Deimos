#[allow(clippy::missing_safety_doc)]
pub const unsafe fn bytes_as_slice<T>(bytes: &[u8]) -> &[T] {
    unsafe {
        // assert_eq!(bytes.len() % std::mem::size_of::<T>(), 0);
        &*std::ptr::slice_from_raw_parts(
            bytes.as_ptr().cast::<T>(),
            bytes.len() / std::mem::size_of::<T>(),
        )
    }
}
