use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use rusty_map::run_transform;
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
};

const TEMPLATE: &str = "\
// export function before_transform() {
//   return {};
// }

// export function try_filter(item, ctx) {
//   return true;
// }

export function try_map(item, ctx) {
  return item;
}

// export function after_transform(result) {
//   return result;
// }
";

#[derive(Subcommand)]
enum Commands {
    /// Print an example JavaScript transform template to stdout
    GenerateTemplate,
}

#[derive(Parser)]
#[command(name = "rusty_map", about = "Transform JSON via a JavaScript script")]
#[command(subcommand_negates_reqs = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Path to input JSON file (reads stdin if omitted)
    #[arg(short = 'f', long = "file")]
    json_file: Option<PathBuf>,

    /// Path to JavaScript script file (must export a default function)
    #[arg(required = true)]
    script_file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if let Some(Commands::GenerateTemplate) = cli.command {
        print!("{}", TEMPLATE);
        return Ok(());
    }

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

    let script_file = cli.script_file.unwrap();
    let script = fs::read_to_string(&script_file)
        .with_context(|| format!("Failed to read script: {}", script_file.display()))?;

    let output = run_transform(&json_str, &script)?;
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
