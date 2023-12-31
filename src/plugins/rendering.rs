use bevy_ecs::{
    component::Component,
    event::{Event, EventReader, Events},
    schedule::{IntoSystemConfigs as _, IntoSystemSetConfigs as _, Schedule, SystemSet},
    system::{Query, Res, ResMut, Resource},
    world::World,
};
use winit::window::Window;

use crate::application::ResizeEvent;

#[derive(SystemSet, Clone, Hash, Eq, PartialEq, Debug)]
pub enum RenderStage {
    Prepare,
    Render,
    Flush,
}

#[derive(Resource)]
pub struct WgpuAdapter(pub wgpu::Adapter);

#[derive(Resource)]
pub struct WgpuSurface(pub wgpu::Surface);

#[derive(Resource)]
pub struct WgpuDevice(pub wgpu::Device);

#[derive(Resource)]
pub struct WgpuQueue(pub wgpu::Queue);

#[derive(Resource)]
pub struct WgpuConfig(pub wgpu::SurfaceConfiguration);

#[derive(Resource)]
pub struct CameraBindGroupLayout(pub wgpu::BindGroupLayout);

pub async fn init_render_schedule(
    world: &mut World,
    window: &Window,
    schedule: &mut Schedule,
) -> Result<(), anyhow::Error> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let surface = unsafe { instance.create_surface(window) }?;

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or(anyhow::anyhow!("Failed to find an appropriate adapter"))?;

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                ..Default::default()
            },
            None,
        )
        .await?;

    let surface_capabilities = surface.get_capabilities(&adapter);

    let surface_format = surface_capabilities
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .ok_or(anyhow::anyhow!("No SRGB Surface"))?;

    let size = window.inner_size();

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: *surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
    };

    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    world.insert_resource(WgpuAdapter(adapter));
    world.insert_resource(WgpuSurface(surface));
    world.insert_resource(WgpuDevice(device));
    world.insert_resource(WgpuQueue(queue));
    world.insert_resource(WgpuConfig(config));
    world.insert_resource(CameraBindGroupLayout(camera_bind_group_layout));

    world.insert_resource(Events::<ResizeEvent>::default());
    world.insert_resource(Events::<CommandBufferFinishedEvent>::default());

    // define order
    schedule.configure_sets(
        (
            RenderStage::Prepare,
            RenderStage::Render,
            RenderStage::Flush,
        )
            .chain(),
    );

    schedule.add_systems((
        (reconfigure_device_on_resize_system, prepare_render_system)
            .chain()
            .in_set(RenderStage::Prepare),
        flush_render_system.in_set(RenderStage::Flush),
    ));

    Ok(())
}

fn prepare_render_system(
    device: Res<WgpuDevice>,
    surface: Res<WgpuSurface>,
    camera_bind_group_layout: Res<CameraBindGroupLayout>,
    mut cameras: Query<&mut Camera>,
    queue: Res<WgpuQueue>,
) {
    for mut camera in cameras.iter_mut() {
        let output = surface
            .0
            .get_current_texture()
            .expect("cant get surface to draw on");

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        if camera.uniform.is_none() {
            let uniform = device.0.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Camera Uniform"),
                size: std::mem::size_of::<glam::Mat4>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            camera.uniform = Some(uniform);
        }

        // set uniform
        camera.projection = camera.get_projection_matrix();

        let uniform = camera.uniform.as_ref().unwrap();
        queue
            .0
            .write_buffer(uniform, 0, bytemuck::bytes_of(&camera.projection));

        if camera.bind_group.is_none() {
            let bind_group = device.0.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Camera Bind Group"),
                layout: &camera_bind_group_layout.0,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                }],
            });

            camera.bind_group = Some(bind_group);
        }

        camera.output = Some(output);
        camera.view = Some(view);
    }
}

#[derive(Event)]
pub struct CommandBufferFinishedEvent(pub wgpu::CommandBuffer);

#[derive(Component, Debug)]
pub struct Camera {
    // public
    pub eye: glam::Vec3,
    pub direction: glam::Vec3,
    pub aspect: f32,
    pub fov: f32,
    pub clip_near: f32,
    pub clip_far: f32,

    // render internals
    pub output: Option<wgpu::SurfaceTexture>,
    pub view: Option<wgpu::TextureView>,
    pub projection: glam::Mat4,
    pub uniform: Option<wgpu::Buffer>,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl Camera {
    fn get_projection_matrix(&self) -> glam::Mat4 {
        let view = glam::Mat4::look_to_rh(self.eye, self.direction, glam::Vec3::Y);
        let projection =
            glam::Mat4::perspective_rh(self.fov, self.aspect, self.clip_near, self.clip_far);

        projection * view
    }
}

fn flush_render_system(
    queue: Res<WgpuQueue>,
    mut command_buffers: ResMut<Events<CommandBufferFinishedEvent>>,
    mut cameras: Query<&mut Camera>,
) {
    queue
        .0
        .submit(command_buffers.drain().map(|buffer| buffer.0));

    for mut camera in cameras.iter_mut() {
        if let Some(output) = camera.output.take() {
            output.present()
        }
    }
}

fn reconfigure_device_on_resize_system(
    mut resize_event: EventReader<ResizeEvent>,
    device: Res<WgpuDevice>,
    surface: Res<WgpuSurface>,
    mut config: ResMut<WgpuConfig>,
    mut cameras: Query<&mut Camera>,
) {
    for e in resize_event.read() {
        log::info!("Resizing to {:?}", e.0);
        let new_size = e.0;
        config.0.width = new_size.width;
        config.0.height = new_size.height;
        surface.0.configure(&device.0, &config.0);

        let aspect = new_size.width as f32 / new_size.height as f32;
        for mut camera in cameras.iter_mut() {
            camera.aspect = aspect;
        }
    }
}
