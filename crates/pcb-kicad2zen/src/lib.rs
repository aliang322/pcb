//! KiCad to Zener importer
//!
//! Converts KiCad projects (`.kicad_sch`, `.kicad_pcb`, `.kicad_pro`) to Zener (`.zen`) files.

pub mod mapping;
pub mod parser;

use anyhow::{Context, Result};
use std::path::Path;

pub use parser::{KicadPcb, KicadPro, KicadSchematic};

/// Output mode for Zen generation
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputMode {
    /// Preserve exact KiCad data for round-trip fidelity
    #[default]
    Faithful,
    /// Map to stdlib generics for human-readable output
    Idiomatic,
}

/// A parsed KiCad project
#[derive(Debug, Default)]
pub struct KicadProject {
    /// Project name (derived from directory or file name)
    pub name: String,
    /// Parsed schematic data
    pub schematic: Option<KicadSchematic>,
    /// Parsed PCB layout data
    pub pcb: Option<KicadPcb>,
    /// Parsed project settings
    pub project: Option<KicadPro>,
}

impl KicadProject {
    /// Parse a KiCad project from a directory
    pub fn parse(dir: &Path) -> Result<Self> {
        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("project")
            .to_string();

        let mut project = KicadProject {
            name,
            schematic: None,
            pcb: None,
            project: None,
        };

        // Find and parse schematic files
        for entry in std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read directory: {}", dir.display()))?
        {
            let entry = entry?;
            let path = entry.path();

            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                match ext {
                    "kicad_sch" => {
                        project.schematic = Some(KicadSchematic::parse(&path)?);
                    }
                    "kicad_pcb" => {
                        project.pcb = Some(KicadPcb::parse(&path)?);
                    }
                    "kicad_pro" => {
                        project.project = Some(KicadPro::parse(&path)?);
                    }
                    _ => {}
                }
            }
        }

        Ok(project)
    }

    /// Convert the project to Zener source code
    pub fn to_zen(&self, _mode: OutputMode) -> String {
        // TODO: Implement in subsequent commits
        String::new()
    }
}
