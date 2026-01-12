//! Import command for converting external formats to Zener

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use pcb_kicad2zen::{KicadProject, OutputMode};
use std::fs;
use std::path::PathBuf;

/// Arguments for the `import` command
#[derive(Args, Debug)]
#[command(about = "Import designs from external formats")]
pub struct ImportArgs {
    #[command(subcommand)]
    pub command: ImportCommands,
}

#[derive(Subcommand, Debug)]
pub enum ImportCommands {
    /// Import a KiCad project to Zener format
    Kicad(KicadArgs),
}

/// Arguments for the `import kicad` subcommand
#[derive(Args, Debug)]
pub struct KicadArgs {
    /// Path to KiCad project directory containing .kicad_sch, .kicad_pcb files
    #[arg(value_name = "PATH", value_hint = clap::ValueHint::DirPath)]
    pub path: PathBuf,

    /// Output file path (defaults to <project-name>-imported.zen in current directory)
    #[arg(short, long, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    pub output: Option<PathBuf>,

    /// Print to stdout instead of writing to file
    #[arg(long)]
    pub stdout: bool,
}

/// Execute the `import` command
pub fn execute(args: ImportArgs) -> Result<()> {
    match args.command {
        ImportCommands::Kicad(args) => execute_kicad(args),
    }
}

/// Execute the `import kicad` subcommand
fn execute_kicad(args: KicadArgs) -> Result<()> {
    // Validate input path
    if !args.path.exists() {
        anyhow::bail!("Path does not exist: {}", args.path.display());
    }
    if !args.path.is_dir() {
        anyhow::bail!(
            "Path must be a directory containing KiCad files: {}",
            args.path.display()
        );
    }

    // Parse the KiCad project
    eprintln!("Parsing KiCad project: {}", args.path.display());
    let project = KicadProject::parse(&args.path)
        .with_context(|| format!("Failed to parse KiCad project: {}", args.path.display()))?;

    // Report what was found
    let mut found_any = false;
    if project.schematic.is_some() {
        eprintln!("  Found schematic (.kicad_sch)");
        found_any = true;
    }
    if project.pcb.is_some() {
        eprintln!("  Found PCB layout (.kicad_pcb)");
        found_any = true;
    }
    if project.project.is_some() {
        eprintln!("  Found project settings (.kicad_pro)");
    }

    // Warn if no KiCad files found
    if !found_any {
        eprintln!("  Warning: No .kicad_sch or .kicad_pcb files found in directory");
    }

    // Generate Zener output
    let zen_output = project.to_zen(OutputMode::Idiomatic);

    // Output
    if args.stdout {
        println!("{}", zen_output);
    } else {
        let output_path = args.output.unwrap_or_else(|| {
            let name = project.name.clone();
            PathBuf::from(format!("{}-imported.zen", name))
        });

        fs::write(&output_path, &zen_output)
            .with_context(|| format!("Failed to write output file: {}", output_path.display()))?;

        eprintln!("Wrote {}", output_path.display());
    }

    Ok(())
}
