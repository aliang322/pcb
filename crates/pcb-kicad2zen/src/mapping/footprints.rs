//! Footprint name → package extraction

use regex::Regex;
use std::sync::LazyLock;

/// Regex patterns for extracting package size from footprint names
static PACKAGE_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        // Standard metric SMD: R_0402_1005Metric, C_0603_1608Metric, L_1206_3216Metric
        Regex::new(r"^[RCL]_(\d{4})_\d+Metric").unwrap(),
        // LED SMD: LED_0402_1005Metric, LED_0805_2012Metric
        Regex::new(r"^LED_(\d{4})_\d+Metric").unwrap(),
        // Inductor SMD: L_0402_1005Metric
        Regex::new(r"^L_(\d{4})_\d+Metric").unwrap(),
        // Ferrite bead (uses resistor footprints): R_0402_1005Metric
        // Already covered by first pattern
        // Crystal packages: Crystal_SMD_0603-2Pin, Crystal_SMD_3215-2Pin
        Regex::new(r"^Crystal_SMD_(\d{4})-\d+Pin").unwrap(),
        // Diode packages: D_0603, D_SOD-123
        Regex::new(r"^D_(\d{4})$").unwrap(),
        // SOD packages: SOD-123, SOD-323, SOD-523
        Regex::new(r"^D_(SOD-\d+)").unwrap(),
        // SOT packages: SOT-23, SOT-223, SOT-323
        Regex::new(r"^(SOT-\d+)").unwrap(),
        // QFN/DFN packages: QFN-16, DFN-8
        Regex::new(r"^(QFN-\d+|DFN-\d+)").unwrap(),
        // SOIC packages: SOIC-8, SOIC-16
        Regex::new(r"^(SOIC-\d+)").unwrap(),
        // TSSOP packages: TSSOP-8, TSSOP-16
        Regex::new(r"^(TSSOP-\d+)").unwrap(),
    ]
});

/// Extract the package size from a KiCad footprint name
///
/// Examples:
/// - `Resistor_SMD:R_0402_1005Metric` → `Some("0402")`
/// - `Capacitor_SMD:C_0603_1608Metric` → `Some("0603")`
/// - `LED_SMD:LED_0805_2012Metric` → `Some("0805")`
/// - `Package_SO:SOIC-8_3.9x4.9mm_P1.27mm` → `Some("SOIC-8")`
/// - `Custom:MyFootprint` → `None`
pub fn extract_package(footprint: &str) -> Option<String> {
    // Strip library prefix if present (e.g., "Resistor_SMD:")
    let name = footprint
        .split(':')
        .last()
        .unwrap_or(footprint);

    for pattern in PACKAGE_PATTERNS.iter() {
        if let Some(caps) = pattern.captures(name) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }

    None
}

/// Check if a footprint represents an SMD component
#[allow(dead_code)]
pub fn is_smd_footprint(footprint: &str) -> bool {
    let lower = footprint.to_lowercase();
    lower.contains("smd") || lower.contains("metric") || lower.contains("_smd")
}

/// Component type inferred from footprint name
#[derive(Debug, Clone, PartialEq)]
pub enum FootprintComponentType {
    Resistor,
    Capacitor,
    Inductor,
    Led,
    Diode,
    Transistor,
    Crystal,
    Connector,
    Unknown,
}

