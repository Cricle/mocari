use std::sync::Arc;
use std::time::Instant;

use mocari::engine::{DesktopPetConfig, Live2dEngine};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = DesktopPetApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct DesktopPetApp {
    state: Option<PetState>,
}

struct PetState {
    engine: Live2dEngine,
    window: Arc<Window>,
    last_frame: Instant,
    is_interactive: bool,
}

impl ApplicationHandler for DesktopPetApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let config = DesktopPetConfig::new()
            .size(400, 400)
            .title("Live2D Pet");

        let window = match config.create_window(event_loop) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("failed to create window: {e}");
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
                self.state = Some(PetState {
                    engine,
                    window,
                    last_frame: Instant::now(),
                    is_interactive: false,
                });
            }
            Err(e) => {
                eprintln!("failed to initialize engine: {e}");
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

            WindowEvent::CursorMoved { position, .. } => {
                // Check if cursor is over the model area (center of window)
                let size = state.window.inner_size();
                let cx = size.width as f64 / 2.0;
                let cy = size.height as f64 / 2.0;
                let radius = (size.width.min(size.height) as f64) * 0.4;

                let dx = position.x - cx;
                let dy = position.y - cy;
                let over_model = dx * dx + dy * dy < radius * radius;

                if over_model && !state.is_interactive {
                    state.is_interactive = true;
                    let _ = state.engine.set_click_through(false);
                } else if !over_model && state.is_interactive {
                    state.is_interactive = false;
                    let _ = state.engine.set_click_through(true);
                }
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if state.is_interactive {
                    let _ = state.window.drag_window();
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // Escape to quit
                if event.state == ElementState::Pressed
                    && event.logical_key
                        == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
                {
                    event_loop.exit();
                }
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            if state.engine.needs_continuous_redraw() {
                let next = state.engine.next_render_time();
                event_loop.set_control_flow(ControlFlow::WaitUntil(next));
                state.window.request_redraw();
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }
}
