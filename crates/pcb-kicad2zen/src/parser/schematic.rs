//! Parser for `.kicad_sch` schematic files

use anyhow::{Context, Result};
use pcb_sexpr::Sexpr;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parsed KiCad schematic
#[derive(Debug, Default)]
pub struct KicadSchematic {
    /// KiCad version number
    pub version: u32,
    /// Schematic UUID
    pub uuid: String,
    /// Library symbols defined in this schematic
    pub lib_symbols: HashMap<String, LibSymbol>,
    /// Symbol instances placed in the schematic
    pub symbols: Vec<SchematicSymbol>,
}

/// A library symbol definition
#[derive(Debug, Clone)]
pub struct LibSymbol {
    /// Symbol name (e.g., "Device:R")
    pub name: String,
    /// Properties defined on the symbol
    pub properties: HashMap<String, String>,
    /// Pin definitions
    pub pins: Vec<LibPin>,
}

/// A pin in a library symbol
#[derive(Debug, Clone)]
pub struct LibPin {
    /// Pin number (e.g., "1", "2")
    pub number: String,
    /// Pin name
    pub name: String,
    /// Pin type (e.g., "passive", "input", "output")
    pub pin_type: String,
}

/// A symbol instance in the schematic
#[derive(Debug, Clone)]
pub struct SchematicSymbol {
    /// Symbol UUID
    pub uuid: String,
    /// Library ID reference (e.g., "Device:R")
    pub lib_id: String,
    /// Position (x, y, rotation)
    pub at: (f64, f64, f64),
    /// Reference designator (e.g., "R1")
    pub reference: String,
    /// Value (e.g., "10k", "ERJ-2RKF1003X")
    pub value: String,
    /// Footprint (e.g., "Resistor_SMD:R_0402_1005Metric")
    pub footprint: String,
    /// Do not populate flag
    pub dnp: bool,
    /// Exclude from BOM flag
    pub exclude_from_bom: bool,
    /// Exclude from board flag
    pub exclude_from_board: bool,
    /// Pin UUIDs mapped by pin number
    pub pins: HashMap<String, String>,
    /// All properties
    pub properties: HashMap<String, String>,
}

impl KicadSchematic {
    /// Parse a `.kicad_sch` file
    pub fn parse(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read schematic file: {}", path.display()))?;

        Self::parse_str(&content)
    }

