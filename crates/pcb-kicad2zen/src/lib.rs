//! KiCad to Zener importer
//!
//! Converts KiCad projects (`.kicad_sch`, `.kicad_pcb`, `.kicad_pro`) to Zener (`.zen`) files.

pub mod emit;
pub mod mapping;
pub mod parser;

use anyhow::{Context, Result};
use std::path::Path;

pub use emit::{emit_zen, OutputMode};
pub use parser::{KicadPcb, KicadPro, KicadSchematic};

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
    pub fn to_zen(&self, mode: OutputMode) -> String {
        emit::emit_zen(self, mode)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_emit_zen() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/pcb-sch/test/kicad-bom");
        let project = KicadProject::parse(&path).unwrap();

        let output = project.to_zen(OutputMode::Idiomatic);

        // Should have header
        assert!(output.contains("# Auto-generated from KiCad project"));

        // Should have imports
        assert!(output.contains("load(\"@stdlib/board_config.zen\", \"Board\")"));

        // Should have module alias for Resistor
        assert!(output.contains("Resistor = Module(\"@stdlib/generics/Resistor.zen\")"));

        // Should use Resistor() generic
        assert!(output.contains("Resistor("));
        assert!(output.contains("name = \"R1\""));
        assert!(output.contains("name = \"R2\""));
        assert!(output.contains("name = \"R3\""));

        // Should have package extracted
        assert!(output.contains("package = \"0402\""));

        // Should have Board() call
        assert!(output.contains("Board("));
    }
}
