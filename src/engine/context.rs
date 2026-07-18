use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

use crate::render::wgpu::preferred_surface_format;

use super::EngineError;

/// Owns wgpu device, queue, surface, and surface configuration.
pub(super) struct WgpuContext {
    #[allow(dead_code)]
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl WgpuContext {
    pub(super) async fn new(window: Arc<Window>) -> Result<Self, EngineError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| EngineError::Surface(e.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .map_err(|_| EngineError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("live2d.engine.device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(|e| EngineError::Device(e.to_string()))?;

        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| EngineError::Surface("surface not supported by adapter".into()))?;

        let capabilities = surface.get_capabilities(&adapter);
        config.format = preferred_surface_format(&capabilities.formats)
            .ok_or_else(|| EngineError::Surface("no suitable surface format".into()))?;

        // Prefer vsync (Fifo) for natural frame pacing and low idle CPU.
        // Falls back to AutoNoVsync if somehow Fifo isn't supported.
        config.present_mode = [wgpu::PresentMode::Fifo, wgpu::PresentMode::Mailbox]
            .into_iter()
            .find(|mode| capabilities.present_modes.contains(mode))
            .unwrap_or(wgpu::PresentMode::AutoNoVsync);

        config.desired_maximum_frame_latency = 3;
        surface.configure(&device, &config);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
        })
    }

    pub(super) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub(super) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub(super) fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    pub(super) fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub(super) fn window(&self) -> &Arc<Window> {
        &self.window
    }

    pub(super) fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }
}
