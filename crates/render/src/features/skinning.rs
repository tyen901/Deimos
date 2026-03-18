use bit_field::BitField;
use glam::{I16Vec3, U16Vec3, Vec3, vec3};

use crate::asset::vertex_buffer::VertexBuffer;

pub fn quant_snorm(v: f32, bits: u32) -> i32 {
    let denom: i32 = 1 << (bits - 1);
    let max_i = denom - 1;
    let min_i = -denom;

    let v = v.clamp(-1.0, 0.999999);
    let scaled = (v * (denom as f32)).round() as i32;
    scaled.clamp(min_i, max_i)
}

pub fn pack_pos_words_u32x2(sn: Vec3) -> (u32, u32) {
    let xi = quant_snorm(sn.x, 21);
    let yi = quant_snorm(sn.y, 21);
    let zi = quant_snorm(sn.z, 22);

    let mut word0: u32 = 0;
    let mut word1: u32 = 0;

    // X
    word0.set_bits(0..21, (xi as u32) & 0x1FFFFF);

    // Y (split between words 0 and 1)
    word0.set_bits(21..32, yi as u32 & 0x7FF);
    word1.set_bits(0..10, (yi as u32 >> 11) & 0x3FF);

    // Z
    word1.set_bits(10..32, (zi as u32) & 0x3FFFFF);

    (word0, word1)
}

pub fn repack_pos_words_from_vb(
    data: &[u8],
    stride: usize,
    original_scale_3d: Vec3,
    new_scale_1d: f32,
) -> anyhow::Result<Vec<u32>> {
    let mut words = Vec::with_capacity(data.len() / stride);
    if stride < 6 {
        anyhow::bail!("Stride must be at least 6 bytes")
    }
    for vertex in data.chunks_exact(stride) {
        let v: I16Vec3 = bytemuck::cast_slice(&vertex[0..6])[0];
        let pos = vec3(
            v.x as f32 / 32767.0,
            v.y as f32 / 32767.0,
            v.z as f32 / 32767.0,
        );

        let pos_rescaled = (pos * original_scale_3d) / new_scale_1d;

        let (word0, word1) = pack_pos_words_u32x2(pos_rescaled);
        words.push(word0);
        words.push(word1);
    }
    Ok(words)
}
