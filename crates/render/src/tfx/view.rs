// pub enum View {
//     Shaded(ShadedView),
//     Shadow(ShadowView),
// }

use crate::visibility::frustum::Frustum;

pub struct ShadedView {
    pub culling_frustum: Frustum,
}

// pub struct ShadowView {}
