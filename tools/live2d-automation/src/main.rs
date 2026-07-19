use live2d_automation::{config, pipeline};

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "live2d-automation", about = "Automated Live2D model generation from character images")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    /// Run the full pipeline from image to model bundle
    Run {
        /// Path to the input character image
        #[arg(long)]
        image_path: String,
        /// Output directory for the model bundle
        #[arg(long)]
        output_dir: String,
        /// Name for the generated model
        #[arg(long)]
        model_name: String,
        /// Motion types to generate (comma-separated: idle,tap,move,emotional)
        #[arg(long, default_value = "idle,tap,move,emotional")]
        motion_types: String,
        /// Path to configuration file (optional)
        #[arg(long)]
        config: Option<String>,
    },
    /// Generate default configuration file
    InitConfig {
        /// Output path for configuration file
        #[arg(long, default_value = "live2d-config.json")]
        output: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Run {
            image_path,
            output_dir,
            model_name,
            motion_types,
            config,
        } => {
            let _config = if let Some(config_path) = config {
                config::PipelineConfig::from_file(&config_path)?
            } else {
                config::PipelineConfig::default()
            };

            let motion_list: Vec<String> = motion_types
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            pipeline::run_pipeline(&image_path, &output_dir, &model_name, &motion_list)?;
        }
        Command::InitConfig { output } => {
            let config = config::PipelineConfig::default();
            config.save(&output)?;
            println!("Configuration file created: {output}");
        }
    }
    Ok(())
}
