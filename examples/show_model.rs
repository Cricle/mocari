use std::sync::Arc;
use std::time::Instant;

use mocari::engine::Live2dEngine;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = ShowModelApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct ShowModelApp {
    state: Option<WindowState>,
}

struct WindowState {
    engine: Live2dEngine,
    window: Arc<Window>,
    last_frame: Instant,
}

impl ApplicationHandler for ShowModelApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Live2D - Mocari Engine")
            .with_inner_size(LogicalSize::new(900u32, 900u32));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("failed to create window: {error}");
                event_loop.exit();
                return;
            }
        };

        match pollster::block_on(Live2dEngine::new(window.clone())) {
            Ok(mut engine) => {
                if let Err(e) = engine.load_model("assets/models/Ren/Ren.model3.json") {
                    eprintln!("failed to load model: {e}");
                    event_loop.exit();
                    return;
                }
                window.request_redraw();
                self.state = Some(WindowState {
                    engine,
                    window,
                    last_frame: Instant::now(),
                });
            }
            Err(error) => {
                eprintln!("failed to initialize engine: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if state.window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.engine.resize(size);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                state.engine.resize(state.window.inner_size());
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta = now.duration_since(state.last_frame).as_secs_f32();
                state.last_frame = now;

                state.engine.tick(delta);
                if let Err(e) = state.engine.render() {
                    eprintln!("render failed: {e}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref()
            && state.engine.needs_continuous_redraw()
        {
            state.window.request_redraw();
        }
    }
}
