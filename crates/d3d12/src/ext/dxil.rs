use int_enum::IntEnum;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, IntEnum)]
pub enum PsvResourceType {
    Invalid = 0,

    Sampler,
    CBV,
    SRVTyped,
    SRVRaw,
    SRVStructured,
    UAVTyped,
    UAVRaw,
    UAVStructured,
    UAVStructuredWithCounter,
}

#[derive(Debug, Clone)]
pub struct PsvResourceBinding {
    pub res_type: PsvResourceType,
    pub space: u32,
    pub lower_bound: u32, // register slot
    pub upper_bound: u32, // for arrays; same as lower_bound if not an array
}

pub fn parse_psv0_resources(bytecode: &[u8]) -> Option<Vec<PsvResourceBinding>> {
    let chunk_data = find_dxbc_chunk(bytecode, b"PSV0")?;
    let mut cursor = 0usize;

    let runtime_info_size = read_u32(chunk_data, cursor) as usize;
    cursor += 4;
    cursor += runtime_info_size; // skip the runtime info header

    let resource_count = read_u32(chunk_data, cursor) as usize;
    cursor += 4;

    let bind_info_size = read_u32(chunk_data, cursor) as usize;
    cursor += 4;

    let mut bindings = Vec::with_capacity(resource_count);

    for _ in 0..resource_count {
        if cursor + bind_info_size > chunk_data.len() {
            break;
        }

        let res_type = read_u32(chunk_data, cursor);
        let space = read_u32(chunk_data, cursor + 4);
        let lower = read_u32(chunk_data, cursor + 8);
        let upper = read_u32(chunk_data, cursor + 12);

        bindings.push(PsvResourceBinding {
            res_type: PsvResourceType::try_from(res_type).unwrap(),
            space,
            lower_bound: lower,
            upper_bound: upper,
        });

        cursor += bind_info_size;
    }

    Some(bindings)
}

fn find_dxbc_chunk<'a>(bytecode: &'a [u8], fourcc: &[u8; 4]) -> Option<&'a [u8]> {
    if bytecode.len() < 0x20 || &bytecode[0..4] != b"DXBC" {
        return None;
    }

    let chunk_count = read_u32(bytecode, 0x1C) as usize;

    for i in 0..chunk_count {
        let offset_pos = 0x20 + i * 4;
        let chunk_offset = read_u32(bytecode, offset_pos) as usize;

        if &bytecode[chunk_offset..chunk_offset + 4] == fourcc {
            let chunk_size = read_u32(bytecode, chunk_offset + 4) as usize;
            let data_start = chunk_offset + 8;
            return Some(&bytecode[data_start..data_start + chunk_size]);
        }
    }

    None
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap())
}
