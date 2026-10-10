use std::fmt::{Debug, Display, Formatter};
use tiger_parse::TigerReadable;
#[cfg(not(feature = "cpu"))]
use tiger_pkg::package_manager;
use tiger_pkg::{TagHash, TagHash64};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum WideHash {
    Hash32(TagHash),
    Hash64(TagHash64),
}

impl WideHash {
    /// Key that is safe to use for caching/lookup tables
    pub const fn key(&self) -> u64 {
        match self {
            Self::Hash32(v) => v.0 as u64,
            Self::Hash64(v) => v.0,
        }
    }

    /// Will lookup hash64 in package managers's h64 table in the case of a 64 bit hash
    /// Falls back to `TagHash::NONE` if not found
    #[cfg(not(feature = "cpu"))]
    pub fn hash32(&self) -> TagHash {
        self.hash32_checked().unwrap_or(TagHash::NONE)
    }

    /// Will lookup hash64 in package managers's h64 table in the case of a 64 bit hash
    /// Returns None if the hash is not found or null in case of a 32 bit hash
    #[cfg(not(feature = "cpu"))]
    pub fn hash32_checked(&self) -> Option<TagHash> {
        match self {
            Self::Hash32(v) => v.is_some().then_some(*v),
            Self::Hash64(v) => package_manager()
                .lookup
                .tag64_entries
                .get(&v.0)
                .map(|v| v.hash32),
        }
    }

    pub fn is_some(&self) -> bool {
        match self {
            Self::Hash32(h) => h.is_some(),
            // TODO(cohae): Double check this
            Self::Hash64(h) => h.0 != 0 && h.0 != u64::MAX,
        }
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
}

impl Debug for WideHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hash32(h) => f.write_fmt(format_args!("Hash32({h})")),
            Self::Hash64(h) => f.write_fmt(format_args!("Hash64({h})")),
        }
    }
}

impl Display for WideHash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hash32(h) => <TagHash as Display>::fmt(h, f),
            Self::Hash64(h) => <TagHash64 as Display>::fmt(h, f),
        }
    }
}

impl std::hash::Hash for WideHash {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.key());
    }
}

#[cfg(not(feature = "cpu"))]
impl From<WideHash> for TagHash {
    fn from(val: WideHash) -> Self {
        val.hash32()
    }
}

impl From<TagHash> for WideHash {
    fn from(val: TagHash) -> Self {
        Self::Hash32(val)
    }
}

impl TigerReadable for WideHash {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let hash32: TagHash = TigerReadable::read_ds_endian(reader, endian)?;
        let is_hash32: u32 = TigerReadable::read_ds_endian(reader, endian)?;
        let hash64: TagHash64 = TigerReadable::read_ds_endian(reader, endian)?;

        if is_hash32 != 0 {
            Ok(Self::Hash32(hash32))
        } else {
            Ok(Self::Hash64(hash64))
        }
    }

    const SIZE: usize = 16;
}
