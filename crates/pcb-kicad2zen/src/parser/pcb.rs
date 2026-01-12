//! Parser for `.kicad_pcb` PCB layout files

use anyhow::{Context, Result};
use pcb_sexpr::Sexpr;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parsed KiCad PCB layout
#[derive(Debug, Default)]
pub struct KicadPcb {
    /// KiCad version number
    pub version: u32,
    /// Board thickness in mm
    pub thickness: f64,
    /// Layer definitions
    pub layers: Vec<Layer>,
    /// Net definitions
    pub nets: HashMap<u32, String>,
    /// Footprint instances
    pub footprints: Vec<Footprint>,
}

/// A PCB layer
#[derive(Debug, Clone)]
pub struct Layer {
    /// Layer number
    pub number: u32,
    /// Layer name (e.g., "F.Cu", "B.Cu")
    pub name: String,
    /// Layer type (e.g., "signal", "user")
    pub layer_type: String,
}

/// A footprint instance on the PCB
#[derive(Debug, Clone)]
pub struct Footprint {
    /// Footprint UUID
    pub uuid: String,
    /// Footprint name (e.g., "Resistor_SMD:R_0402_1005Metric")
    pub footprint: String,
    /// Layer (e.g., "F.Cu", "B.Cu")
    pub layer: String,
    /// Position (x, y, rotation)
    pub at: (f64, f64, f64),
    /// Path to schematic symbol UUID
    pub path: String,
    /// Reference designator
    pub reference: String,
    /// Value
    pub value: String,
    /// Attributes (smd, through_hole, dnp, exclude_from_bom)
    pub attrs: Vec<String>,
    /// Pad definitions with net assignments
    pub pads: Vec<Pad>,
    /// All properties
    pub properties: HashMap<String, String>,
}

impl Footprint {
    /// Check if footprint has DNP attribute
    pub fn is_dnp(&self) -> bool {
        self.attrs.iter().any(|a| a == "dnp")
    }

    /// Check if footprint is excluded from BOM
    pub fn is_exclude_from_bom(&self) -> bool {
        self.attrs.iter().any(|a| a == "exclude_from_bom")
    }

    /// Check if footprint is SMD
    pub fn is_smd(&self) -> bool {
        self.attrs.iter().any(|a| a == "smd")
    }
}

/// A pad on a footprint
#[derive(Debug, Clone)]
pub struct Pad {
    /// Pad number/name (e.g., "1", "2", "A1")
    pub number: String,
    /// Pad type (e.g., "smd", "thru_hole")
    pub pad_type: String,
    /// Pad shape (e.g., "roundrect", "circle", "rect")
    pub shape: String,
    /// Position relative to footprint origin
    pub at: (f64, f64, f64),
    /// Net ID
    pub net_id: u32,
    /// Net name
    pub net_name: String,
}

impl KicadPcb {
    /// Parse a `.kicad_pcb` file
    pub fn parse(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read PCB file: {}", path.display()))?;

        Self::parse_str(&content)
    }

