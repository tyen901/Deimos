use deimos_data::tfx::{TfxFeatureRenderer, features::dynamic::RenderStageSubscription};

use crate::features::FeatureRenderer;

slotmap::new_key_type! {
    pub struct RenderObjectHandle;
}

pub struct RenderObject {
    pub renderer: Box<dyn FeatureRenderer>,
    pub feature_type: TfxFeatureRenderer,
    pub stages: RenderStageSubscription,
}

impl RenderObject {
    pub fn new(kind: TfxFeatureRenderer, renderer: Box<dyn FeatureRenderer>) -> Self {
        Self {
            stages: renderer.subscribed_stages(),
            renderer,
            feature_type: kind,
        }
    }

    pub fn dyn_clone(&self) -> Option<Self> {
        Some(Self {
            stages: self.stages,
            renderer: self.renderer.dyn_clone()?,
            feature_type: self.feature_type,
        })
    }
}

unsafe impl Send for RenderObject {}
unsafe impl Sync for RenderObject {}
