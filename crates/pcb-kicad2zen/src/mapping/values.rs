//! Value string normalization for KiCad → Zener

use regex::Regex;
use std::sync::LazyLock;

/// Pattern for resistor shorthand values like "4k7", "10k", "1M"
static RESISTOR_SHORTHAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\d+)([kKmM])(\d*)(?:\s*[oO]hm)?s?$").unwrap()
});

/// Pattern for resistor values with explicit units like "10kOhm", "4.7kohm"
static RESISTOR_EXPLICIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\d+(?:\.\d+)?)\s*([kmgμu]?)\s*[oO]hm").unwrap()
});

/// Pattern for plain resistor values like "100", "4.7"
static RESISTOR_PLAIN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(\d+(?:\.\d+)?)$").unwrap()
});

/// Pattern for capacitor shorthand values like "100n", "10u", "1p"
static CAPACITOR_SHORTHAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\d+(?:\.\d+)?)\s*([pnuμm])[fF]?$").unwrap()
});

/// Pattern for capacitor values with explicit units like "100nF", "10uF"
static CAPACITOR_EXPLICIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\d+(?:\.\d+)?)\s*([pnuμm])[fF]").unwrap()
});

/// Pattern for inductor shorthand values like "100n", "10u", "1m"
static INDUCTOR_SHORTHAND: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(\d+(?:\.\d+)?)\s*([pnuμm])[hH]?$").unwrap()
});

/// Component type for context-aware value parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    Resistor,
    Capacitor,
    Inductor,
    Other,
}

impl ComponentType {
    /// Infer component type from lib_id
    pub fn from_lib_id(lib_id: &str) -> Self {
        let lower = lib_id.to_lowercase();
        // Use word boundaries to avoid false matches (e.g., "LED" matching ":L")
        if lower.contains(":r_") || lower.contains(":r ") || lower.ends_with(":r") || lower.contains("resistor") {
            ComponentType::Resistor
        } else if lower.contains(":c_") || lower.contains(":c ") || lower.ends_with(":c") || lower.contains("capacitor") {
            ComponentType::Capacitor
        } else if lower.contains(":l_") || lower.contains(":l ") || lower.ends_with(":l") || lower.contains("inductor") {
            ComponentType::Inductor
        } else {
            ComponentType::Other
        }
    }
}

/// Normalize a KiCad value string to Zener-compatible format
///
/// Examples:
/// - Resistors: "10k" → "10kohm", "4k7" → "4.7kohm", "1M" → "1Mohm"
/// - Capacitors: "100n" → "100nF", "10u" → "10uF", "1p" → "1pF"
/// - Inductors: "10u" → "10uH", "100n" → "100nH"
///
/// If the value cannot be parsed, it is returned as-is.
pub fn normalize_value(value: &str, component_type: ComponentType) -> String {
    let value = value.trim();

    // Skip if it looks like an MPN (has letters in unusual positions)
    if looks_like_mpn(value) {
        return value.to_string();
    }

    match component_type {
        ComponentType::Resistor => normalize_resistor_value(value),
        ComponentType::Capacitor => normalize_capacitor_value(value),
        ComponentType::Inductor => normalize_inductor_value(value),
        ComponentType::Other => value.to_string(),
    }
}

fn normalize_resistor_value(value: &str) -> String {
    // Handle shorthand like "4k7", "10k", "1M2"
    if let Some(caps) = RESISTOR_SHORTHAND.captures(value) {
        let whole = caps.get(1).map(|m| m.as_str()).unwrap_or("0");
        let multiplier = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let decimal = caps.get(3).map(|m| m.as_str()).unwrap_or("");

        let prefix = match multiplier.to_lowercase().as_str() {
            "k" => "k",
            "m" => "M",
            _ => "",
        };

        if decimal.is_empty() {
            return format!("{whole}{prefix}ohm");
        } else {
            return format!("{whole}.{decimal}{prefix}ohm");
        }
    }

    // Handle explicit units like "10kohm", "4.7Mohm"
    if let Some(caps) = RESISTOR_EXPLICIT.captures(value) {
        let num = caps.get(1).map(|m| m.as_str()).unwrap_or("0");
        let prefix = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let prefix_normalized = match prefix.to_lowercase().as_str() {
            "k" => "k",
            "m" => "M",
            "g" => "G",
            "μ" | "u" => "u",
            _ => "",
        };
        return format!("{num}{prefix_normalized}ohm");
    }

    // Handle plain numbers (assume ohms)
    if RESISTOR_PLAIN.is_match(value) {
        return format!("{value}ohm");
    }

    value.to_string()
}

