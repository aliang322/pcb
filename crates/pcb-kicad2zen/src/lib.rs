//! KiCad to Zener importer
//!
//! Converts KiCad projects (`.kicad_sch`, `.kicad_pcb`, `.kicad_pro`) to Zener (`.zen`) files.

pub mod parser;

use anyhow::Result;
use std::path::Path;

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
    pub schematic: Option<parser::KicadSchematic>,
    /// Parsed PCB layout data
    pub pcb: Option<parser::KicadPcb>,
    /// Parsed project settings
    pub project: Option<parser::KicadPro>,
}

impl KicadProject {
    /// Parse a KiCad project from a directory
    pub fn parse(_dir: &Path) -> Result<Self> {
        // TODO: Implement in subsequent commits
        Ok(Self::default())
    }

    /// Convert the project to Zener source code
    pub fn to_zen(&self, _mode: OutputMode) -> String {
        // TODO: Implement in subsequent commits
        String::new()
    }
}