impl FootprintComponentType {
    /// Get the stdlib module path for this component type
    pub fn module_path(&self) -> Option<&'static str> {
        match self {
            FootprintComponentType::Resistor => Some("@stdlib/generics/Resistor.zen"),
            FootprintComponentType::Capacitor => Some("@stdlib/generics/Capacitor.zen"),
            FootprintComponentType::Inductor => Some("@stdlib/generics/Inductor.zen"),
            FootprintComponentType::Led => Some("@stdlib/generics/Led.zen"),
            FootprintComponentType::Diode => Some("@stdlib/generics/Diode.zen"),
            FootprintComponentType::Crystal => Some("@stdlib/generics/Crystal.zen"),
            _ => None,
        }
    }

    /// Get the module name (e.g., "Resistor")
    pub fn module_name(&self) -> Option<&'static str> {
        match self {
            FootprintComponentType::Resistor => Some("Resistor"),
            FootprintComponentType::Capacitor => Some("Capacitor"),
            FootprintComponentType::Inductor => Some("Inductor"),
            FootprintComponentType::Led => Some("Led"),
            FootprintComponentType::Diode => Some("Diode"),
            FootprintComponentType::Crystal => Some("Crystal"),
            _ => None,
        }
    }

    /// Get the pin map for this component type (KiCad pin → Zener pin)
    pub fn pin_map(&self) -> &[(&'static str, &'static str)] {
        match self {
            FootprintComponentType::Resistor => &[("1", "P1"), ("2", "P2")],
            FootprintComponentType::Capacitor => &[("1", "P1"), ("2", "P2")],
            FootprintComponentType::Inductor => &[("1", "P1"), ("2", "P2")],
            FootprintComponentType::Led => &[("1", "K"), ("2", "A")],
            FootprintComponentType::Diode => &[("1", "K"), ("2", "A")],
            FootprintComponentType::Crystal => &[("1", "P1"), ("2", "P2")],
            _ => &[],
        }
    }
}

/// Infer component type from footprint name
///
/// Examples:
/// - `R_0402_1005Metric` → Resistor
/// - `C_0603_1608Metric` → Capacitor  
/// - `LED_0402_1005Metric` → Led
/// - `L_0805_2012Metric` → Inductor
pub fn infer_component_type(footprint: &str) -> FootprintComponentType {
    // Strip library prefix if present
    let name = footprint.split(':').last().unwrap_or(footprint);
    let lower = name.to_lowercase();

    // Check by prefix patterns
    if name.starts_with("R_") || lower.contains("resistor") {
        FootprintComponentType::Resistor
    } else if name.starts_with("C_") || lower.contains("capacitor") {
        FootprintComponentType::Capacitor
    } else if name.starts_with("L_") || lower.contains("inductor") {
        FootprintComponentType::Inductor
    } else if name.starts_with("LED_") || lower.contains("led") {
        FootprintComponentType::Led
    } else if name.starts_with("D_") || lower.contains("diode") {
        FootprintComponentType::Diode
    } else if lower.contains("crystal") {
        FootprintComponentType::Crystal
    } else if lower.contains("conn") || lower.contains("pin_header") || lower.contains("pin_socket") {
        FootprintComponentType::Connector
    } else {
        FootprintComponentType::Unknown
    }
}

/// Infer component type from reference designator
///
/// Examples:
/// - `R1` → Resistor
/// - `C2` → Capacitor
/// - `D1` → Led or Diode (check footprint to disambiguate)
pub fn infer_type_from_reference(reference: &str) -> FootprintComponentType {
    let prefix = reference.chars().take_while(|c| c.is_alphabetic()).collect::<String>();
    
    match prefix.as_str() {
        "R" => FootprintComponentType::Resistor,
        "C" => FootprintComponentType::Capacitor,
        "L" => FootprintComponentType::Inductor,
        "D" => FootprintComponentType::Diode, // Could be LED - check footprint
        "Q" => FootprintComponentType::Transistor,
        "Y" | "X" => FootprintComponentType::Crystal,
        "J" | "P" => FootprintComponentType::Connector,
        _ => FootprintComponentType::Unknown,
    }
}

