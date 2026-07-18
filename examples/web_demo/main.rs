//! Web demo: loads a Live2D model in the browser using WebGPU.
//!
//! Build with trunk: `cd examples/web_demo && trunk serve`

use std::sync::{Arc, Mutex};

use wasm_bindgen::prelude::*;
use winit::platform::web::EventLoopExtWebSys;

use mocari::engine::Live2dEngine;

#[wasm_bindgen(start)]
pub fn main() {
    let event_loop = winit::event_loop::EventLoop::new().expect("event loop");
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    event_loop.spawn_app(WebApp::default());
}

type SharedEngine = Arc<Mutex<Option<Live2dEngine>>>;

#[derive(Default)]
struct WebApp {
    window: Option<Arc<winit::window::Window>>,
    engine: SharedEngine,
}

impl winit::application::ApplicationHandler for WebApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attrs = winit::window::Window::default_attributes()
            .with_title("Live2D - Mocari")
            .with_inner_size(winit::dpi::LogicalSize::new(900u32, 900u32));

        let window = match event_loop.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                web_sys::console::error_1(&format!("window creation failed: {e}").into());
                event_loop.exit();
                return;
            }
        };

        let engine = self.engine.clone();
        let window_clone = window.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match Live2dEngine::new(window_clone).await {
                Ok(e) => {
                    *engine.lock().unwrap() = Some(e);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("engine init failed: {e}").into());
                }
            }
        });

        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let mut guard = self.engine.lock().unwrap();
        let Some(engine) = guard.as_mut() else { return };
        match event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::Resized(size) => {
                engine.resize(size);
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            winit::event::WindowEvent::RedrawRequested => {
                engine.tick(1.0 / 60.0);
                let _ = engine.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let guard = self.engine.lock().unwrap();
        if let Some(engine) = guard.as_ref() {
            if engine.needs_continuous_redraw() {
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
        }
    }
}
