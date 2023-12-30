use std::time::{Duration, Instant};

pub trait TimestepScheduler {
    fn update(&mut self, function: impl FnMut(f32));
    fn render(&mut self, function: impl FnMut());
    fn after_frame(&mut self);
}

pub struct FixedUpdateScheduler {
    target_tps: u16,
    delta: f32,
    last_tick: Instant,
    target_frame_time: Duration,
}

impl FixedUpdateScheduler {
    pub fn new(target_tps: u16, target_fps: u16) -> Self {
        assert!(
            target_tps >= target_fps,
            "TODO: this case isnt handled correctly yet"
        );

        Self {
            target_tps,
            delta: 1.0 / target_tps as f32,
            last_tick: Instant::now(),
            target_frame_time: Duration::from_secs_f32(1.0 / target_fps as f32),
        }
    }
}

impl TimestepScheduler for FixedUpdateScheduler {
    #[inline]
    fn update(&mut self, mut function: impl FnMut(f32)) {
        let amount_ticks =
            (self.last_tick.elapsed().as_secs_f32() * self.target_tps as f32) as usize;

        for x in 0..amount_ticks {
            if x == amount_ticks - 1 {
                self.last_tick = Instant::now();
            }
            function(self.delta);
        }
    }

    #[inline]
    fn render(&mut self, mut function: impl FnMut()) {
        function();
    }

    #[inline]
    fn after_frame(&mut self) {
        if let Some(duration) = self.target_frame_time.checked_sub(self.last_tick.elapsed()) {
            std::thread::sleep(duration);
        }
    }
}
