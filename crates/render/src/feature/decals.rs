use deimos_data::tfx::features::{decals::SDecalCollection, dynamic::RenderStageSubscription};
use tiger_parse::PackageManagerExt;
use tiger_pkg::package_manager;
use tiger_pkg::TagHash;

use crate::{
    asset::{vertex_buffer::VertexBuffer, Handle},
    gpu::command_list::CommandList,
    tfx::technique::Technique,
    Renderer,
};

use super::FeatureRenderer;

pub struct DecalCollectionRenderer {
    sets: Vec<DecalSet>,
    vb0: Handle<VertexBuffer>,
    vb1: Handle<VertexBuffer>,
}

pub struct DecalSet {
    pub technique: Handle<Technique>,
    pub start: u16,
    pub count: u16,
}

impl DecalCollectionRenderer {
    #[profiling::function]
    pub fn load(hash: TagHash) -> anyhow::Result<Box<Self>> {
        let collection = package_manager().read_tag_struct::<SDecalCollection>(hash)?;

        let vb0 = Renderer::instance().asset_manager.load(collection.vb0);
        let vb1 = Renderer::instance().asset_manager.load(collection.vb1);

        let sets = collection
            .unk8
            .into_iter()
            .map(|set| DecalSet {
                technique: Renderer::instance().asset_manager.load(set.technique),
                start: set.start,
                count: set.count,
            })
            .collect();

        Ok(Box::new(Self { sets, vb0, vb1 }))
    }
}

impl FeatureRenderer for DecalCollectionRenderer {
    fn extract_and_prepare(
        &mut self,
        _renderer: &Renderer,
        _data: &mut dyn super::FeatureRendererData,
        _extracted_data: &dyn std::any::Any,
    ) {
    }

    fn submit(&self, cmd: &mut CommandList, _stage: deimos_data::tfx::RenderStage) {
        let Some((vb0, vb1)) = self.vb0.get().zip(self.vb1.get()) else {
            return;
        };

        Renderer::instance().globals.scopes.decal.bind(cmd).unwrap();

        cmd.input_assembler_set_vertex_buffers(
            0,
            &[Some(vb1.buffer.clone()), Some(vb0.buffer.clone())],
            Some(&[vb1.stride, vb0.stride]),
            Some(&[0, 0]),
        )
        .unwrap();
        cmd.set_input_layout(23);
        cmd.set_input_topology(deimos_data::tfx::PrimitiveType::Triangles);
        for set in &self.sets {
            let Some(t) = set.technique.get() else {
                continue;
            };
            t.bind(cmd).unwrap();
            cmd.draw_instanced(36 as u32, set.count as u32, 0, set.start as u32);
        }
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        RenderStageSubscription::DECALS
    }
}
