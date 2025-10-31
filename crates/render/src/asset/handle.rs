use std::{
    any::Any,
    sync::{
        atomic::{AtomicBool, AtomicUsize},
        Arc,
    },
};

use arc_atomic::AtomicArc;
use tiger_pkg::TagHash;

use super::Asset;

struct AssetHolder {
    data: AtomicArc<Arc<dyn Any + Send + Sync>>,
    loaded: AtomicBool,
    ref_count: AtomicUsize,
}

impl AssetHolder {
    fn new() -> Self {
        Self {
            data: AtomicArc::new(Arc::new(Arc::new(()))),
            loaded: AtomicBool::new(false),
            ref_count: AtomicUsize::new(1),
        }
    }
}

pub struct UntypedHandle {
    inner: Arc<AssetHolder>,
    pub tag: TagHash,
}

impl Default for UntypedHandle {
    fn default() -> Self {
        Self::new(TagHash::NONE)
    }
}

impl UntypedHandle {
    pub fn new(tag: TagHash) -> Self {
        Self {
            inner: Arc::new(AssetHolder::new()),
            tag,
        }
    }

    /// # Safety
    /// The caller must ensure that the asset is of the correct type.
    pub unsafe fn clone_as_typed_unchecked<T: Asset>(&self) -> Handle<T> {
        Handle {
            asset: self.clone(),
            _marker: std::marker::PhantomData,
        }
    }

    #[deprecated(note = "Use `update_boxed` instead")]
    pub fn update<T: Asset + Send + Sync + 'static>(&self, asset: T) {
        self.update_boxed(Box::new(asset));
    }

    pub fn update_boxed<T: Asset + Send + Sync + 'static>(&self, asset: Box<T>) {
        self.inner.data.store(Arc::new(Arc::<T>::from(asset)));
        self.inner
            .loaded
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn ref_count(&self) -> usize {
        self.inner
            .ref_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }
}

impl Clone for UntypedHandle {
    fn clone(&self) -> Self {
        self.inner
            .ref_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Self {
            inner: self.inner.clone(),
            tag: self.tag,
        }
    }
}

impl Drop for UntypedHandle {
    fn drop(&mut self) {
        self.inner
            .ref_count
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }
}

pub struct Handle<T: Asset + 'static> {
    asset: UntypedHandle,
    _marker: std::marker::PhantomData<T>,
}

impl<T: Asset + Sync + Send + 'static> Handle<T> {
    pub fn is_loaded(&self) -> bool {
        self.asset
            .inner
            .loaded
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn is_null(&self) -> bool {
        self.asset.tag.is_none()
    }

    pub fn tag(&self) -> TagHash {
        self.asset.tag
    }

    pub fn get(&self) -> Option<Arc<T>> {
        if !self.is_loaded() {
            return None;
        }

        let data = Arc::clone(&*self.asset.inner.data.load());
        data.downcast().ok()
    }

    pub fn update(&self, asset: Box<T>) {
        self.asset.update_boxed(asset);
    }

    pub fn ref_count(&self) -> usize {
        self.asset.ref_count()
    }
}

impl<T: Asset + 'static> Clone for Handle<T> {
    fn clone(&self) -> Self {
        Self {
            asset: self.asset.clone(),
            _marker: std::marker::PhantomData,
        }
    }
}
