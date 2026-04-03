use anyhow::Context;
use deimos_data::tfx::{
    FixedFunctionState, PrimitiveType, RenderStage,
    features::{decals::SDecalCollection, dynamic::RenderStageSubscription},
    geometry::AxisAlignedBBox,
};

use crate::{
    asset::{Handle, vertex_buffer::VertexBuffer},
    features::{FeatureRenderer, static_instances::StaticSubmitKey},
    gpu::command_list::{CommandList, DepthMode},
    renderer::{
        Renderer,
        packet::{RenderPerFrameNode, RenderPerViewNode, SubmitNode},
    },
    tfx::technique::Technique,
};

pub struct DecalCollectionRenderer {
    sets: Vec<DecalSet>,
    vb0: Handle<VertexBuffer>,
    vb1: Handle<VertexBuffer>,
    bounds: AxisAlignedBBox,
    render_stage: RenderStage,
}

pub struct DecalSet {
    pub bounds: AxisAlignedBBox,
    pub technique: Handle<Technique>,
    pub start: u16,
    pub count: u16,
}

impl DecalCollectionRenderer {
    // #[profiling::function]
    pub fn load(renderer: &Renderer, collection: SDecalCollection) -> anyhow::Result<Box<Self>> {
        let vb0 = renderer.asset_manager.load(collection.vb0);
        let vb1 = renderer.asset_manager.load(collection.vb1);

        let sets = collection
            .decals
            .into_iter()
            .map(|set| {
                let r = (set.start as usize)..((set.start + set.count) as usize);
                Ok(DecalSet {
                    bounds: collection
                        .decal_bounds
                        .bounds
                        .get(r)
                        .context("decal set bounds out of bounds")?
                        .iter()
                        .map(|b| b.bb)
                        .sum(),
                    technique: renderer.asset_manager.load(set.technique),
                    start: set.start,
                    count: set.count,
                })
            })
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(Box::new(Self {
            sets,
            vb0,
            vb1,
            bounds: collection.bounds,
            render_stage: collection.render_stage,
        }))
    }
}

// #[profiling::all_functions]
impl FeatureRenderer for DecalCollectionRenderer {
    fn populate_submit_node_blocks(
        &self,
        _renderer: &crate::renderer::Renderer,
        (view_node, _): (usize, &RenderPerViewNode),
        frame_node: &RenderPerFrameNode,
        visibility: &crate::visibility::ViewVisibility,
        submit_node_blocks: &mut crate::renderer::packet::SubmitNodeContainer,
    ) {
        for (i, set) in self.sets.iter().enumerate() {
            if visibility.is_visible(&set.bounds) {
                submit_node_blocks.broadcast(
                    self.subscribed_stages(),
                    SubmitNode {
                        view_node,
                        key: StaticSubmitKey {
                            technique: u32::MAX, // set.technique.hash().0,
                            model_index: i as u16,
                            is_special_mesh: false,
                            mesh_index: 0,
                        }
                        .to_u64(),
                    },
                );
            }
        }
    }

    fn submit(
        &self,
        cmd: &mut CommandList,
        _stage: deimos_data::tfx::RenderStage,
        _frame_node: &crate::renderer::packet::RenderPerFrameNode,
        _view_node: &crate::renderer::packet::RenderPerViewNode,
        submit_key: u64,
    ) {
        let key = StaticSubmitKey::from_u64(submit_key);
        let Some((vb0, vb1)) = self.vb0.get().zip(self.vb1.get()) else {
            return;
        };

        cmd.disable_smart_technique_binding();
        cmd.set_ffstate_override(FixedFunctionState::new(None, None, Some(1), None));
        cmd.set_depth_mode(DepthMode::Forward);

        // cmd.input_assembler_set_vertex_buffers(
        //     0,
        //     &[Some(&vb1.buffer), Some(&vb0.buffer)],
        //     Some(&[vb1.stride, vb0.stride]),
        //     Some(&[0, 0]),
        // )
        // .unwrap();
        cmd.ia_set_vertex_buffers(0, &[vb1.view(), vb0.view()]);

        cmd.set_input_layout(17);
        cmd.set_input_topology(PrimitiveType::Triangles);
        let set = &self.sets[key.model_index as usize];
        let Some(t) = set.technique.get() else {
            return;
        };
        t.bind(cmd);
        cmd.draw_instanced(0..36, set.start as u32..(set.start + set.count) as u32);

        cmd.set_depth_mode(DepthMode::Reverse);
        cmd.reset_ffstate_override();
    }

    fn subscribed_stages(&self) -> RenderStageSubscription {
        // TODO(cohae): Dynamic decals rarely actually use the decals stage, they should actually use GenerateGbuffer instead
        if self.render_stage == RenderStage::GenerateGbuffer {
            RenderStage::Decals.into()
        } else {
            self.render_stage.into()
        }
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
