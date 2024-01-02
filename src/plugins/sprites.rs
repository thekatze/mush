use bevy_ecs::{
    event::EventWriter,
    schedule::IntoSystemConfigs as _,
    system::{Query, Res, Resource},
};
use wgpu::include_wgsl;

use super::{
    rendering::{
        Camera, CommandBufferFinishedEvent, RenderStage, WgpuConfig, WgpuDevice, WgpuQueue,
    },
    Plugin,
};

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

#[rustfmt::skip]
const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.25, 0.25, 0.0], color: [1.0, 0.0, 0.0] },
    Vertex { position: [-0.25, -0.25, 0.0], color: [0.0, 1.0, 0.0] },
    Vertex { position: [0.25, 0.25, 0.0], color: [0.0, 0.0, 1.0] },
    Vertex { position: [0.25, -0.25, 0.0], color: [1.0, 0.0, 1.0] },
];

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    #[inline]
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

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

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: std::mem::size_of_val(VERTICES) as u64,
            mapped_at_creation: false,
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Sprite Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vertex_main",
                buffers: &[Vertex::desc()],
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

        world.insert_resource(SpritePluginContext {
            pipeline,
            vertex_buffer,
        });

        schedule.add_systems(draw_sprites_system.in_set(RenderStage::Render));
    }
}

#[derive(Resource)]
pub struct SpritePluginContext {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
}

pub fn draw_sprites_system(
    device: Res<WgpuDevice>,
    cameras: Query<&Camera>,
    queue: Res<WgpuQueue>,
    mut buffer_queue: EventWriter<CommandBufferFinishedEvent>,
    sprite_plugin_context: Res<SpritePluginContext>,
) {
    let mut encoder = device
        .0
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Sprites"),
        });

    queue.0.write_buffer(
        &sprite_plugin_context.vertex_buffer,
        0,
        bytemuck::cast_slice(VERTICES),
    );

    for camera in cameras.iter() {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: camera.view.as_ref().unwrap(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&sprite_plugin_context.pipeline);
        render_pass.set_vertex_buffer(0, sprite_plugin_context.vertex_buffer.slice(..));
        render_pass.draw(0..4, 0..1);
    }

    buffer_queue.send(CommandBufferFinishedEvent(encoder.finish()));
}