    /// Parse schematic from string content
    pub fn parse_str(content: &str) -> Result<Self> {
        let sexpr = pcb_sexpr::parse(content).context("Failed to parse schematic S-expression")?;

        let list = sexpr.as_list().context("Expected root list")?;
        if list.first().and_then(|s| s.as_sym()) != Some("kicad_sch") {
            anyhow::bail!("Expected kicad_sch root element");
        }

        let mut schematic = KicadSchematic::default();

        for item in list.iter().skip(1) {
            if let Some(items) = item.as_list() {
                match items.first().and_then(|s| s.as_sym()) {
                    Some("version") => {
                        if let Some(v) = items.get(1) {
                            schematic.version = v.as_int().unwrap_or(0) as u32;
                        }
                    }
                    Some("uuid") => {
                        if let Some(v) = items.get(1) {
                            schematic.uuid = get_string_value(v);
                        }
                    }
                    Some("lib_symbols") => {
                        schematic.lib_symbols = parse_lib_symbols(items)?;
                    }
                    Some("symbol") => {
                        if let Ok(sym) = parse_schematic_symbol(items) {
                            schematic.symbols.push(sym);
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(schematic)
    }
}

fn parse_lib_symbols(list: &[Sexpr]) -> Result<HashMap<String, LibSymbol>> {
    let mut symbols = HashMap::new();

    for item in list.iter().skip(1) {
        if let Some(items) = item.as_list() {
            if items.first().and_then(|s| s.as_sym()) == Some("symbol") {
                if let Some(name) = items.get(1).and_then(|s| s.as_str().or_else(|| s.as_sym())) {
                    let mut lib_symbol = LibSymbol {
                        name: name.to_string(),
                        properties: HashMap::new(),
                        pins: Vec::new(),
                    };

                    // Parse properties and pins
                    for sub in items.iter().skip(2) {
                        if let Some(sub_items) = sub.as_list() {
                            match sub_items.first().and_then(|s| s.as_sym()) {
                                Some("property") => {
                                    if let (Some(key), Some(val)) = (
                                        sub_items.get(1).and_then(|s| s.as_str()),
                                        sub_items.get(2),
                                    ) {
                                        lib_symbol
                                            .properties
                                            .insert(key.to_string(), get_string_value(val));
                                    }
                                }
                                Some("symbol") => {
                                    // Nested symbol units contain pins
                                    parse_pins_from_symbol(sub_items, &mut lib_symbol.pins);
                                }
                                _ => {}
                            }
                        }
                    }

                    symbols.insert(name.to_string(), lib_symbol);
                }
            }
        }
    }

    Ok(symbols)
}

fn parse_pins_from_symbol(list: &[Sexpr], pins: &mut Vec<LibPin>) {
    for item in list.iter() {
        if let Some(items) = item.as_list() {
            if items.first().and_then(|s| s.as_sym()) == Some("pin") {
                let pin_type = items.get(1).and_then(|s| s.as_sym()).unwrap_or("passive");
                let mut pin = LibPin {
                    number: String::new(),
                    name: String::new(),
                    pin_type: pin_type.to_string(),
                };

                for sub in items.iter().skip(2) {
                    if let Some(sub_items) = sub.as_list() {
                        match sub_items.first().and_then(|s| s.as_sym()) {
                            Some("name") => {
                                if let Some(n) = sub_items.get(1) {
                                    pin.name = get_string_value(n);
                                }
                            }
                            Some("number") => {
                                if let Some(n) = sub_items.get(1) {
                                    pin.number = get_string_value(n);
                                }
                            }
                            _ => {}
                        }
                    }
                }

                if !pin.number.is_empty() {
                    pins.push(pin);
                }
            }
        }
    }
}

fn parse_schematic_symbol(list: &[Sexpr]) -> Result<SchematicSymbol> {
    let mut symbol = SchematicSymbol {
        uuid: String::new(),
        lib_id: String::new(),
        at: (0.0, 0.0, 0.0),
        reference: String::new(),
        value: String::new(),
        footprint: String::new(),
        dnp: false,
        exclude_from_bom: false,
        exclude_from_board: false,
        pins: HashMap::new(),
        properties: HashMap::new(),
    };

    for item in list.iter().skip(1) {
        if let Some(items) = item.as_list() {
            match items.first().and_then(|s| s.as_sym()) {
                Some("lib_id") => {
                    if let Some(v) = items.get(1) {
                        symbol.lib_id = get_string_value(v);
                    }
                }
                Some("uuid") => {
                    if let Some(v) = items.get(1) {
                        symbol.uuid = get_string_value(v);
                    }
                }
                Some("at") => {
                    symbol.at = (
                        items.get(1).and_then(|s| s.as_float()).unwrap_or(0.0),
                        items.get(2).and_then(|s| s.as_float()).unwrap_or(0.0),
                        items.get(3).and_then(|s| s.as_float()).unwrap_or(0.0),
                    );
                }
                Some("dnp") => {
                    symbol.dnp = items
                        .get(1)
                        .and_then(|s| s.as_sym())
                        .map(|s| s == "yes")
                        .unwrap_or(false);
                }
                Some("in_bom") => {
                    symbol.exclude_from_bom = items
                        .get(1)
                        .and_then(|s| s.as_sym())
                        .map(|s| s == "no")
                        .unwrap_or(false);
                }
                Some("on_board") => {
                    symbol.exclude_from_board = items
                        .get(1)
                        .and_then(|s| s.as_sym())
                        .map(|s| s == "no")
                        .unwrap_or(false);
                }
                Some("property") => {
                    if let (Some(key), Some(val)) =
                        (items.get(1).and_then(|s| s.as_str()), items.get(2))
                    {
                        let value = get_string_value(val);
                        symbol.properties.insert(key.to_string(), value.clone());

                        match key {
                            "Reference" => symbol.reference = value,
                            "Value" => symbol.value = value,
                            "Footprint" => symbol.footprint = value,
                            _ => {}
                        }
                    }
                }
                Some("pin") => {
                    // (pin "1" (uuid "..."))
                    if let Some(pin_num) = items.get(1).and_then(|s| s.as_str()) {
                        if let Some(uuid_list) = items.get(2).and_then(|s| s.as_list()) {
                            if uuid_list.first().and_then(|s| s.as_sym()) == Some("uuid") {
                                if let Some(uuid) = uuid_list.get(1) {
                                    symbol
                                        .pins
                                        .insert(pin_num.to_string(), get_string_value(uuid));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(symbol)
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
    fn test_parse_schematic() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../crates/pcb-sch/test/kicad-bom/layout.kicad_sch");
        let schematic = KicadSchematic::parse(&path).unwrap();

        assert_eq!(schematic.version, 20250114);
        assert!(!schematic.uuid.is_empty());

        // Should have Device:R lib symbol
        assert!(schematic.lib_symbols.contains_key("Device:R"));

        // Should have 3 resistor symbols
        assert_eq!(schematic.symbols.len(), 3);

        // Check first symbol
        let r1 = schematic
            .symbols
            .iter()
            .find(|s| s.reference == "R1")
            .unwrap();
        assert_eq!(r1.lib_id, "Device:R");
        assert_eq!(r1.value, "ERJ-2RKF1003X");
        assert_eq!(r1.footprint, "Resistor_SMD:R_0402_1005Metric");
        assert!(!r1.dnp);
        assert!(!r1.exclude_from_bom);

        // Check DNP symbol
        let r2 = schematic
            .symbols
            .iter()
            .find(|s| s.reference == "R2")
            .unwrap();
        assert!(r2.dnp);

        // Check exclude_from_bom symbol
        let r3 = schematic
            .symbols
            .iter()
            .find(|s| s.reference == "R3")
            .unwrap();
        assert!(r3.exclude_from_bom);
    }
}
