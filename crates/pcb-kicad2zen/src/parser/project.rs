//! Parser for `.kicad_pro` project files (JSON format)

use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// Parsed KiCad project settings
#[derive(Debug, Default, Clone)]
pub struct KicadPro {
    /// Net class definitions
    pub net_classes: Vec<NetClass>,
    /// Design rules
    pub design_rules: DesignRules,
}

/// A net class definition
#[derive(Debug, Clone)]
pub struct NetClass {
    /// Net class name (e.g., "Default", "Power")
    pub name: String,
    /// Track width in mm
    pub track_width: f64,
    /// Clearance in mm
    pub clearance: f64,
    /// Via diameter in mm
    pub via_diameter: f64,
    /// Via drill in mm
    pub via_drill: f64,
    /// Differential pair width in mm
    pub diff_pair_width: f64,
    /// Differential pair gap in mm
    pub diff_pair_gap: f64,
}

impl Default for NetClass {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            track_width: 0.2,
            clearance: 0.2,
            via_diameter: 0.6,
            via_drill: 0.3,
            diff_pair_width: 0.2,
            diff_pair_gap: 0.25,
        }
    }
}

/// Design rules from the project file
#[derive(Debug, Default, Clone)]
pub struct DesignRules {
    /// Minimum clearance in mm
    pub min_clearance: f64,
    /// Minimum track width in mm
    pub min_track_width: f64,
    /// Minimum via diameter in mm
    pub min_via_diameter: f64,
    /// Minimum via drill in mm
    pub min_via_drill: f64,
    /// Minimum hole clearance in mm
    pub min_hole_clearance: f64,
    /// Minimum copper to edge clearance in mm
    pub min_copper_edge_clearance: f64,
}

impl KicadPro {
    /// Parse a `.kicad_pro` file
    pub fn parse(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read project file: {}", path.display()))?;

        Self::parse_str(&content)
    }

    /// Parse project from string content
    pub fn parse_str(content: &str) -> Result<Self> {
        let json: ProjectJson =
            serde_json::from_str(content).context("Failed to parse project JSON")?;

        let mut pro = KicadPro::default();

        // Parse net classes
        if let Some(net_settings) = json.net_settings {
            for class in net_settings.classes {
                pro.net_classes.push(NetClass {
                    name: class.name,
                    track_width: class.track_width,
                    clearance: class.clearance,
                    via_diameter: class.via_diameter,
                    via_drill: class.via_drill,
                    diff_pair_width: class.diff_pair_width,
                    diff_pair_gap: class.diff_pair_gap,
                });
            }
        }

        // Parse design rules
        if let Some(board) = json.board {
            if let Some(ds) = board.design_settings {
                if let Some(rules) = ds.rules {
                    pro.design_rules = DesignRules {
                        min_clearance: rules.min_clearance.unwrap_or(0.0),
                        min_track_width: rules.min_track_width.unwrap_or(0.0),
                        min_via_diameter: rules.min_via_diameter.unwrap_or(0.5),
                        min_via_drill: rules.min_via_drill.unwrap_or(0.0),
                        min_hole_clearance: rules.min_hole_clearance.unwrap_or(0.25),
                        min_copper_edge_clearance: rules.min_copper_edge_clearance.unwrap_or(0.5),
                    };
                }
            }
        }

        Ok(pro)
    }
}

// Internal JSON structures for deserialization

#[derive(Debug, Deserialize)]
struct ProjectJson {
    board: Option<BoardJson>,
    net_settings: Option<NetSettingsJson>,
}

#[derive(Debug, Deserialize)]
struct BoardJson {
    design_settings: Option<DesignSettingsJson>,
}

#[derive(Debug, Deserialize)]
struct DesignSettingsJson {
    rules: Option<RulesJson>,
}

#[derive(Debug, Deserialize)]
struct RulesJson {
    min_clearance: Option<f64>,
    min_track_width: Option<f64>,
    min_via_diameter: Option<f64>,
    min_via_drill: Option<f64>,
    min_hole_clearance: Option<f64>,
    min_copper_edge_clearance: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct NetSettingsJson {
    classes: Vec<NetClassJson>,
}

#[derive(Debug, Deserialize)]
struct NetClassJson {
    name: String,
    #[serde(default = "default_track_width")]
    track_width: f64,
    #[serde(default = "default_clearance")]
    clearance: f64,
    #[serde(default = "default_via_diameter")]
    via_diameter: f64,
    #[serde(default = "default_via_drill")]
    via_drill: f64,
    #[serde(default = "default_diff_pair_width")]
    diff_pair_width: f64,
    #[serde(default = "default_diff_pair_gap")]
    diff_pair_gap: f64,
}

fn default_track_width() -> f64 {
    0.2
}
fn default_clearance() -> f64 {
    0.2
}
fn default_via_diameter() -> f64 {
    0.6
}
fn default_via_drill() -> f64 {
    0.3
}
fn default_diff_pair_width() -> f64 {
    0.2
}
fn default_diff_pair_gap() -> f64 {
    0.25
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_project() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/pcb-sch/test/kicad-bom/layout.kicad_pro");
        let pro = KicadPro::parse(&path).unwrap();

        // Should have at least the Default net class
        assert!(!pro.net_classes.is_empty());

        let default_class = pro.net_classes.iter().find(|c| c.name == "Default").unwrap();
        assert!((default_class.track_width - 0.2).abs() < 0.01);
        assert!((default_class.clearance - 0.2).abs() < 0.01);
        assert!((default_class.via_diameter - 0.6).abs() < 0.01);
        assert!((default_class.via_drill - 0.3).abs() < 0.01);
    }
}
