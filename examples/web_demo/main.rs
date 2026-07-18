//! Web demo: loads a Live2D model in the browser using WebGPU.
//!
//! Build with trunk: `cd examples/web_demo && trunk serve`

use std::sync::{Arc, Mutex};

use wasm_bindgen::prelude::*;
use winit::platform::web::EventLoopExtWebSys;

use mocari::engine::Live2dEngine;

fn set_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        web_sys::console::error_1(&format!("[mocari PANIC] {msg}").into());
    }));
}

#[wasm_bindgen(start)]
pub fn main() {
    set_panic_hook();
    web_sys::console::log_1(&"[mocari] wasm module loaded".into());
    let event_loop = match winit::event_loop::EventLoop::new() {
        Ok(el) => el,
        Err(e) => {
            web_sys::console::error_1(&format!("[mocari] event loop failed: {e}").into());
            return;
        }
    };
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);
    web_sys::console::log_1(&"[mocari] spawning app".into());
    event_loop.spawn_app(WebApp::default());
}

type SharedEngine = Arc<Mutex<Option<Live2dEngine>>>;

#[derive(Default)]
struct WebApp {
    window: Option<Arc<winit::window::Window>>,
    engine: SharedEngine,
    init_started: bool,
}

impl winit::application::ApplicationHandler for WebApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        web_sys::console::log_1(&"[mocari] resumed called".into());
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

        web_sys::console::log_1(&"[mocari] window created".into());
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
        // Kick off async engine + model init once
        if !self.init_started {
            if let Some(window) = &self.window {
                web_sys::console::log_1(&"[mocari] about_to_wait: starting engine init".into());
                self.init_started = true;
                let engine = self.engine.clone();
                let window = window.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    init_engine(engine, window).await;
                });
            }
        }

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

async fn init_engine(engine: SharedEngine, window: Arc<winit::window::Window>) {
    use mocari::engine::web;

    let mut e = match Live2dEngine::new(window.clone()).await {
        Ok(e) => e,
        Err(err) => {
            web_sys::console::error_1(&format!("engine init failed: {err}").into());
            return;
        }
    };

    // Fetch model assets
    let model_json = match web::fetch_text("/models/Ren/Ren.model3.json").await {
        Ok(s) => s,
        Err(err) => {
            web_sys::console::error_1(&format!("fetch model json failed: {err}").into());
            return;
        }
    };
    let moc3 = match web::fetch_bytes("/models/Ren/Ren.moc3").await {
        Ok(b) => b,
        Err(err) => {
            web_sys::console::error_1(&format!("fetch moc3 failed: {err}").into());
            return;
        }
    };
    let tex = match web::fetch_bytes("/models/Ren/Ren.2048/texture_00.png").await {
        Ok(b) => b,
        Err(err) => {
            web_sys::console::error_1(&format!("fetch texture failed: {err}").into());
            return;
        }
    };

    let tex_ref: &[u8] = &tex;
    let textures: Vec<&[u8]> = vec![tex_ref];
    if let Err(err) = e.load_model_from_bytes(&model_json, &moc3, &textures) {
        web_sys::console::error_1(&format!("load model failed: {err}").into());
        return;
    }

    web_sys::console::log_1(&"model loaded!".into());
    *engine.lock().unwrap() = Some(e);
    window.request_redraw();
}
