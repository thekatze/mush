pub mod rendering;
pub mod sprites;

pub trait Plugin {
    fn build(self, world: &mut bevy_ecs::world::World, schedule: &mut bevy_ecs::schedule::Schedule);
}
