use d3d12::GpuVirtualAddress;
use deimos_data::tfx::{FeatureRendererSubscription, geometry::AxisAlignedBBox};
use deimos_ecs::{
    object::{ObjectChannels, PermutationConfig},
    transform::Transform,
};
use glam::Vec3;

use crate::{
    ecs::render_objects::{DynamicRenderObject, StaticRenderObject},
    features::rigid_model::{DynamicModel, DynamicObjectData},
    renderer::{Renderer, packet::ViewPacket, scene::SceneRenderer},
    visibility::ViewVisibility,
};

pub mod render_objects;
pub fn s_update_object_channels(world: &hecs::World) {
    for (_entity, (transform, object_channels)) in world
        .query::<(Option<&Transform>, &mut ObjectChannels)>()
        .iter()
    {
        object_channels.reset_usage_counters();
        object_channels.set_by_name(
            "interpolated_world_position",
            transform.map_or(Vec3::ZERO, |t| t.translation).extend(1.0),
        );
    }
}

pub fn s_extract_frame_packet(
    world: &hecs::World,
    scene: &mut SceneRenderer,
    visibility: &ViewVisibility,
    features: FeatureRendererSubscription,
) {
    let SceneRenderer {
        frame_packet,
        main_view,
        parent: renderer,
        ..
    } = scene;

    let mut render_objects = renderer.objects.write();

    let views = [&main_view];
    frame_packet.insert_view(0, main_view.culling_frustum.clone());

    for (_entity, (static_render_object, bounds)) in world
        .query::<(&StaticRenderObject, Option<&AxisAlignedBBox>)>()
        .iter()
    {
        if !features.is_subscribed(render_objects[static_render_object.handle()].feature_type) {
            continue;
        }

        let bounds = bounds.map_or(AxisAlignedBBox::EVERYTHING, |b| *b);
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

    for (_entity, (transform, render_object, permutations, bounds, object_channels)) in world
        .query::<(
            Option<&Transform>,
            &DynamicRenderObject,
            Option<&PermutationConfig>,
            Option<&AxisAlignedBBox>,
            Option<&ObjectChannels>,
        )>()
        .iter()
    {
        if !features.is_subscribed(render_objects[render_object.handle()].feature_type) {
            continue;
        }

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

        // IMMEDIATE_SHAPES.push(ImmediateShape {
        //     color: if visibility.is_visible(&bounds) {
        //         [0, 255, 0, 255]
        //     } else {
        //         [255, 0, 0, 255]
        //     },
        //     primitive: ImmediatePrimitive::BoundingBox(bounds),
        // });

        if let Some(object_channels) = object_channels
            && let Some(rigid_model) = render_objects
                .get_mut(render_object.handle())
                .and_then(|r| r.get_mut::<DynamicModel>())
        {
            for channel in &object_channels.0 {
                rigid_model.channels.insert(channel.name, channel.clone());
            }
        }

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
            if visibility.is_visible(&bounds) {
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

pub fn node_visibility_test(scene: &mut SceneRenderer, visibility: &ViewVisibility) {
    let SceneRenderer {
        parent: renderer,
        frame_packet,
        ..
    } = scene;

    let mut render_objects = renderer.objects.write();
    for ViewPacket { view_nodes, .. } in frame_packet.views.iter_mut() {
        for (_view_node, per_frame_node) in view_nodes
            .iter()
            .enumerate()
            .map(|(view_node, v)| (view_node, &frame_packet.per_frame_nodes[v.frame_node]))
        {
            let render_object = &mut render_objects[per_frame_node.object];
            render_object.renderer.visibility_test(visibility);
        }
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
        for (view_node_index, view_node, frame_node, render_object) in view_nodes
            .iter()
            .enumerate()
            .map(|(view_node, v)| (view_node, v, &frame_packet.per_frame_nodes[v.frame_node]))
            .map(|(view_node, v, o)| (view_node, v, o, &render_objects[o.object]))
        {
            render_object.renderer.populate_submit_node_blocks(
                renderer,
                (view_node_index, view_node),
                frame_node,
                visibility,
                submit_node_blocks,
            );
        }

        {
            profiling::scope!("sort stages");
            for stage in submit_node_blocks.blocks_mut() {
                stage.sort_by_key(|a| a.key);
            }
        }
    }
}

pub fn s_are_all_objects_loaded(world: &hecs::World, renderer: &Renderer) -> bool {
    for (_entity, static_render_object) in world.query::<&StaticRenderObject>().iter() {
        if !renderer.is_object_loaded(static_render_object.handle()) {
            return false;
        }
    }

    for (_entity, render_object) in world.query::<&DynamicRenderObject>().iter() {
        if !renderer.is_object_loaded(render_object.handle()) {
            return false;
        }
    }

    true
}
