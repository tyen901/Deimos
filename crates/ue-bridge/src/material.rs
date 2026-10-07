//! Source material programs. No GPU translation or inferred texture semantics.
use crate::entity::{array, source_identity, wide, word};
use anyhow::{Context, ensure};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tiger_pkg::{PackageManager, TagHash};
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn optional(tag: u32) -> Option<TagHash> {
    if tag == 0 || tag == u32::MAX {
        None
    } else {
        Some(TagHash(tag))
    }
}
fn entry(manager: &PackageManager, tag: TagHash) -> anyhow::Result<Value> {
    let e = manager
        .get_entry(tag)
        .with_context(|| format!("missing material dependency {tag}"))?;
    Ok(
        json!({"tag":tag.to_string(),"reference":format!("{:08X}",e.reference),"size":e.file_size,"file_type":e.file_type,"file_subtype":e.file_subtype}),
    )
}
fn float4(data: &[u8], offset: usize) -> anyhow::Result<[f32; 4]> {
    let mut result = [0.; 4];
    for (i, v) in result.iter_mut().enumerate() {
        *v = f32::from_bits(word(data, offset + i * 4)?);
        ensure!(v.is_finite(), "nonfinite technique constant");
    }
    Ok(result)
}
fn shader(manager: &PackageManager, tag: TagHash) -> anyhow::Result<Value> {
    let header = manager.read_tag(tag)?;
    let payload = TagHash(
        manager
            .get_entry(tag)
            .context("missing shader header")?
            .reference,
    );
    let bytes = manager
        .read_tag(payload)
        .with_context(|| format!("shader {tag} payload {payload}"))?;
    ensure!(
        bytes.get(0..4) == Some(b"DXBC"),
        "shader {tag} unsupported container"
    );
    ensure!(
        word(&bytes, 24)? as usize == bytes.len(),
        "shader {tag} container size mismatch"
    );
    let count = word(&bytes, 28)? as usize;
    ensure!(
        count <= bytes.len().saturating_sub(32) / 4,
        "shader chunk table outside container"
    );
    let mut chunks = Vec::new();
    for i in 0..count {
        let o = word(&bytes, 32 + i * 4)? as usize;
        let len = word(&bytes, o.checked_add(4).context("shader chunk overflow")?)? as usize;
        let end = o
            .checked_add(8)
            .and_then(|v| v.checked_add(len))
            .context("shader chunk overflow")?;
        ensure!(
            o >= 32 + count * 4 && end <= bytes.len(),
            "shader chunk outside container"
        );
        let fourcc = std::str::from_utf8(&bytes[o..o + 4])?;
        chunks.push(json!({"fourcc":fourcc,"offset":o,"size":len}));
    }
    Ok(
        json!({"header":entry(manager,tag)?,"header_sha256":digest(&header),"payload":entry(manager,payload)?,"payload_sha256":digest(&bytes),"container":"DXBC","chunks":chunks}),
    )
}
fn texture(manager: &PackageManager, tag: TagHash) -> anyhow::Result<Value> {
    let e = manager.get_entry(tag).context("missing texture header")?;
    ensure!(
        e.file_type == 32,
        "texture {tag} has unexpected entry type {}",
        e.file_type
    );
    let b = manager.read_tag(tag)?;
    ensure!(b.len() == 0x40, "texture {tag} header size mismatch");
    let half = |o| -> anyhow::Result<u16> {
        Ok(u16::from_le_bytes(
            b.get(o..o + 2)
                .context("texture header truncated")?
                .try_into()?,
        ))
    };
    ensure!(
        half(0x20)? == 0xCAFE,
        "texture {tag} header marker mismatch"
    );
    let large = optional(word(&b, 0x3C)?);
    let tail = TagHash(e.reference);
    Ok(
        json!({"header":entry(manager,tag)?,"header_sha256":digest(&b),"data_size":word(&b,0)?,"dxgi_format":word(&b,4)?,"width":half(0x22)?,"height":half(0x24)?,"depth":half(0x26)?,"array_size":half(0x28)?,"tile_count":half(0x2A)?,"source_mip_field":b[0x2D],"large_buffer":large.map(|t|entry(manager,t)).transpose()?,"header_payload":entry(manager,tail)?}),
    )
}
pub fn read_technique(manager: &PackageManager, tag: TagHash) -> anyhow::Result<Vec<u8>> {
    ensure!(
        manager
            .get_entry(tag)
            .context("missing technique")?
            .reference
            == 0x808031D8,
        "{tag} is not a source technique"
    );
    let b = manager.read_tag(tag)?;
    ensure!(b.len() >= 0x388, "truncated technique {tag}");
    ensure!(
        wide(&b, 0)? as usize == b.len(),
        "technique {tag} declared size mismatch"
    );
    let bind = word(&b, 8)?;
    ensure!(
        (1..=6).contains(&bind),
        "unsupported source bind mode {bind}"
    );
    let mut stages = Vec::new();
    for (i, name) in ["vertex", "hull", "domain", "geometry", "pixel", "compute"]
        .iter()
        .enumerate()
    {
        let base = 0x58 + i * 0x88;
        let Some(shader_tag) = optional(word(&b, base)?) else {
            continue;
        };
        let core = base + 8;
        let mut textures = Vec::new();
        for p in array(&b, core, 24, 0x808086C6)? {
            let texture_tag = source_identity(manager, &b, p + 8)?;
            textures.push(json!({"slot":word(&b,p)?,"source_identity_bytes":&b[p+8..p+24],"texture":texture(manager,texture_tag)?}));
        }
        let bytecode = array(&b, core + 0x18, 1, 0x80800009)?
            .into_iter()
            .map(|p| b[p])
            .collect::<Vec<_>>();
        let constants = array(&b, core + 0x28, 16, 0x80800090)?
            .into_iter()
            .map(|p| float4(&b, p))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let auxiliary = array(&b, core + 0x48, 16, 0x80800090)?
            .into_iter()
            .map(|p| float4(&b, p))
            .collect::<anyhow::Result<Vec<_>>>()?;
        let mut samplers = Vec::new();
        for p in array(&b, core + 0x38, 16, 0x8080013F)? {
            let sampler_tag = TagHash(word(&b, p)?);
            let data = manager.read_tag(sampler_tag)?;
            samplers.push(json!({"header":entry(manager,sampler_tag)?,"source_bytes":data,"assignment_words":[word(&b,p+4)?,word(&b,p+8)?,word(&b,p+12)?]}));
        }
        let cb = optional(word(&b, core + 0x64)?);
        let constant_buffer = if let Some(t) = cb {
            let data = manager.read_tag(t)?;
            Some(json!({"entry":entry(manager,t)?,"source_bytes":data}))
        } else {
            None
        };
        stages.push(json!({"stage":name,"shader":shader(manager,shader_tag)?,"stage_unknown":word(&b,base+4)?,"textures":textures,"samplers":samplers,"tfx":{"bytecode":bytecode,"constants":constants,"auxiliary_constants":auxiliary,"unknown10":format!("{:016X}",wide(&b,core+0x10)? as u64),"unknown58":[word(&b,core+0x58)?,word(&b,core+0x5C)?],"unknown68":(0..6).map(|j|word(&b,core+0x68+j*4)).collect::<anyhow::Result<Vec<_>>>()?},"constant_buffer_slot":word(&b,core+0x60)? as i32,"constant_buffer":constant_buffer}));
    }
    serde_json::to_vec(&json!({"technique":tag.to_string(),"source":entry(manager,tag)?,"source_sha256":digest(&b),"bind_mode":bind,"used_scopes":format!("{:016X}",wide(&b,0x20)? as u64),"compatible_scopes":format!("{:016X}",wide(&b,0x28)? as u64),"fixed_function_state":word(&b,0x30)?,"header_unknown_words":(0..5).map(|i|word(&b,0xC+i*4)).collect::<anyhow::Result<Vec<_>>>()?,"state_unknown_words":(0..9).map(|i|word(&b,0x34+i*4)).collect::<anyhow::Result<Vec<_>>>()?,"stages":stages})).map_err(Into::into)
}
