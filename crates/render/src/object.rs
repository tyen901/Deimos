use std::any::Any;

use deimos_data::tfx::{
    features::dynamic::RenderStageSubscription, RenderStage, TfxFeatureRenderer,
};

use crate::{
    feature::{FeatureRenderer, FeatureRendererData},
    gpu::command_list::CommandList,
    util::arena,
    visibility::frustum::Frustum,
    Renderer,
};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RenderObjectHandle(pub(crate) arena::Index);

impl RenderObjectHandle {
    pub const INVALID: Self = Self(arena::Index::INVALID);

    pub fn is_valid(&self) -> bool {
        self != &Self::INVALID
    }
}

impl From<RenderObjectHandle> for arena::Index {
    fn from(handle: RenderObjectHandle) -> Self {
        handle.0
    }
}

pub struct RenderObject {
    pub data: Box<dyn FeatureRendererData>,
    renderer: Box<dyn FeatureRenderer>,
    pub feature_type: TfxFeatureRenderer,
    pub stages: RenderStageSubscription,
}

impl RenderObject {
    pub fn new(
        kind: TfxFeatureRenderer,
        renderer: Box<dyn FeatureRenderer>,
        // TODO(cohae): Can we make a nice way to check that the data is the correct type so we can make it easier to debug when the wrong data is passed?
        data: Box<dyn FeatureRendererData>,
    ) -> Self {
        Self {
            data,
            stages: renderer.subscribed_stages(),
            renderer,
            feature_type: kind,
        }
    }

    pub fn dyn_clone(&self) -> Option<Self> {
        Some(Self {
            data: self.data.dyn_clone()?,
            stages: self.stages,
            renderer: self.renderer.dyn_clone()?,
            feature_type: self.feature_type,
        })
    }
}

impl RenderObject {
    pub fn visibility_test(&mut self, frustum: &Frustum) -> bool {
        self.renderer.visibility_test(frustum)
    }

    pub fn extract_and_prepare(&mut self, renderer: &Renderer, data: &dyn Any) {
        self.renderer
            .extract_and_prepare(renderer, &mut *self.data, data);
    }

    pub fn submit(&self, cmd: &mut CommandList, stage: RenderStage) {
        self.renderer.submit(cmd, stage);
    }
}