fn normalize_capacitor_value(value: &str) -> String {
    // Handle shorthand like "100n", "10u"
    if let Some(caps) = CAPACITOR_SHORTHAND.captures(value) {
        let num = caps.get(1).map(|m| m.as_str()).unwrap_or("0");
        let prefix = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let prefix_normalized = match prefix.to_lowercase().as_str() {
            "p" => "p",
            "n" => "n",
            "u" | "μ" => "u",
            "m" => "m",
            _ => "",
        };
        return format!("{num}{prefix_normalized}F");
    }

    // Handle explicit units like "100nF", "10uF"
    if let Some(caps) = CAPACITOR_EXPLICIT.captures(value) {
        let num = caps.get(1).map(|m| m.as_str()).unwrap_or("0");
        let prefix = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let prefix_normalized = match prefix.to_lowercase().as_str() {
            "p" => "p",
            "n" => "n",
            "u" | "μ" => "u",
            "m" => "m",
            _ => "",
        };
        return format!("{num}{prefix_normalized}F");
    }

    value.to_string()
}

fn normalize_inductor_value(value: &str) -> String {
    // Handle shorthand like "100n", "10u"
    if let Some(caps) = INDUCTOR_SHORTHAND.captures(value) {
        let num = caps.get(1).map(|m| m.as_str()).unwrap_or("0");
        let prefix = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let prefix_normalized = match prefix.to_lowercase().as_str() {
            "p" => "p",
            "n" => "n",
            "u" | "μ" => "u",
            "m" => "m",
            _ => "",
        };
        return format!("{num}{prefix_normalized}H");
    }

    value.to_string()
}

/// Check if a value looks like an MPN rather than a component value
fn looks_like_mpn(value: &str) -> bool {
    // MPNs typically have:
    // - Mix of letters and numbers in unusual patterns
    // - Dashes
    // - More than 6 characters with mixed case
    if value.contains('-') && value.len() > 4 {
        return true;
    }
    // Check for typical MPN patterns like "ERJ-2RKF1003X"
    if value.len() > 8 && value.chars().filter(|c| c.is_alphabetic()).count() > 3 {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resistor_shorthand() {
        assert_eq!(
            normalize_value("10k", ComponentType::Resistor),
            "10kohm"
        );
        assert_eq!(
            normalize_value("4k7", ComponentType::Resistor),
            "4.7kohm"
        );
        assert_eq!(
            normalize_value("1M", ComponentType::Resistor),
            "1Mohm"
        );
        assert_eq!(
            normalize_value("2M2", ComponentType::Resistor),
            "2.2Mohm"
        );
        assert_eq!(
            normalize_value("100", ComponentType::Resistor),
            "100ohm"
        );
    }

    #[test]
    fn test_resistor_explicit() {
        assert_eq!(
            normalize_value("10kohm", ComponentType::Resistor),
            "10kohm"
        );
        assert_eq!(
            normalize_value("4.7kOhm", ComponentType::Resistor),
            "4.7kohm"
        );
        assert_eq!(
            normalize_value("1 Mohm", ComponentType::Resistor),
            "1Mohm"
        );
    }

    #[test]
    fn test_capacitor_shorthand() {
        assert_eq!(
            normalize_value("100n", ComponentType::Capacitor),
            "100nF"
        );
        assert_eq!(
            normalize_value("10u", ComponentType::Capacitor),
            "10uF"
        );
        assert_eq!(
            normalize_value("1p", ComponentType::Capacitor),
            "1pF"
        );
        assert_eq!(
            normalize_value("4.7u", ComponentType::Capacitor),
            "4.7uF"
        );
    }

    #[test]
    fn test_capacitor_explicit() {
        assert_eq!(
            normalize_value("100nF", ComponentType::Capacitor),
            "100nF"
        );
        assert_eq!(
            normalize_value("10uF", ComponentType::Capacitor),
            "10uF"
        );
    }

    #[test]
    fn test_inductor_shorthand() {
        assert_eq!(
            normalize_value("10u", ComponentType::Inductor),
            "10uH"
        );
        assert_eq!(
            normalize_value("100n", ComponentType::Inductor),
            "100nH"
        );
    }

    #[test]
    fn test_mpn_passthrough() {
        // MPNs should pass through unchanged
        assert_eq!(
            normalize_value("ERJ-2RKF1003X", ComponentType::Resistor),
            "ERJ-2RKF1003X"
        );
        assert_eq!(
            normalize_value("GRM155R71C104KA88D", ComponentType::Capacitor),
            "GRM155R71C104KA88D"
        );
    }

    #[test]
    fn test_component_type_inference() {
        assert_eq!(ComponentType::from_lib_id("Device:R"), ComponentType::Resistor);
        assert_eq!(ComponentType::from_lib_id("Device:C"), ComponentType::Capacitor);
        assert_eq!(ComponentType::from_lib_id("Device:L"), ComponentType::Inductor);
        assert_eq!(ComponentType::from_lib_id("Device:LED"), ComponentType::Other);
    }
}
