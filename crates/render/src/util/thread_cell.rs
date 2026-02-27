use std::{cell::UnsafeCell, ops::Deref};

// TODO(cohae): This is left over from the old implementation for extern management. It is NOT SAFE.
// Should be replaced with an RwLock and immutable copy sometime
pub struct ThreadMutCell<T> {
    inner: UnsafeCell<T>,
    /// The ID of the thread that created this cell. This thread is the only one that can mutate the inner value.
    thread: std::thread::ThreadId,
}

impl<T> ThreadMutCell<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner: UnsafeCell::new(inner),
            thread: std::thread::current().id(),
        }
    }

    pub fn get(&self) -> &T {
        unsafe { &*self.inner.get() }
    }

    #[allow(clippy::mut_from_ref)]
    pub fn get_mut(&self) -> &mut T {
        if std::thread::current().id() != self.thread {
            panic!(
                "Attempted to get mutable reference to ThreadMutCell from a different thread than \
                 the one that created it"
            );
        }
        unsafe { &mut *self.inner.get() }
    }
}

unsafe impl<T: Send> Send for ThreadMutCell<T> {}
unsafe impl<T: Sync> Sync for ThreadMutCell<T> {}

impl<T> Deref for ThreadMutCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}
