use std::fmt::{Debug, Formatter};

use tiger_parse::{PackageManagerExt, TigerReadable};
use tiger_pkg::{package_manager, TagHash};

#[derive(Clone)]
pub struct TagRef<T: TigerReadable>(pub T, TagHash);

impl<T: TigerReadable> TigerReadable for TagRef<T> {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let tag = TagHash::read_ds_endian(reader, endian)?;
        if tag.is_none() {
            return Err(tiger_parse::Error::TagReadFailed(
                "Attempted to read Tag with an unset TagHash (0xFFFFFFFF). Perhaps you meant to use an OptionalTag<T>?".to_string(),
            ));
        }
        Ok(Self(package_manager().read_tag_struct(tag)?, tag))
    }

    const SIZE: usize = TagHash::SIZE;
}

impl<T: TigerReadable> TagRef<T> {
    pub const fn new(value: T, taghash: TagHash) -> Self {
        Self(value, taghash)
    }

    pub const fn taghash(&self) -> TagHash {
        self.1
    }
}

impl<T: TigerReadable> std::ops::Deref for TagRef<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TigerReadable + Debug> Debug for TagRef<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("Tag({}, ", self.1))?;
        self.0.fmt(f)?;
        f.write_str(")")
    }
}

#[derive(Clone)]
pub struct OptionalTagRef<T: TigerReadable>(pub Option<T>, TagHash);

impl<T: TigerReadable> TigerReadable for OptionalTagRef<T> {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let tag = TagHash::read_ds_endian(reader, endian)?;
        let data = if tag.is_some() {
            Some(package_manager().read_tag_struct::<T>(tag)?)
        } else {
            None
        };

        Ok(Self(data, tag))
    }

    const SIZE: usize = TagHash::SIZE;
}

impl<T: TigerReadable> std::ops::Deref for OptionalTagRef<T> {
    type Target = Option<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TigerReadable> OptionalTagRef<T> {
    pub const fn taghash(&self) -> TagHash {
        self.1
    }
}

impl<T: TigerReadable + Debug> Debug for OptionalTagRef<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("OptionalTag({}, ", self.1))?;
        self.0.fmt(f)?;
        f.write_str(")")
    }
}

pub use crate::wide_hash::WideHash;

#[derive(Clone)]
pub struct WideTag<T: TigerReadable>(pub T, pub TagHash);

impl<T: TigerReadable> TigerReadable for WideTag<T> {
    fn read_ds_endian<R: std::io::prelude::Read + std::io::prelude::Seek>(
        reader: &mut R,
        endian: tiger_parse::Endian,
    ) -> tiger_parse::Result<Self> {
        let tag = WideHash::read_ds_endian(reader, endian)?;
        match tag {
            WideHash::Hash32(h) => Ok(Self(
                package_manager()
                    .read_tag_struct(h)
                    .map_err(|e| tiger_parse::Error::TagReadFailed(e.to_string()))?,
                h,
            )),
            WideHash::Hash64(h) => Ok(Self(
                package_manager()
                    .read_tag64_struct(h)
                    .map_err(|e| tiger_parse::Error::TagReadFailed(e.to_string()))?,
                tag.hash32(),
            )),
        }
    }

    const SIZE: usize = WideHash::SIZE;
}

impl<T: TigerReadable> std::ops::Deref for WideTag<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: TigerReadable + Debug> Debug for WideTag<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
