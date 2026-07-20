//! Simple example showing Mocari's high-level API.
//!
//! Run with: cargo run --example simple --features wgpu

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simplest usage - just show a model with default settings
    Ok(mocari::engine::run("assets/models/Haru/Haru.model3.json")?)
}

// For custom window size and title, use run_with_config:
#[allow(dead_code)]
fn custom_window() -> Result<(), Box<dyn std::error::Error>> {
    use mocari::engine::RunConfig;

    Ok(mocari::engine::run_with_config(
        "assets/models/Haru/Haru.model3.json",
        RunConfig::new()
            .title("My Custom Title")
            .size(800, 600),
    )?)
}

// For full control over the engine, load manually and use the API:
#[allow(dead_code)]
fn advanced_usage() -> Result<(), Box<dyn std::error::Error>> {
    use mocari::assets::load_model_runtime;
    use mocari::{MotionPlayer, ExpressionManager};
    use mocari::motion::load_motion;
    use mocari::expression::load_expression;

    // Load model
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json")?;
    let runtime = model.runtime_mut();

    // Set parameters directly
    runtime.set_parameter("ParamAngleX", 10.0);
    runtime.set_parameter_normalized("ParamEyeLOpen", 0.5);

    // Play a motion
    let motion = load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json")?;
    let mut motion_player = MotionPlayer::new(motion);
    motion_player.tick(0.016); // 60fps delta
    motion_player.apply(runtime);

    // Apply an expression
    let expression = load_expression("assets/models/Haru/expressions/f01.exp3.json")?;
    let mut expr_manager = ExpressionManager::new();
    expr_manager.play(expression);
    expr_manager.tick(0.016);
    expr_manager.apply(runtime);

    // Update meshes after all parameter changes
    runtime.update_meshes();

    // Access mesh data for custom rendering
    for mesh in runtime.meshes() {
        println!("Drawable: {} vertices, {} indices",
                 mesh.vertices().len(), mesh.indices().len());
    }

    Ok(())
}

// Hit testing example
#[allow(dead_code)]
fn hit_test_example() -> Result<(), Box<dyn std::error::Error>> {
    use mocari::assets::load_model_runtime;

    let model = load_model_runtime("assets/models/Haru/Haru.model3.json")?;
    let runtime = model.runtime();

    // Hit test at canvas coordinates (normalized -1 to 1)
    if let Some(hit) = runtime.hit_test(0.0, 0.0) {
        println!("Hit area: {} (ID: {})", hit.name(), hit.id());
    }

    Ok(())
}
