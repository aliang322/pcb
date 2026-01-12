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
pub fn is_smd_footprint(footprint: &str) -> bool {
    let lower = footprint.to_lowercase();
    lower.contains("smd") || lower.contains("metric") || lower.contains("_smd")
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
}
