const PIX_EVENT_TYPE_BIT_SHIFT: u64 = 10;
const PIX_EVENT_BEGIN_NO_ARGS: u64 = 0x002 << PIX_EVENT_TYPE_BIT_SHIFT; // = 0x0800

// const PIX_EVENT_MARKER_NO_ARGS: u64 = 0x008 << PIX_EVENT_TYPE_BIT_SHIFT; // = 0x2000

const PIX_STRING_INFO_ANSI: u64 = (8_u64 << 55) | (1_u64 << 54); // = 0x0140_0000_0000_0000

pub const fn pix_color(r: u8, g: u8, b: u8) -> u64 {
    0xFF00_0000 | ((r as u64) << 16) | ((g as u64) << 8) | (b as u64)
}

pub fn pix3_begin_event_blob(color: u64, name: &str) -> Vec<u8> {
    pix3_blob(PIX_EVENT_BEGIN_NO_ARGS, color, name)
}

// pub fn pix3_set_marker_blob(color: u64, name: &str) -> Vec<u8> {
//     pix3_blob(PIX_EVENT_MARKER_NO_ARGS, color, name)
// }

fn pix3_blob(event_info: u64, color: u64, name: &str) -> Vec<u8> {
    let bytes = name.as_bytes();
    let string_qwords = (bytes.len() + 1).div_ceil(8);

    let mut blob = Vec::with_capacity((3 + string_qwords) * 8);
    blob.extend_from_slice(&event_info.to_le_bytes());
    blob.extend_from_slice(&color.to_le_bytes());
    blob.extend_from_slice(&PIX_STRING_INFO_ANSI.to_le_bytes());

    let mut string_data = vec![0u8; string_qwords * 8];
    string_data[..bytes.len()].copy_from_slice(bytes);
    blob.extend_from_slice(&string_data);

    blob
}
