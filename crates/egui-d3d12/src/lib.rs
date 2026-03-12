mod mesh;
mod painter;
// mod shader;
mod texture;

pub use painter::*;

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("Unrecoverable error: {0}")]
    General(String),

    #[error("D3D11 error {0}")]
    D3D11(#[from] d3d12::Error),
}
