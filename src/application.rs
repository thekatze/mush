use std::sync::Arc;

use crate::graphics::wgpu::WgpuGraphics;
use crate::timestep_scheduler::TimestepScheduler;
use bevy_ecs::{schedule::Schedule, world::World};

use wgpu::CommandEncoder;
use winit::{
    event_loop::{self, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::{
    resources::{Delta, RenderContext},
    timestep_scheduler::FixedUpdateScheduler,
};

pub struct Application {
    world: World,
    graphics: WgpuGraphics,
    // window has to be after wgpu, because it has unsafe references onto the window
    window: Window,
    event_loop: EventLoop<()>,
}

impl Application {
    pub async fn build() -> Result<Self, anyhow::Error> {
        let event_loop = event_loop::EventLoop::new()?;
        let window = WindowBuilder::new().build(&event_loop)?;

        Ok(Self {
            world: World::new(),
            graphics: WgpuGraphics::build_for_window(&window).await?,
            window,
            event_loop,
        })
    }

    pub fn run(mut self) {
        log::info!("Starting application");

        use winit::event::Event;
        use winit::event::WindowEvent;

        let mut update_schedule = Schedule::default();

        let mut render_schedule = Schedule::default();
        render_schedule.add_systems(crate::systems::clear_screen_system::clear_screen_system);

        self.window.set_title("Made with Unity(TM)");

        let mut scheduler = FixedUpdateScheduler::new(60, 60);

        self.event_loop
            .run(move |event, window| match event {
                Event::AboutToWait => {
                    scheduler.after_frame();
                    self.window.request_redraw();
                }
                Event::WindowEvent {
                    window_id,
                    event: window_event,
                } if window_id == self.window.id() => match window_event {
                    WindowEvent::RedrawRequested => {
                        scheduler.update(|delta| {
                            self.world.insert_resource(Delta(delta));
                            update_schedule.run(&mut self.world);
                        });

                        self.world.remove_resource::<Delta>();

                        scheduler.render(|| {
                            let (tx, rx) = std::sync::mpsc::channel::<CommandEncoder>();

                            let output = self
                                .graphics
                                .surface
                                .get_current_texture()
                                .expect("cant get surface to draw on");

                            {
                                let view = Arc::new(
                                    output
                                        .texture
                                        .create_view(&wgpu::TextureViewDescriptor::default()),
                                );

                                let context = RenderContext {
                                    device: std::sync::Arc::clone(&self.graphics.device),
                                    view: view.clone(),
                                    tx,
                                };

                                self.world.insert_resource(context);

                                render_schedule.run(&mut self.world);
                            }

                            self.graphics
                                .queue
                                .submit(rx.try_iter().map(|x| x.finish()));

                            output.present();
                        });
                    }
                    WindowEvent::CloseRequested => {
                        window.exit();
                    }
                    WindowEvent::KeyboardInput {
                        device_id: _,
                        event,
                        is_synthetic: _,
                    } => {
                        if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                            window.exit();
                        }
                    }
                    // TODO: handle scale factor changed event
                    WindowEvent::Resized(size) => {
                        self.graphics.resize(size);
                    }
                    _ => (),
                },
                _ => (),
            })
            .expect("Event loop failed");
    }
}
