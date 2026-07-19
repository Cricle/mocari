//! Web demo: multi-model Live2D viewer with mouse interaction and FPS display.
//!
//! Build: `cargo build --target wasm32-unknown-unknown --features web --example web_demo --release`
//! Deploy: `wasm-bindgen --target web --out-dir examples/web_demo/dist target/wasm32-unknown-unknown/release/examples/web_demo.wasm`

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Promise, Uint8Array};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use mocari::engine::{Live2dEngine, ModelHandle};
use wgpu::Color;

const MODELS: &[&str] = &["Haru", "Hiyori", "Mao", "Mark", "Natori", "Ren", "Rice", "Wanko"];

struct State {
    engine: RefCell<Live2dEngine>,
    handle: RefCell<ModelHandle>,
    mouse: RefCell<[f32; 2]>,
    stats: RefCell<ModelStats>,
}

#[derive(Default, Clone)]
struct ModelStats {
    drawables: usize,
    vertices: usize,
    triangles: usize,
    parameters: usize,
}

struct FpsCounter {
    times: [f64; 61],
    head: usize,
    len: usize,
}

impl FpsCounter {
    fn new() -> Self {
        Self { times: [0.0; 61], head: 0, len: 0 }
    }

    fn push(&mut self, t: f64) {
        self.times[self.head] = t;
        self.head = (self.head + 1) % 61;
        if self.len < 61 { self.len += 1; }
    }

    fn fps(&self) -> f64 {
        if self.len < 2 { return 0.0; }
        let newest = self.times[(self.head + 60) % 61];
        let oldest = if self.len == 61 { self.times[self.head] } else { self.times[0] };
        (self.len - 1) as f64 / (newest - oldest) * 1000.0
    }
}

async fn fetch_batch(base: &str, paths: &[&str]) -> Result<Vec<Vec<u8>>, JsValue> {
    let win = web_sys::window().unwrap();
    let promises: Array = paths.iter().map(|p| {
        let url = format!("{base}/{p}");
        let req = web_sys::Request::new_with_str(&url).unwrap();
        win.fetch_with_request(&req)
    }).collect();
    let all = JsFuture::from(Promise::all(&promises)).await?;
    let arr: Array = all.unchecked_into();
    let mut results = Vec::with_capacity(arr.length() as usize);
    for i in 0..arr.length() {
        let resp: web_sys::Response = arr.get(i).unchecked_into();
        let buf = JsFuture::from(resp.array_buffer()?).await?;
        results.push(Uint8Array::new(&buf).to_vec());
    }
    Ok(results)
}

#[wasm_bindgen]
pub fn main() {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&format!("[mocari PANIC] {info}").into());
    }));

    let win = web_sys::window().unwrap();
    let canvas = win.document().unwrap().get_element_by_id("live2d").unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>().unwrap();

    let dpr = win.device_pixel_ratio();
    let rect = canvas.get_bounding_client_rect();
    canvas.set_width((rect.width() * dpr).max(256.0) as u32);
    canvas.set_height((rect.height() * dpr).max(256.0) as u32);

    wasm_bindgen_futures::spawn_local(run(canvas));
}

