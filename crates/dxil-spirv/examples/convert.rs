use dxil_spirv::*;

fn main() {
    let data = include_bytes!("80B447C5.ps.cso");

    let hlsl = convert_dxil_to_hlsl(data).expect("Failed to convert DXIL to HLSL");
    println!("{hlsl}");
}
