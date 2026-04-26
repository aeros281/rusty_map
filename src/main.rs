use anyhow::{Context, Result};
use clap::Parser;
use rusty_map::run_transform;
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[command(name = "rusty_map", about = "Transform JSON via a JavaScript script")]
struct Cli {
    /// Path to input JSON file
    json_file: PathBuf,

    /// Path to JavaScript script file (must export a default function)
    script_file: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let json_str = fs::read_to_string(&cli.json_file)
        .with_context(|| format!("Failed to read JSON file: {}", cli.json_file.display()))?;

    let script = fs::read_to_string(&cli.script_file)
        .with_context(|| format!("Failed to read script: {}", cli.script_file.display()))?;

    let output = run_transform(&json_str, &script)?;
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