async fn run(canvas: web_sys::HtmlCanvasElement) {
    let mut engine = match Live2dEngine::from_canvas(canvas.clone()).await {
        Ok(e) => e,
        Err(err) => {
            web_sys::console::error_1(&format!("engine init failed: {err}").into());
            return;
        }
    };
    // Match canvas CSS background (#16162a)
    engine.set_clear_color(Some(Color { r: 0.086, g: 0.086, b: 0.165, a: 1.0 }));

    let data = fetch_model(MODELS[0]).await;
    let handle = apply_model(&mut engine, &data);
    let stats = model_stats(&engine, &handle);
    let state = Rc::new(State {
        engine: RefCell::new(engine),
        handle: RefCell::new(handle),
        mouse: RefCell::new([0.0; 2]),
        stats: RefCell::new(stats),
    });

    // Mouse tracking
    {
        let st = state.clone();
        let c = canvas.clone();
        let cb = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
            let rect = c.get_bounding_client_rect();
            let x = ((e.client_x() as f64 - rect.left()) / rect.width()) as f32;
            let y = ((e.client_y() as f64 - rect.top()) / rect.height()) as f32;
            *st.mouse.borrow_mut() = [x * 2.0 - 1.0, y * 2.0 - 1.0];
        }) as Box<dyn FnMut(_)>);
        let _ = canvas.add_event_listener_with_callback("mousemove", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    // Model selector
    {
        let st = state.clone();
        let sel = el("model-selector");
        let s = sel.clone();
        let cb = Closure::wrap(Box::new(move |e: web_sys::MouseEvent| {
            let target: web_sys::Element = match e.target().and_then(|t| t.dyn_into().ok()) {
                Some(el) => el,
                None => return,
            };
            // Walk up to find the button with data-model
            let btn = target.closest("[data-model]").ok().flatten();
            let btn = match btn {
                Some(b) => b,
                None => return,
            };
            let name = btn.get_attribute("data-model").unwrap_or_default();
            let Some(idx) = MODELS.iter().position(|&m| m == name) else {
                web_sys::console::warn_1(&format!("[mocari] unknown model: '{name}'").into());
                return;
            };
            let children = s.children();
            for i in 0..children.length() {
                if let Some(c) = children.item(i) {
                    let _ = c.remove_attribute("class");
                }
            }
            btn.set_class_name("active");
            el("loading").set_class_name("show");
            let st = st.clone();
            wasm_bindgen_futures::spawn_local(async move {
                {
                    let mut e = st.engine.borrow_mut();
                    let old = st.handle.borrow().clone();
                    let _ = e.unload_model(&old);
                }
                let data = fetch_model(MODELS[idx]).await;
                let mut e = st.engine.borrow_mut();
                let new_h = apply_model(&mut e, &data);
                *st.stats.borrow_mut() = model_stats(&e, &new_h);
                drop(e);
                *st.handle.borrow_mut() = new_h;
                el("loading").set_class_name("");
            });
        }) as Box<dyn FnMut(_)>);
        let _ = sel.add_event_listener_with_callback("click", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    // Resize handler
    {
        let st = state.clone();
        let c = canvas.clone();
        let cb = Closure::wrap(Box::new(move || {
            let dpr = web_sys::window().unwrap().device_pixel_ratio();
            let rect = c.get_bounding_client_rect();
            let w = (rect.width() * dpr).max(256.0) as u32;
            let h = (rect.height() * dpr).max(256.0) as u32;
            if c.width() != w || c.height() != h {
                c.set_width(w);
                c.set_height(h);
                st.engine.borrow_mut().resize(winit::dpi::PhysicalSize::new(w, h));
            }
        }) as Box<dyn FnMut()>);
        let _ = web_sys::window().unwrap().add_event_listener_with_callback("resize", cb.as_ref().unchecked_ref());
        cb.forget();
    }

    // Render loop
    let stats_el = el("stats");
    let mut fps = FpsCounter::new();
    let perf = web_sys::window().unwrap().performance().unwrap();
    let f: Rc<RefCell<Option<Closure<dyn FnMut()>>>> = Rc::new(RefCell::new(None));
    let g = f.clone();
    let st = state.clone();

    *g.borrow_mut() = Some(Closure::new(move || {
        let frame_start = perf.now();

        let [mx, my] = *st.mouse.borrow();
        let handle = st.handle.borrow().clone();
        let mut e = st.engine.borrow_mut();
        if let Some(rt) = e.model_mut(&handle) {
            rt.set_parameter("ParamAngleX", mx * 30.0);
            rt.set_parameter("ParamAngleY", my * -30.0);
            rt.set_parameter("ParamBodyAngleX", mx * 10.0);
            rt.set_parameter("ParamEyeBallX", mx);
            rt.set_parameter("ParamEyeBallY", -my);
        }
        e.tick(1.0 / 60.0);
        let _ = e.render();
        drop(e);

        let frame_ms = perf.now() - frame_start;
        fps.push(perf.now());

        if fps.head % 30 == 0 {
            let ModelStats { drawables, vertices, triangles, parameters } = *st.stats.borrow();
            let mem = js_sys::Reflect::get(&js_sys::global(), &"performance".into())
                .ok()
                .and_then(|p| js_sys::Reflect::get(&p, &"memory".into()).ok())
                .and_then(|m| js_sys::Reflect::get(&m, &"usedJSHeapSize".into()).ok())
                .and_then(|v| v.as_f64())
                .map(|b| format!("  mem {:.1}MB", b / 1048576.0))
                .unwrap_or_default();
            stats_el.set_text_content(Some(&format!(
                "{:.0} fps  {:.1}ms\n{} draw  {} vert\n{} tri  {} param{}",
                fps.fps(), frame_ms,
                drawables, vertices,
                triangles, parameters, mem
            )));
        }

        web_sys::window().unwrap()
            .request_animation_frame(f.borrow().as_ref().unwrap().as_ref().unchecked_ref())
            .unwrap();
    }));

    web_sys::window().unwrap()
        .request_animation_frame(g.borrow().as_ref().unwrap().as_ref().unchecked_ref())
        .unwrap();
}

struct ModelData {
    json: String,
    motion: Option<String>,
    moc3: Vec<u8>,
    textures: Vec<Vec<u8>>,
}

async fn fetch_model(name: &str) -> ModelData {
    let base = format!("/models/{name}");
    let url = format!("{base}/{name}.model3.json");
    let req = web_sys::Request::new_with_str(&url).unwrap();
    let resp: web_sys::Response = JsFuture::from(
        web_sys::window().unwrap().fetch_with_request(&req)
    ).await.unwrap().unchecked_into();
    let json = JsFuture::from(resp.text().unwrap()).await.unwrap().as_string().unwrap();

    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let tex_paths: Vec<&str> = v["FileReferences"]["Textures"].as_array().unwrap()
        .iter().map(|t| t.as_str().unwrap()).collect();
    let motion_file = v["FileReferences"]["Motions"].as_object()
        .and_then(|g| g.get("Idle").or_else(|| g.values().next()))
        .and_then(|g| g.as_array())
        .and_then(|a| a.first())
        .and_then(|m| m["File"].as_str());

    let mut paths = vec![format!("{name}.moc3")];
    paths.extend(tex_paths.iter().map(|t| t.to_string()));
    let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
    let data = fetch_batch(&base, &path_refs).await.unwrap();

    let motion = if let Some(file) = motion_file {
        fetch_batch(&base, &[file]).await.ok()
            .and_then(|d| String::from_utf8(d[0].clone()).ok())
    } else {
        None
    };

    ModelData {
        json,
        motion,
        moc3: data[0].clone(),
        textures: data[1..].to_vec(),
    }
}

fn apply_model(engine: &mut Live2dEngine, data: &ModelData) -> ModelHandle {
    let tex_refs: Vec<&[u8]> = data.textures.iter().map(|b| b.as_slice()).collect();
    let handle = engine.load_model_from_bytes(&data.json, &data.moc3, &tex_refs).unwrap();
    if let Some(ref motion) = data.motion {
        let _ = engine.play_motion_from_json(&handle, motion);
    }
    handle
}

fn model_stats(engine: &Live2dEngine, handle: &ModelHandle) -> ModelStats {
    if let Some(rt) = engine.model(handle) {
        let meshes = rt.meshes();
        ModelStats {
            drawables: meshes.len(),
            vertices: meshes.iter().map(|m| m.vertices().len()).sum(),
            triangles: meshes.iter().map(|m| m.indices().len()).sum::<usize>() / 3,
            parameters: rt.parameter_ids().len(),
        }
    } else {
        ModelStats::default()
    }
}

fn el(id: &str) -> web_sys::Element {
    web_sys::window().unwrap().document().unwrap().get_element_by_id(id).unwrap()
}
