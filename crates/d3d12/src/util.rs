use std::ffi::CString;

use windows::{
    core::{Result, PCSTR},
    Win32::Graphics::Direct3D::ID3DBlob,
};

// Helper for working with DX11 methods that return results as HRESULT and out parameters as an Option<T>
pub fn wrap_option_out_result<T, F>(f: F) -> Result<T>
where
    F: FnOnce(Option<*mut Option<T>>) -> Result<()>,
{
    let mut result = None;
    f(Some(&raw mut result))?;
    Ok(result.unwrap())
}

pub fn wrap_out_result<T, F>(f: F) -> Result<T>
where
    F: FnOnce(*mut Option<T>) -> Result<()>,
{
    let mut result = None;
    f(&raw mut result)?;
    Ok(result.unwrap())
}

pub fn wrap_out_ptr<T: Default, F>(f: F) -> Result<T>
where
    F: FnOnce(*mut T) -> Result<()>,
{
    let mut result = T::default();
    f(&raw mut result)?;
    Ok(result)
}

/// Verifies that the given struct has the same size as the corresponding FFI struct, and generates an `as_ffi` method that returns a pointer to the FFI struct.
#[macro_export]
macro_rules! verify_ffi_struct {
    ($struct:ident, $ffi:ident) => {
        static_assertions::assert_eq_size!($struct, $ffi);

        impl $struct {
            #[allow(dead_code)]
            pub(crate) fn as_ffi(&self) -> *const $ffi {
                self as *const _ as _
            }
        }
    };
}

pub fn to_pcstr<S>(s: S) -> (CString, PCSTR)
where
    S: AsRef<str>,
{
    let cstr = CString::new(s.as_ref()).expect("Failed to convert string to CString");
    let pcstr = cstr.as_ptr();
    (cstr, PCSTR::from_raw(pcstr as _))
}

// pub fn to_pcwstr<S>(s: S) -> (Vec<u16>, PCWSTR)
// where
//     S: AsRef<str>,
// {
//     let wstr: Vec<u16> = s.as_ref().encode_utf16().chain(Some(0)).collect();
//     let pwstr = wstr.as_ptr();
//     (wstr, PCWSTR::from_raw(pwstr))
// }

pub trait OptionalParam: Sized {
    type Output;

    fn as_option(&self) -> Option<&Self::Output>;
}

impl<T> OptionalParam for &T {
    type Output = T;

    fn as_option(&self) -> Option<&Self::Output> {
        Some(self)
    }
}

impl<T> OptionalParam for Option<&T> {
    type Output = T;
    fn as_option(&self) -> Option<&Self::Output> {
        *self
    }
}

/// Helper macro to cast a slice of Option<&Resource> to a SmallVec of Option<NonNull<c_void>>
/// for use in D3D11 methods that take raw resource pointers.
/// # Arguments
/// * `$stack_count`: The number of elements to store on the stack before spilling to the heap.
/// * `$input_slice`: The input slice of Option<&Resource>.
#[macro_export]
macro_rules! cast_optional_resource_refs {
    ($stack_count:expr, $input_slice:ident) => {
        if $input_slice.len() > 1 {
            type TempStorage =
                smallvec::SmallVec<[Option<std::ptr::NonNull<std::ffi::c_void>>; $stack_count]>;
            let mut temp_storage = TempStorage::with_capacity($input_slice.len());
            for res in $input_slice.iter() {
                let ptr = res.as_ref().map(|b| std::mem::transmute_copy(*b));
                temp_storage.push(ptr);
            }
            temp_storage
        } else {
            let ptr: Option<std::ptr::NonNull<std::ffi::c_void>> = $input_slice
                .get(0)
                .and_then(|res| res.as_ref().map(|b| std::mem::transmute_copy(*b)));
            smallvec::smallvec![ptr]
        }
    };
}

pub fn blob_to_vec(blob: ID3DBlob) -> Vec<u8> {
    unsafe {
        let slice =
            std::slice::from_raw_parts(blob.GetBufferPointer().cast(), blob.GetBufferSize());
        slice.to_vec()
    }
}
