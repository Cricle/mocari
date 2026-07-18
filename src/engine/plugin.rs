/// Extension point for custom behavior.
pub trait Live2dPlugin: Send + Sync {
    /// Called after all models tick, before render.
    fn on_frame(&mut self, _ctx: &mut FrameContext) {}
    /// Called after Live2D model renders, before present.
    fn on_render(&mut self, _ctx: &mut RenderContext) {}
    /// Called when a model is loaded.
    fn on_model_loaded(&mut self, _model: &super::ModelHandle) {}
    /// Called when a model is about to be unloaded.
    fn on_model_unloaded(&mut self, _model_id: &str) {}
}

/// Context passed to frame callbacks and plugin `on_frame`.
pub struct FrameContext<'a> {
    // Fields populated by engine
    pub(super) delta: f32,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
}

impl FrameContext<'_> {
    /// Returns the frame delta in seconds.
    pub fn delta(&self) -> f32 {
        self.delta
    }

    /// Returns the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        self.device
    }

    /// Returns the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        self.queue
    }
}

/// Context passed to render callbacks and plugin `on_render`.
pub struct RenderContext<'a> {
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) view: &'a wgpu::TextureView,
    pub(super) renderer: &'a super::WgpuLive2dRenderer,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
}

impl RenderContext<'_> {
    /// Returns the command encoder for custom render passes.
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
    }

    /// Returns the current frame texture view.
    pub fn view(&self) -> &wgpu::TextureView {
        self.view
    }

    /// Returns the Live2D renderer for custom draw calls.
    pub fn renderer(&self) -> &super::WgpuLive2dRenderer {
        self.renderer
    }

    /// Returns the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        self.device
    }

    /// Returns the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        self.queue
    }
}
