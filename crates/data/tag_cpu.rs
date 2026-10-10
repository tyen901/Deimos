//! Deferred references for session-owned CPU loading. Wire schemas are shared
//! with the renderer; reading a header never opens unrelated dependencies.
pub use crate::wide_hash::WideHash;
use std::{
    fmt::{Debug, Formatter},
    marker::PhantomData,
};
use tiger_parse::TigerReadable;
use tiger_pkg::TagHash;

macro_rules! reference {
    ($name:ident, $hash:ty) => {
        pub struct $name<T: TigerReadable>(pub $hash, PhantomData<T>);
        impl<T: TigerReadable> $name<T> {
            pub const fn taghash(&self) -> $hash {
                self.0
            }
        }
        impl<T: TigerReadable> Clone for $name<T> {
            fn clone(&self) -> Self {
                Self(self.0, PhantomData)
            }
        }
        impl<T: TigerReadable> Debug for $name<T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                Debug::fmt(&self.0, f)
            }
        }
        impl<T: TigerReadable> TigerReadable for $name<T> {
            fn read_ds_endian<R: std::io::Read + std::io::Seek>(
                r: &mut R,
                e: tiger_parse::Endian,
            ) -> tiger_parse::Result<Self> {
                Ok(Self(<$hash>::read_ds_endian(r, e)?, PhantomData))
            }
            const SIZE: usize = <$hash>::SIZE;
        }
    };
}
reference!(TagRef, TagHash);
reference!(OptionalTagRef, TagHash);
reference!(WideTag, WideHash);
