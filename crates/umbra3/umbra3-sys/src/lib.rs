#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

// cohae: link_cplusplus needs to be referenced as external crate in order to link stdc++
#[allow(unused_extern_crates)]
extern crate link_cplusplus;

// pub mod query;
// pub mod tome;
