use std::{sync::Arc, time::Instant};

use deimos_render::{camera::Camera, gpu::command_list::CommandList, Renderer};

// pub struct Scene {
//     renderer: Arc<Renderer>,
//     camera: Camera,
//     start_time: Instant,

//     surface: d3d11::Texture2D,
//     surface_rtv: d3d11::RenderTargetView,
// }

// impl Scene {
//     pub fn render(&mut self, delta_time: f32) {
//         let resolution = self.renderer.surfaces.framebuffer_resolution();
//         self.camera.aspect_ratio = resolution.0 as f32 / resolution.1 as f32;
//         let proj = self.camera.projection_matrix(self.camera.aspect_ratio);
//         // proj.z_axis.z = 2.6226E-06;

//         let view = self.camera.view_matrix();
//         self.camera.update();
//         // Renderer::instance()
//         //     .immediate
//         //     .lock()
//         //     .frustum(&frustum, 0x00ffff);

//         self.renderer.frame_packet.write().reset();

//         // {
//         //     let mut fp = self.renderer.frame_packet.write();
//         //     for t in &self.static_render_objects {
//         //         fp.push_static_render_object(*t);
//         //     }
//         // }

//         let gpu = &self.renderer.gpu;
//         let mut cmd = CommandList::from_device_context(gpu, gpu.context().clone());
//         {
//             profiling::scope!("prepare");

//             cmd.clear_render_target_view(&gpu.acquire_rtv(), &[0.0, 0.0, 0.0, 1.0]);

//             {
//                 profiling::scope!("visibility");
//                 self.renderer
//                     .frame_packet
//                     .write()
//                     .frame_nodes
//                     .retain(|node| {
//                         if let Some(render_object) = self
//                             .renderer
//                             .objects
//                             .write()
//                             .get_mut(node.render_object_handle.into())
//                         {
//                             render_object.visibility_test(&self.camera)
//                         } else {
//                             true
//                         }
//                     });
//             }

//             for node in self.renderer.frame_packet.read().frame_nodes.iter() {
//                 if let Some(render_object) = self
//                     .renderer
//                     .objects
//                     .write()
//                     .get_mut(node.render_object_handle.into())
//                 {
//                     render_object.extract_and_prepare(&self.renderer, &*node.data);
//                 } else if node.render_object_handle.is_valid() {
//                     error!("Render object not found: {:?}", node.render_object_handle);
//                 }
//             }

//             // Sort nodes by distance
//             self.renderer
//                 .frame_packet
//                 .write()
//                 .frame_nodes
//                 .sort_by(|n1, n2| {
//                     n2.distance
//                         .partial_cmp(&n1.distance)
//                         .unwrap_or(std::cmp::Ordering::Equal)
//                 });

//             cmd.rasterizer_set_viewports(&[d3d11::Viewport::builder()
//                 .width(self.renderer.surfaces.framebuffer_resolution().0 as f32)
//                 .height(self.renderer.surfaces.framebuffer_resolution().1 as f32)
//                 .build()]);

//             cmd.output_merger_set_blend_state(None, None, 0xFFFFFFFF);
//         }

//         self.renderer.submit_world(
//             &mut cmd,
//             view,
//             proj,
//             self.start_time.elapsed().as_secs_f32(),
//             delta_time,
//         );
//         // let cmd = self.draw_world(delta_time);
//         // self.renderer.gpu.submit_command_list(cmd);

//         // if self.show_debug_text
//         // {
//         //     let gpu = &self.renderer.gpu;
//         //     let context = gpu.context();

//         //     context.rasterizer_set_viewports(&[d3d11::Viewport::builder()
//         //         .width(gpu.swapchain_resolution().0 as f32)
//         //         .height(gpu.swapchain_resolution().1 as f32)
//         //         .build()]);
//         //     context.output_merger_set_render_targets(&[Some(gpu.acquire_rtv())], None);
//         //     context.output_merger_set_depth_stencil_state(None, 0);
//         //     context.rasterizer_set_state(None);
//         //     self.renderer.debug_text.lock().draw(&self.renderer.gpu);
//         // }
//     }
// }
