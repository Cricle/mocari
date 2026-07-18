use mocari::engine::RunConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simplest usage — default 900×900 window:
    //   mocari::engine::run("assets/models/Ren/Ren.model3.json")?;

    // With custom window config:
    mocari::engine::run_with_config(
        "assets/models/Ren/Ren.model3.json",
        RunConfig::new().title("Live2D - Mocari Engine").size(900, 900),
    )?;
    Ok(())
}
