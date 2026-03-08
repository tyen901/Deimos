use d3d12::GpuVirtualAddress;
use deimos_data::tfx::geometry::AxisAlignedBBox;
use deimos_ecs::{permutations::PermutationConfig, transform::Transform};

use crate::{
    ecs::render_objects::{DynamicRenderObject, StaticRenderObject},
    features::rigid_model::DynamicObjectData,
    renderer::{packet::ViewPacket, scene::SceneRenderer},
    visibility::ViewVisibility,
};

pub mod render_objects;

pub fn s_extract_frame_packet(
    world: &hecs::World,
    scene: &mut SceneRenderer,
    visibility: &ViewVisibility,
) {
    let SceneRenderer {
        frame_packet,
        main_view,
        ..
    } = scene;

    let views = [&main_view];
    frame_packet.insert_view(0, main_view.culling_frustum.clone());

    for (_entity, (static_render_object, bounds)) in world
        .query::<(&StaticRenderObject, Option<&AxisAlignedBBox>)>()
        .iter()
    {
        let bounds = bounds.map_or(AxisAlignedBBox::EVERYTHING, |b| b.clone());
        let frame_node = frame_packet.push_frame_node::<()>(
            static_render_object.handle(),
            bounds.sphere(),
            None,
        );

        for (view_id, _v) in views.iter().enumerate() {
            if visibility.is_visible_quick(&bounds) {
                frame_packet.push_view_node::<()>(view_id, frame_node, None);
            }
        }
    }

    for (_entity, (transform, render_object, permutations, bounds)) in world
        .query::<(
            Option<&Transform>,
            &DynamicRenderObject,
            Option<&PermutationConfig>,
            Option<&AxisAlignedBBox>,
        )>()
        .iter()
    {
        let transform = transform.copied().unwrap_or_default();
        let permutation = if let Some(permutation) = permutations {
            permutation
                .calculate_permutation_index()
                .unwrap_or(render_object.permutation)
        } else {
            render_object.permutation
        };

        let bounds = bounds.map_or(AxisAlignedBBox::EVERYTHING, |b| {
            b.transformed(transform.local_to_world())
        });
        let frame_node = frame_packet.push_frame_node(
            render_object.handle(),
            bounds.sphere(),
            Some(DynamicObjectData {
                local_to_world: transform.local_to_world(),
                permutation,
                cbuffer_gpuva: GpuVirtualAddress::NULL,
            }),
        );

        for (view_id, _v) in views.iter().enumerate() {
            if visibility.is_visible_quick(&bounds) {
                frame_packet.push_view_node::<()>(view_id, frame_node, None);
            }
        }

        // render_objects[render_object.handle]
        //     .renderer
        //     .extract(renderer, &(transform.local_to_world(), permutation));
        // frame_packet.push_dynamic_render_object(
        //     render_object.handle,
        //     transform.local_to_world().into(),
        //     permutation,
        // );
    }
}

pub fn populate_submit_nodes(scene: &mut SceneRenderer, visibility: &ViewVisibility) {
    let SceneRenderer {
        parent: renderer,
        frame_packet,
        ..
    } = scene;

    let render_objects = renderer.objects.read();
    for ViewPacket {
        view_nodes,
        submit_node_blocks,
        ..
    } in frame_packet.views.iter_mut()
    {
        for (view_node, render_object) in view_nodes
            .iter()
            .enumerate()
            .map(|(view_node, v)| (view_node, &frame_packet.per_frame_nodes[v.frame_node]))
            .map(|(view_node, o)| (view_node, &render_objects[o.object]))
        {
            render_object.renderer.populate_submit_node_blocks(
                renderer,
                view_node,
                visibility,
                submit_node_blocks,
            );
        }
    }
}
