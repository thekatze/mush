use bevy_ecs::{
    event::EventWriter,
    schedule::IntoSystemConfigs as _,
    system::{Query, Res, Resource},
};
use image::GenericImageView;
use wgpu::{include_wgsl, util::RenderEncoder};

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
    texture_coordinates: [f32; 2],
}

#[rustfmt::skip]
const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.25, 0.25, 0.0], texture_coordinates: [0.0, 0.0] },
    Vertex { position: [-0.25, -0.25, 0.0], texture_coordinates: [0.0, 1.0] },
    Vertex { position: [0.25, 0.25, 0.0], texture_coordinates: [1.0, 0.0] },
    Vertex { position: [0.25, -0.25, 0.0], texture_coordinates: [1.0, 1.0] },
];

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

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
        let queue = &world.resource::<WgpuQueue>().0;

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Sprite Texture Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let shader = device.create_shader_module(include_wgsl!("sprite.wgsl"));
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Sprite Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Sprite Vertex Buffer"),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            size: std::mem::size_of_val(VERTICES) as u64,
            mapped_at_creation: false,
        });

        let bind_group = {
            let bytes = include_bytes!("../../happy-tree.png");
            let image = image::load_from_memory(bytes).expect("valid png");
            let rgba = image.to_rgba8();

            let dimensions = image.dimensions();

            let texture_size = wgpu::Extent3d {
                width: dimensions.0,
                height: dimensions.1,
                depth_or_array_layers: 1,
            };

            let texture = device.create_texture(&wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("../../happy-tree.png"),
                view_formats: &[],
            });

            queue.write_texture(
                wgpu::ImageCopyTextureBase {
                    texture: &texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &rgba,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * dimensions.0),
                    rows_per_image: Some(dimensions.1),
                },
                texture_size,
            );

            let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::Repeat,
                address_mode_v: wgpu::AddressMode::Repeat,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Nearest,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                label: Some("Sprite Texture Bind Group"),
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    },
                ],
            })
        };

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
            bind_group,
        });

        schedule.add_systems(draw_sprites_system.in_set(RenderStage::Render));
    }
}

#[derive(Resource)]
pub struct SpritePluginContext {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
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
        render_pass.set_bind_group(0, &sprite_plugin_context.bind_group, &[]);
        render_pass.draw(0..4, 0..1);
    }

    buffer_queue.send(CommandBufferFinishedEvent(encoder.finish()));
}
