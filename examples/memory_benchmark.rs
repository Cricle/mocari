//! Memory usage benchmark for model loading and runtime operations.
//!
//! Run with: `cargo bench --bench memory_benchmark`

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Delegating to System allocator which is safe
        let ret = unsafe { System.alloc(layout) };
        if !ret.is_null() {
            let size = layout.size();
            let current = ALLOCATED.fetch_add(size, Ordering::SeqCst) + size;
            let mut peak = PEAK.load(Ordering::SeqCst);
            while current > peak {
                match PEAK.compare_exchange_weak(peak, current, Ordering::SeqCst, Ordering::SeqCst) {
                    Ok(_) => break,
                    Err(p) => peak = p,
                }
            }
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Delegating to System allocator which is safe
        unsafe { System.dealloc(ptr, layout) };
        ALLOCATED.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn reset_memory_tracking() {
    ALLOCATED.store(0, Ordering::SeqCst);
    PEAK.store(0, Ordering::SeqCst);
}

fn current_memory() -> usize {
    ALLOCATED.load(Ordering::SeqCst)
}

fn peak_memory() -> usize {
    PEAK.load(Ordering::SeqCst)
}

use mocari::assets::load_model_runtime;

fn benchmark_model_load(model_name: &str) {
    reset_memory_tracking();

    let baseline = current_memory();
    let path = format!("assets/models/{}/{}.model3.json", model_name, model_name);

    let load_start = std::time::Instant::now();
    let model = load_model_runtime(&path).expect("failed to load model");
    let load_time = load_start.elapsed();

    let loaded_memory = current_memory() - baseline;
    let peak_during_load = peak_memory() - baseline;

    // Simulate frame updates
    let mut runtime = model.runtime().clone();
    reset_memory_tracking();

    let frame_start = std::time::Instant::now();
    for _ in 0..100 {
        runtime.set_parameter("ParamAngleX", 10.0);
        runtime.update_meshes();
    }
    let frame_time = frame_start.elapsed();

    let frame_memory = peak_memory();

    println!("=== {} ===", model_name);
    println!("Load time: {:.2}ms", load_time.as_secs_f64() * 1000.0);
    println!("Loaded memory: {:.2}MB", loaded_memory as f64 / 1_048_576.0);
    println!("Peak during load: {:.2}MB", peak_during_load as f64 / 1_048_576.0);
    println!("Frame memory (100 updates): {:.2}MB", frame_memory as f64 / 1_048_576.0);
    println!("Avg frame time: {:.2}µs", frame_time.as_micros() as f64 / 100.0);
    println!();
}

fn main() {
    let models = ["Haru", "Hiyori", "Mao", "Mark", "Natori", "Ren", "Rice", "Wanko"];

    println!("Mocari Memory & Performance Benchmark");
    println!("=====================================\n");

    for model in models {
        benchmark_model_load(model);
    }
}
