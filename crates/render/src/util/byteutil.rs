pub unsafe fn bytes_as_slice<'a, T>(bytes: &'a [u8]) -> &'a [T] {
    // assert_eq!(bytes.len() % std::mem::size_of::<T>(), 0);
    &*std::ptr::slice_from_raw_parts(
        bytes.as_ptr() as *const T,
        bytes.len() / std::mem::size_of::<T>(),
    )
}
