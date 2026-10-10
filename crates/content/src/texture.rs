use crate::Installation;
use anyhow::Result;
use deimos_data::tfx::texture::{STextureHeader, load_texture_payload};
use tiger_pkg::TagHash;
pub struct Texture {
    pub header: STextureHeader,
    pub bytes: Vec<u8>,
}
impl Installation {
    pub fn texture(&self, tag: u32) -> Result<Texture> {
        let header: STextureHeader = self.read_type(tag)?;
        let bytes = load_texture_payload(&header, TagHash(self.reference(tag)?), true, |hash| {
            self.read(hash.0)
        })?;
        Ok(Texture { header, bytes })
    }
}
