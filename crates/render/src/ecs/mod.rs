use deimos_data::tfx::geometry::AxisAlignedBBox;
use hecs::World;

use crate::{
    ecs::render_objects::StaticRenderObject,
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
            if visibility.is_visible(&bounds) {
                frame_packet.push_view_node::<()>(view_id, frame_node, None);
            }
        }
    }

    // for (_entity, (transform, render_object, permutations)) in world
    //     .query::<(
    //         Option<&Transform>,
    //         &DynamicRenderObject,
    //         Option<&PermutationConfig>,
    //     )>()
    //     .iter()
    // {
    //     let transform = transform.copied().unwrap_or_default();
    //     let permutation = if let Some(permutation) = permutations {
    //         permutation
    //             .calculate_permutation_index()
    //             .unwrap_or(render_object.permutation)
    //     } else {
    //         render_object.permutation
    //     };

    //     render_objects[render_object.handle]
    //         .renderer
    //         .extract(renderer, &(transform.local_to_world(), permutation));
    //     // frame_packet.push_dynamic_render_object(
    //     //     render_object.handle,
    //     //     transform.local_to_world().into(),
    //     //     permutation,
    //     // );
    // }
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
