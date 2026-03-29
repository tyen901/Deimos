use bit_field::BitField;
use glam::{I16Vec3, Vec3, vec3};

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
    if stride < 6 {
        anyhow::bail!("Stride must be at least 6 bytes")
    }

    let mut words = Vec::with_capacity(data.len() / stride);
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

pub fn repack_normal_tangent_words_from_vb(data: &[u8], stride: usize) -> anyhow::Result<Vec<u32>> {
    if stride < 24 {
        anyhow::bail!("Stride must be at least 24 bytes");
    }

    let mut words = Vec::with_capacity((data.len() / stride) * 2);

    for vertex in data.chunks_exact(stride) {
        let n: I16Vec3 = bytemuck::cast_slice(&vertex[8..14])[0];
        let t: I16Vec3 = bytemuck::cast_slice(&vertex[16..22])[0];

        let nf = Vec3::new(
            n.x as f32 / 32767.0,
            n.y as f32 / 32767.0,
            n.z as f32 / 32767.0,
        );
        let tf = Vec3::new(
            t.x as f32 / 32767.0,
            t.y as f32 / 32767.0,
            t.z as f32 / 32767.0,
        );

        let (nu, nv) = oct_encode(nf);
        let (tu, tv) = oct_encode(tf);

        let nu_i = quant_snorm(nu, 16);
        let nv_i = quant_snorm(nv, 16);
        let tu_i = quant_snorm(tu, 16);
        let tv_i = quant_snorm(tv, 16);

        let c = ((nv_i as u32 & 0xFFFF) << 16) | (nu_i as u32 & 0xFFFF);
        let d = ((tv_i as u32 & 0xFFFF) << 16) | (tu_i as u32 & 0xFFFF);

        words.push(c);
        words.push(d);
    }

    Ok(words)
}

/// Octahedral encoding for a unit vector.
fn oct_encode(v: Vec3) -> (f32, f32) {
    // Project onto octahedron: v /= (|x|+|y|+|z|)
    let sum = v.x.abs() + v.y.abs() + v.z.abs();
    let mut x = v.x / sum;
    let mut y = v.y / sum;
    let z = v.z / sum;

    if z < 0.0 {
        // Remap the back face
        let old_x = x;
        x = (1.0 - y.abs()) * (if x >= 0.0 { 1.0 } else { -1.0 });
        y = (1.0 - old_x.abs()) * (if y >= 0.0 { 1.0 } else { -1.0 });
    }

    (x, y)
}
