use std::ptr::null_mut;

use umbra3_sys::{Umbra_Tome, Umbra_TomeLoader_freeTome, Umbra_TomeLoader_loadFromBuffer};

pub struct Tome(pub(crate) *const Umbra_Tome);

// Tomes are immutable data, so they can be safely sent between threads.
unsafe impl Send for Tome {}
unsafe impl Sync for Tome {}

impl Tome {
    pub fn load_from_buffer(data: &[u8]) -> Self {
        // TODO(cohae): Error checking
        Self(unsafe { Umbra_TomeLoader_loadFromBuffer(data.as_ptr(), data.len(), null_mut()) })
    }
}

impl Drop for Tome {
    fn drop(&mut self) {
        unsafe {
            Umbra_TomeLoader_freeTome(self.0, null_mut());
        }
    }
}
