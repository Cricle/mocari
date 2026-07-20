use mocari::engine::DesktopPetConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Simplest usage — transparent frameless always-on-top pet window:
    //   mocari::engine::run_desktop_pet("assets/models/Ren/Ren.model3.json")?;

    // With custom config:
    mocari::engine::run_desktop_pet_with_config(
        "assets/models/Ren/Ren.model3.json",
        DesktopPetConfig::new()
            .size(400, 400)
            .title("Live2D Pet"),
    )?;
    Ok(())
}
