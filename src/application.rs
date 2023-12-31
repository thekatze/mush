use crate::plugins::{
    rendering::{init_render_schedule, Camera},
    sprites::SpritePlugin,
    Plugin,
};
use crate::timestep_scheduler::TimestepScheduler;
use bevy_ecs::{
    event::Event,
    schedule::{Schedule, ScheduleLabel},
    world::World,
};

use winit::{
    event_loop::{self, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::{resources::Delta, timestep_scheduler::FixedUpdateScheduler};

pub struct Application {
    world: World,
    // window has to be after wgpu, because it has unsafe references onto the window
    window: Window,
    event_loop: EventLoop<()>,
}

#[derive(Event)]
pub struct ResizeEvent(pub winit::dpi::PhysicalSize<u32>);

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct UpdateSchedule;

#[derive(ScheduleLabel, Hash, PartialEq, Eq, Debug, Clone)]
struct RenderSchedule;

impl Application {
    pub async fn build() -> Result<Self, anyhow::Error> {
        let event_loop = event_loop::EventLoop::new()?;
        let window = WindowBuilder::new().build(&event_loop)?;

        let mut world = World::new();

        let mut update_schedule = Schedule::new(UpdateSchedule);
        world.add_schedule(update_schedule);

        let mut render_schedule = Schedule::new(RenderSchedule);
        init_render_schedule(&mut world, &window, &mut render_schedule).await?;

        SpritePlugin {}.build(&mut world, &mut render_schedule);

        world.add_schedule(render_schedule);

        world.spawn(Camera {
            output: None,
            view: None,
        });

        Ok(Self {
            world,
            window,
            event_loop,
        })
    }

    pub fn run(mut self) {
        log::info!("Starting application");

        use winit::event::Event;
        use winit::event::WindowEvent;

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
                            self.world.run_schedule(UpdateSchedule);
                        });

                        self.world.remove_resource::<Delta>();

                        scheduler.render(|| {
                            self.world.run_schedule(RenderSchedule);
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
                    // TODO: maybe handle scale factor changed event
                    WindowEvent::Resized(size) => {
                        self.world.send_event(ResizeEvent(size));
                    }
                    _ => (),
                },
                _ => (),
            })
            .expect("Event loop failed");
    }
}
