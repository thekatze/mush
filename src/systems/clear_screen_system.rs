use bevy_ecs::system::Res;

use crate::resources::RenderContext;

pub fn clear_screen_system(context: Res<RenderContext>) {
    let (mut encoder, view, tx) = context.get_context("Clear Screen");

    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: view.as_ref(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.1,
                    g: 0.2,
                    b: 0.6,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    });

    tx.send(encoder).expect("channel closed");
}
