#![allow(dead_code)]

mod export;
mod face_detect;
mod layer_gen;
mod mesh;
mod motion;
mod physics;
mod pipeline;
mod rigging;
mod types;

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
        } => {
            let types: Vec<String> = motion_types
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();
            pipeline::run_pipeline(&image_path, &output_dir, &model_name, &types)?;
        }
    }
    Ok(())
}
