use bevy_ecs::{
    event::EventWriter,
    schedule::IntoSystemConfigs as _,
    system::{Query, Res, Resource},
};
use wgpu::include_wgsl;

use super::{
    rendering::{Camera, CommandBufferFinishedEvent, RenderStage, WgpuConfig, WgpuDevice},
    Plugin,
};

pub struct SpritePlugin;

impl Plugin for SpritePlugin {
    fn build(
        self,
        world: &mut bevy_ecs::world::World,
        schedule: &mut bevy_ecs::schedule::Schedule,
    ) {
        let device = &world.resource::<WgpuDevice>().0;
        let config = &world.resource::<WgpuConfig>().0;

        let shader = device.create_shader_module(include_wgsl!("sprite.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fragment_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        world.insert_resource(SpritePluginContext { pipeline });

        schedule.add_systems(clear_screen_system.in_set(RenderStage::Render));
    }
}

#[derive(Resource)]
pub struct SpritePluginContext {
    pipeline: wgpu::RenderPipeline,
}

pub fn clear_screen_system(
    device: Res<WgpuDevice>,
    cameras: Query<&Camera>,
    mut buffer_queue: EventWriter<CommandBufferFinishedEvent>,
    sprite_plugin_context: Res<SpritePluginContext>,
) {
    let mut encoder = device
        .0
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clear Screen"),
        });

    for camera in cameras.iter() {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: camera.view.as_ref().unwrap(),
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

        render_pass.set_pipeline(&sprite_plugin_context.pipeline);
        render_pass.draw(0..3, 0..1);
    }

    buffer_queue.send(CommandBufferFinishedEvent(encoder.finish()));
}
