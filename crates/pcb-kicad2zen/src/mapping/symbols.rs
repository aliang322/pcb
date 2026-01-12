//! Symbol library ID → stdlib generic mapping

use std::collections::HashMap;
use std::sync::LazyLock;

/// Information about a stdlib generic module
#[derive(Debug, Clone)]
pub struct GenericInfo {
    /// Module path (e.g., "@stdlib/generics/Resistor.zen")
    pub module_path: &'static str,
    /// Module name for instantiation (e.g., "Resistor")
    pub module_name: &'static str,
    /// Pin mapping from KiCad pin numbers to Zener pin names
    pub pin_map: &'static [(&'static str, &'static str)],
    /// Additional config flags (e.g., "polarized" for capacitors)
    pub flags: &'static [(&'static str, &'static str)],
}

/// Static mapping from KiCad lib_id patterns to stdlib generics
static SYMBOL_MAP: LazyLock<Vec<(SymbolPattern, GenericInfo)>> = LazyLock::new(|| {
    vec![
        // Resistors
        (
            SymbolPattern::Exact("Device:R"),
            GenericInfo {
                module_path: "@stdlib/generics/Resistor.zen",
                module_name: "Resistor",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:R_Small"),
            GenericInfo {
                module_path: "@stdlib/generics/Resistor.zen",
                module_name: "Resistor",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        // Capacitors
        (
            SymbolPattern::Exact("Device:C"),
            GenericInfo {
                module_path: "@stdlib/generics/Capacitor.zen",
                module_name: "Capacitor",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:C_Small"),
            GenericInfo {
                module_path: "@stdlib/generics/Capacitor.zen",
                module_name: "Capacitor",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:C_Polarized"),
            GenericInfo {
                module_path: "@stdlib/generics/Capacitor.zen",
                module_name: "Capacitor",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[("polarized", "true")],
            },
        ),
        (
            SymbolPattern::Exact("Device:C_Polarized_Small"),
            GenericInfo {
                module_path: "@stdlib/generics/Capacitor.zen",
                module_name: "Capacitor",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[("polarized", "true")],
            },
        ),
        // Inductors
        (
            SymbolPattern::Exact("Device:L"),
            GenericInfo {
                module_path: "@stdlib/generics/Inductor.zen",
                module_name: "Inductor",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:L_Small"),
            GenericInfo {
                module_path: "@stdlib/generics/Inductor.zen",
                module_name: "Inductor",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        // Diodes
        (
            SymbolPattern::Exact("Device:D"),
            GenericInfo {
                module_path: "@stdlib/generics/Diode.zen",
                module_name: "Diode",
                pin_map: &[("1", "K"), ("2", "A")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:D_Small"),
            GenericInfo {
                module_path: "@stdlib/generics/Diode.zen",
                module_name: "Diode",
                pin_map: &[("1", "K"), ("2", "A")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:D_Zener"),
            GenericInfo {
                module_path: "@stdlib/generics/Diode.zen",
                module_name: "Diode",
                pin_map: &[("1", "K"), ("2", "A")],
                flags: &[("diode_type", "\"zener\"")],
            },
        ),
        (
            SymbolPattern::Exact("Device:D_Schottky"),
            GenericInfo {
                module_path: "@stdlib/generics/Diode.zen",
                module_name: "Diode",
                pin_map: &[("1", "K"), ("2", "A")],
                flags: &[("diode_type", "\"schottky\"")],
            },
        ),
        // LEDs
        (
            SymbolPattern::Exact("Device:LED"),
            GenericInfo {
                module_path: "@stdlib/generics/Led.zen",
                module_name: "Led",
                pin_map: &[("1", "K"), ("2", "A")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:LED_Small"),
            GenericInfo {
                module_path: "@stdlib/generics/Led.zen",
                module_name: "Led",
                pin_map: &[("1", "K"), ("2", "A")],
                flags: &[],
            },
        ),
        // Ferrite beads
        (
            SymbolPattern::Exact("Device:Ferrite_Bead"),
            GenericInfo {
                module_path: "@stdlib/generics/FerriteBead.zen",
                module_name: "FerriteBead",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:Ferrite_Bead_Small"),
            GenericInfo {
                module_path: "@stdlib/generics/FerriteBead.zen",
                module_name: "FerriteBead",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        // Crystals
        (
            SymbolPattern::Exact("Device:Crystal"),
            GenericInfo {
                module_path: "@stdlib/generics/Crystal.zen",
                module_name: "Crystal",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:Crystal_Small"),
            GenericInfo {
                module_path: "@stdlib/generics/Crystal.zen",
                module_name: "Crystal",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        (
            SymbolPattern::Exact("Device:Crystal_GND24"),
            GenericInfo {
                module_path: "@stdlib/generics/Crystal.zen",
                module_name: "Crystal",
                pin_map: &[("1", "P1"), ("3", "P2"), ("2", "GND"), ("4", "GND")],
                flags: &[],
            },
        ),
        // Thermistors
        (
            SymbolPattern::Prefix("Device:Thermistor"),
            GenericInfo {
                module_path: "@stdlib/generics/Thermistor.zen",
                module_name: "Thermistor",
                pin_map: &[("1", "P1"), ("2", "P2")],
                flags: &[],
            },
        ),
        // BJTs
        (
            SymbolPattern::Prefix("Device:Q_NPN"),
            GenericInfo {
                module_path: "@stdlib/generics/Bjt.zen",
                module_name: "Bjt",
                pin_map: &[("1", "B"), ("2", "C"), ("3", "E")],
                flags: &[("polarity", "\"NPN\"")],
            },
        ),
        (
            SymbolPattern::Prefix("Device:Q_PNP"),
            GenericInfo {
                module_path: "@stdlib/generics/Bjt.zen",
                module_name: "Bjt",
                pin_map: &[("1", "B"), ("2", "C"), ("3", "E")],
                flags: &[("polarity", "\"PNP\"")],
            },
        ),
        // MOSFETs
        (
            SymbolPattern::Prefix("Device:Q_NMOS"),
            GenericInfo {
                module_path: "@stdlib/generics/Mosfet.zen",
                module_name: "Mosfet",
                pin_map: &[("1", "G"), ("2", "D"), ("3", "S")],
                flags: &[("channel", "\"N\"")],
            },
        ),
        (
            SymbolPattern::Prefix("Device:Q_PMOS"),
            GenericInfo {
                module_path: "@stdlib/generics/Mosfet.zen",
                module_name: "Mosfet",
                pin_map: &[("1", "G"), ("2", "D"), ("3", "S")],
                flags: &[("channel", "\"P\"")],
            },
        ),
        // Test points
        (
            SymbolPattern::Prefix("Connector:TestPoint"),
            GenericInfo {
                module_path: "@stdlib/generics/TestPoint.zen",
                module_name: "TestPoint",
                pin_map: &[("1", "P1")],
                flags: &[],
            },
        ),
    ]
});

#[derive(Debug, Clone)]
enum SymbolPattern {
    /// Exact match
    Exact(&'static str),
    /// Prefix match (for wildcards like Device:Q_NPN_*)
    Prefix(&'static str),
}

impl SymbolPattern {
    fn matches(&self, lib_id: &str) -> bool {
        match self {
            SymbolPattern::Exact(s) => lib_id == *s,
            SymbolPattern::Prefix(s) => lib_id.starts_with(s),
        }
    }
}

/// Map a KiCad lib_id to its stdlib generic equivalent
///
/// Returns `None` if no mapping exists (component should use raw `Component()`)
pub fn map_symbol(lib_id: &str) -> Option<&'static GenericInfo> {
    for (pattern, info) in SYMBOL_MAP.iter() {
        if pattern.matches(lib_id) {
            return Some(info);
        }
    }
    None
}

/// Get the KiCad pin number → Zener pin name mapping for a symbol
#[allow(dead_code)]
pub fn get_pin_map(lib_id: &str) -> HashMap<&'static str, &'static str> {
    if let Some(info) = map_symbol(lib_id) {
        info.pin_map.iter().copied().collect()
    } else {
        HashMap::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_resistor() {
        let info = map_symbol("Device:R").unwrap();
        assert_eq!(info.module_name, "Resistor");
        assert_eq!(info.module_path, "@stdlib/generics/Resistor.zen");
        assert!(info.flags.is_empty());
    }

    #[test]
    fn test_map_capacitor_polarized() {
        let info = map_symbol("Device:C_Polarized").unwrap();
        assert_eq!(info.module_name, "Capacitor");
        assert_eq!(info.flags, &[("polarized", "true")]);
    }

    #[test]
    fn test_map_bjt_npn() {
        let info = map_symbol("Device:Q_NPN_BCE").unwrap();
        assert_eq!(info.module_name, "Bjt");
        assert_eq!(info.flags, &[("polarity", "\"NPN\"")]);
    }

    #[test]
    fn test_map_mosfet_nmos() {
        let info = map_symbol("Device:Q_NMOS_GDS").unwrap();
        assert_eq!(info.module_name, "Mosfet");
        assert_eq!(info.flags, &[("channel", "\"N\"")]);
    }

    #[test]
    fn test_map_unknown() {
        assert!(map_symbol("SomeManufacturer:CustomPart").is_none());
    }

    #[test]
    fn test_get_pin_map() {
        let pin_map = get_pin_map("Device:R");
        assert_eq!(pin_map.get("1"), Some(&"P1"));
        assert_eq!(pin_map.get("2"), Some(&"P2"));
    }
}