/// Infer component type using both footprint and reference
pub fn infer_component_type_combined(footprint: &str, reference: &str) -> FootprintComponentType {
    // First try footprint - more specific
    let from_footprint = infer_component_type(footprint);
    if from_footprint != FootprintComponentType::Unknown {
        return from_footprint;
    }

    // Fall back to reference designator
    infer_type_from_reference(reference)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_resistor_package() {
        assert_eq!(
            extract_package("Resistor_SMD:R_0402_1005Metric"),
            Some("0402".to_string())
        );
        assert_eq!(
            extract_package("R_0603_1608Metric"),
            Some("0603".to_string())
        );
        assert_eq!(
            extract_package("R_0805_2012Metric"),
            Some("0805".to_string())
        );
        assert_eq!(
            extract_package("R_1206_3216Metric"),
            Some("1206".to_string())
        );
    }

    #[test]
    fn test_extract_capacitor_package() {
        assert_eq!(
            extract_package("Capacitor_SMD:C_0402_1005Metric"),
            Some("0402".to_string())
        );
        assert_eq!(
            extract_package("C_0603_1608Metric"),
            Some("0603".to_string())
        );
    }

    #[test]
    fn test_extract_led_package() {
        assert_eq!(
            extract_package("LED_SMD:LED_0402_1005Metric"),
            Some("0402".to_string())
        );
        assert_eq!(
            extract_package("LED_0805_2012Metric"),
            Some("0805".to_string())
        );
    }

    #[test]
    fn test_extract_inductor_package() {
        assert_eq!(
            extract_package("Inductor_SMD:L_0603_1608Metric"),
            Some("0603".to_string())
        );
    }

    #[test]
    fn test_extract_crystal_package() {
        assert_eq!(
            extract_package("Crystal:Crystal_SMD_3215-2Pin_3.2x1.5mm"),
            Some("3215".to_string())
        );
    }

    #[test]
    fn test_extract_unknown() {
        assert_eq!(extract_package("Custom:MyCustomFootprint"), None);
        assert_eq!(extract_package("SomeRandomName"), None);
    }

    #[test]
    fn test_is_smd() {
        assert!(is_smd_footprint("Resistor_SMD:R_0402_1005Metric"));
        assert!(is_smd_footprint("R_0402_1005Metric"));
        assert!(!is_smd_footprint("Resistor_THT:R_Axial_DIN0207"));
    }

    #[test]
    fn test_infer_component_type_from_footprint() {
        assert_eq!(
            infer_component_type("R_0402_1005Metric"),
            FootprintComponentType::Resistor
        );
        assert_eq!(
            infer_component_type("Resistor_SMD:R_0603_1608Metric"),
            FootprintComponentType::Resistor
        );
        assert_eq!(
            infer_component_type("C_0402_1005Metric"),
            FootprintComponentType::Capacitor
        );
        assert_eq!(
            infer_component_type("LED_0402_1005Metric"),
            FootprintComponentType::Led
        );
        assert_eq!(
            infer_component_type("LED_SMD:LED_0805_2012Metric"),
            FootprintComponentType::Led
        );
        assert_eq!(
            infer_component_type("L_0603_1608Metric"),
            FootprintComponentType::Inductor
        );
        assert_eq!(
            infer_component_type("D_SOD-123"),
            FootprintComponentType::Diode
        );
    }

    #[test]
    fn test_infer_type_from_reference() {
        assert_eq!(
            infer_type_from_reference("R1"),
            FootprintComponentType::Resistor
        );
        assert_eq!(
            infer_type_from_reference("C10"),
            FootprintComponentType::Capacitor
        );
        assert_eq!(
            infer_type_from_reference("D1"),
            FootprintComponentType::Diode
        );
        assert_eq!(
            infer_type_from_reference("U5"),
            FootprintComponentType::Unknown
        );
    }

    #[test]
    fn test_infer_component_type_combined() {
        // Footprint takes precedence
        assert_eq!(
            infer_component_type_combined("LED_0402_1005Metric", "D1"),
            FootprintComponentType::Led
        );
        // Falls back to reference when footprint unknown
        assert_eq!(
            infer_component_type_combined("Custom:MyFootprint", "R1"),
            FootprintComponentType::Resistor
        );
    }

    #[test]
    fn test_footprint_component_type_module_path() {
        assert_eq!(
            FootprintComponentType::Resistor.module_path(),
            Some("@stdlib/generics/Resistor.zen")
        );
        assert_eq!(
            FootprintComponentType::Led.module_path(),
            Some("@stdlib/generics/Led.zen")
        );
        assert_eq!(FootprintComponentType::Unknown.module_path(), None);
    }
}