    /// Parse PCB from string content
    pub fn parse_str(content: &str) -> Result<Self> {
        let sexpr = pcb_sexpr::parse(content).context("Failed to parse PCB S-expression")?;

        let list = sexpr.as_list().context("Expected root list")?;
        if list.first().and_then(|s| s.as_sym()) != Some("kicad_pcb") {
            anyhow::bail!("Expected kicad_pcb root element");
        }

        let mut pcb = KicadPcb::default();

        for item in list.iter().skip(1) {
            if let Some(items) = item.as_list() {
                match items.first().and_then(|s| s.as_sym()) {
                    Some("version") => {
                        if let Some(v) = items.get(1) {
                            pcb.version = v.as_int().unwrap_or(0) as u32;
                        }
                    }
                    Some("general") => {
                        pcb.thickness = parse_general(items);
                    }
                    Some("layers") => {
                        pcb.layers = parse_layers(items);
                    }
                    Some("net") => {
                        if let (Some(id), Some(name)) = (
                            items.get(1).and_then(|s| s.as_int()),
                            items.get(2).map(get_string_value),
                        ) {
                            pcb.nets.insert(id as u32, name);
                        }
                    }
                    Some("footprint") => {
                        if let Ok(fp) = parse_footprint(items) {
                            pcb.footprints.push(fp);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(pcb)
    }
}

fn parse_general(list: &[Sexpr]) -> f64 {
    for item in list.iter().skip(1) {
        if let Some(items) = item.as_list() {
            if items.first().and_then(|s| s.as_sym()) == Some("thickness") {
                return items.get(1).and_then(|s| s.as_float()).unwrap_or(1.6);
            }
        }
    }
    1.6
}

fn parse_layers(list: &[Sexpr]) -> Vec<Layer> {
    let mut layers = Vec::new();

    for item in list.iter().skip(1) {
        if let Some(items) = item.as_list() {
            if let Some(num) = items.first().and_then(|s| s.as_int()) {
                let name = items
                    .get(1)
                    .map(|s| get_string_value(s))
                    .unwrap_or_default();
                let layer_type = items
                    .get(2)
                    .and_then(|s| s.as_sym())
                    .unwrap_or("user")
                    .to_string();

                layers.push(Layer {
                    number: num as u32,
                    name,
                    layer_type,
                });
            }
        }
    }

    layers
}

fn parse_footprint(list: &[Sexpr]) -> Result<Footprint> {
    let footprint_name = list
        .get(1)
        .map(get_string_value)
        .context("Missing footprint name")?;

    let mut fp = Footprint {
        uuid: String::new(),
        footprint: footprint_name,
        layer: String::new(),
        at: (0.0, 0.0, 0.0),
        path: String::new(),
        reference: String::new(),
        value: String::new(),
        attrs: Vec::new(),
        pads: Vec::new(),
        properties: HashMap::new(),
    };

    for item in list.iter().skip(2) {
        if let Some(items) = item.as_list() {
            match items.first().and_then(|s| s.as_sym()) {
                Some("uuid") => {
                    fp.uuid = items.get(1).map(get_string_value).unwrap_or_default();
                }
                Some("layer") => {
                    fp.layer = items.get(1).map(get_string_value).unwrap_or_default();
                }
                Some("at") => {
                    fp.at = (
                        items.get(1).and_then(|s| s.as_float()).unwrap_or(0.0),
                        items.get(2).and_then(|s| s.as_float()).unwrap_or(0.0),
                        items.get(3).and_then(|s| s.as_float()).unwrap_or(0.0),
                    );
                }
                Some("path") => {
                    fp.path = items.get(1).map(get_string_value).unwrap_or_default();
                }
                Some("property") => {
                    if let (Some(key), Some(val)) =
                        (items.get(1).and_then(|s| s.as_str()), items.get(2))
                    {
                        let value = get_string_value(val);
                        fp.properties.insert(key.to_string(), value.clone());

                        match key {
                            "Reference" => fp.reference = value,
                            "Value" => fp.value = value,
                            _ => {}
                        }
                    }
                }
                Some("attr") => {
                    for attr in items.iter().skip(1) {
                        if let Some(a) = attr.as_sym() {
                            fp.attrs.push(a.to_string());
                        }
                    }
                }
                Some("pad") => {
                    if let Ok(pad) = parse_pad(items) {
                        fp.pads.push(pad);
                    }
                }
                _ => {}
            }
        }
    }

    Ok(fp)
}

fn parse_pad(list: &[Sexpr]) -> Result<Pad> {
    let number = list.get(1).map(get_string_value).context("Missing pad number")?;
    let pad_type = list
        .get(2)
        .and_then(|s| s.as_sym())
        .unwrap_or("smd")
        .to_string();
    let shape = list
        .get(3)
        .and_then(|s| s.as_sym())
        .unwrap_or("rect")
        .to_string();

    let mut pad = Pad {
        number,
        pad_type,
        shape,
        at: (0.0, 0.0, 0.0),
        net_id: 0,
        net_name: String::new(),
    };

    for item in list.iter().skip(4) {
        if let Some(items) = item.as_list() {
            match items.first().and_then(|s| s.as_sym()) {
                Some("at") => {
                    pad.at = (
                        items.get(1).and_then(|s| s.as_float()).unwrap_or(0.0),
                        items.get(2).and_then(|s| s.as_float()).unwrap_or(0.0),
                        items.get(3).and_then(|s| s.as_float()).unwrap_or(0.0),
                    );
                }
                Some("net") => {
                    pad.net_id = items.get(1).and_then(|s| s.as_int()).unwrap_or(0) as u32;
                    pad.net_name = items.get(2).map(get_string_value).unwrap_or_default();
                }
                _ => {}
            }
        }
    }

    Ok(pad)
}

/// Extract string value from Sexpr (handles both Symbol and String variants)
fn get_string_value(sexpr: &Sexpr) -> String {
    sexpr
        .as_str()
        .or_else(|| sexpr.as_sym())
        .unwrap_or("")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pcb() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/pcb-sch/test/kicad-bom/layout.kicad_pcb");
        let pcb = KicadPcb::parse(&path).unwrap();

        assert_eq!(pcb.version, 20241229);
        assert!((pcb.thickness - 1.6).abs() < 0.01);

        // Should have layers
        assert!(!pcb.layers.is_empty());
        assert!(pcb.layers.iter().any(|l| l.name == "F.Cu"));

        // Should have nets
        assert!(!pcb.nets.is_empty());

        // Should have 3 footprints
        assert_eq!(pcb.footprints.len(), 3);

        // Check R1 footprint
        let r1 = pcb
            .footprints
            .iter()
            .find(|f| f.reference == "R1")
            .unwrap();
        assert_eq!(r1.footprint, "Resistor_SMD:R_0402_1005Metric");
        assert!(r1.is_smd());
        assert!(!r1.is_dnp());
        assert!(!r1.is_exclude_from_bom());
        assert_eq!(r1.pads.len(), 2);

        // Check R2 has DNP
        let r2 = pcb
            .footprints
            .iter()
            .find(|f| f.reference == "R2")
            .unwrap();
        assert!(r2.is_dnp());

        // Check R3 exclude_from_bom
        let r3 = pcb
            .footprints
            .iter()
            .find(|f| f.reference == "R3")
            .unwrap();
        assert!(r3.is_exclude_from_bom());

        // Check pad net assignments
        let r1_pad1 = r1.pads.iter().find(|p| p.number == "1").unwrap();
        assert!(!r1_pad1.net_name.is_empty());
    }
}
