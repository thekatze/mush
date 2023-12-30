use bevy_ecs::system::Resource;

#[derive(Resource)]
pub struct Delta(pub f32);

#[derive(Resource)]
pub struct RenderContext {
    pub device: std::sync::Arc<wgpu::Device>,
    pub view: std::sync::Arc<wgpu::TextureView>,
    pub tx: std::sync::mpsc::Sender<wgpu::CommandEncoder>,
}

impl RenderContext {
    pub fn get_context(
        &self,
        label: &'_ str,
    ) -> (
        wgpu::CommandEncoder,
        std::sync::Arc<wgpu::TextureView>,
        std::sync::mpsc::Sender<wgpu::CommandEncoder>,
    ) {
        let encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });

        (encoder, std::sync::Arc::clone(&self.view), self.tx.clone())
    }
}
