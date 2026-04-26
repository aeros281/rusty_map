use anyhow::{Context, Result};
use clap::Parser;
use rusty_map::run_transform;
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};

#[derive(Parser)]
#[command(name = "rusty_map", about = "Transform JSON via a JavaScript script")]
struct Cli {
    /// Path to input JSON file (reads stdin if omitted)
    #[arg(short = 'f', long = "file")]
    json_file: Option<PathBuf>,

    /// Path to JavaScript script file (must export a default function)
    script_file: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let json_str = match cli.json_file {
        Some(path) => fs::read_to_string(&path)
            .with_context(|| format!("Failed to read JSON file: {}", path.display()))?,
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .context("Failed to read JSON from stdin")?;
            buf
        }
    };

    let script = fs::read_to_string(&cli.script_file)
        .with_context(|| format!("Failed to read script: {}", cli.script_file.display()))?;

    let output = run_transform(&json_str, &script)?;
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
