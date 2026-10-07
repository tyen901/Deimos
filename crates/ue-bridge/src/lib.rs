//! Package-only C boundary. Caller owns scheduling; no UE or GPU objects cross it.
pub mod entity;
pub mod animation;
use std::{ffi::{c_char,CStr}, panic::{catch_unwind,AssertUnwindSafe},ptr};
use tiger_pkg::{PackageManager,GameVersion,MarathonVersion,TagHash};
#[repr(C)]
pub struct MaraBytes { pub data: *mut u8, pub len: usize }
fn bytes(data:Vec<u8>) -> MaraBytes { let mut data=data.into_boxed_slice(); let out=MaraBytes{data:data.as_mut_ptr(),len:data.len()}; std::mem::forget(data);out }
fn boundary(f:impl FnOnce()->anyhow::Result<Vec<u8>>,out:*mut MaraBytes)->i32 {
    if out.is_null(){return -1;}
    let result=catch_unwind(AssertUnwindSafe(f));
    let (status,data)=match result {Ok(Ok(data))=>(0,data),Ok(Err(e))=>(1,format!("{e:#}").into_bytes()),Err(_)=>(2,b"package reader panic".to_vec())};
    unsafe{*out=bytes(data)};status
}
pub unsafe extern "C" fn mara_open(path:*const c_char, context:*mut *mut PackageManager,out:*mut MaraBytes)->i32 {
    boundary(||{
        anyhow::ensure!(!path.is_null() && !context.is_null(),"null open argument");
        unsafe{*context=ptr::null_mut()};
        let path=unsafe{CStr::from_ptr(path)}.to_str()?;
        let index_workers=rayon::ThreadPoolBuilder::new().num_threads(2).thread_name(|i|format!("MarathonIndex{i}")).build()?;
        let manager=index_workers.install(||PackageManager::new(path,GameVersion::Marathon(MarathonVersion::Marathon),None))?;
        anyhow::ensure!(manager.package_paths.len()==manager.lookup.tag32_entries_by_pkg.len(),"incomplete package lookup");
        unsafe{*context=Box::into_raw(Box::new(manager))};Ok(Vec::new())
    },out)
}
pub unsafe extern "C" fn mara_character(context:*const PackageManager,tag:u32,out:*mut MaraBytes)->i32 {
    boundary(||{anyhow::ensure!(!context.is_null(),"null reader");entity::read_character(unsafe{&*context},TagHash(tag))},out)
}
pub unsafe extern "C" fn mara_clip(context:*const PackageManager,tag:u32,out:*mut MaraBytes)->i32 {
    boundary(||{anyhow::ensure!(!context.is_null(),"null reader");animation::read_clip(unsafe{&*context},TagHash(tag))},out)
}
/// No reads may be active when close is called.
pub unsafe extern "C" fn mara_close(context:*mut PackageManager){if !context.is_null(){drop(unsafe{Box::from_raw(context)});}}
pub unsafe extern "C" fn mara_free(data:MaraBytes){if !data.data.is_null(){drop(unsafe{Box::from_raw(ptr::slice_from_raw_parts_mut(data.data,data.len))});}}

/// Read an exact package entry; lookup/decrypt/decompress stays inside tiger-pkg.
pub unsafe extern "C" fn mara_tag(context:*const PackageManager,tag:u32,out:*mut MaraBytes)->i32 {
    boundary(||{anyhow::ensure!(!context.is_null(),"null reader");
        let result=unsafe{&*context}.read_tag(TagHash(tag));
        result.map_err(|e|anyhow::anyhow!("tag {tag:08X}: {e:#}"))
    },out)
}
#[repr(C)]
pub struct MaraEntry {pub reference:u32,pub size:u32,pub file_type:u8,pub file_subtype:u8}
pub unsafe extern "C" fn mara_entry(context:*const PackageManager,tag:u32,entry:*mut MaraEntry)->i32 {
    if context.is_null() || entry.is_null(){return 1;}
    match unsafe{&*context}.get_entry(TagHash(tag)) {
        Some(e)=>{unsafe{*entry=MaraEntry{reference:e.reference,size:e.file_size,file_type:e.file_type,file_subtype:e.file_subtype}};0}
        None=>1
    }
}

/// Wide source identities resolve through the package manager's actual hash table.
pub unsafe extern "C" fn mara_resolve64(context:*const PackageManager,wide:u64,tag:*mut u32)->i32 {
    if context.is_null() || tag.is_null(){return 1;}
    match unsafe{&*context}.lookup.tag64_entries.get(&wide) {
        Some(entry)=>{unsafe{*tag=entry.hash32.0};0}
        None=>1
    }
}
