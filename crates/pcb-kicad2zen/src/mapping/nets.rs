//! Net name → Zener net type inference

use regex::Regex;
use std::sync::LazyLock;

/// Inferred net type for Zener output
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetType {
    /// Power net (VCC, VDD, +3V3, etc.)
    Power,
    /// Ground net (GND, VSS, DGND, etc.)
    Ground,
    /// Differential pair positive signal
    DiffPairP,
    /// Differential pair negative signal
    DiffPairN,
    /// Generic signal net
    Signal,
}

/// Patterns for power net detection
static POWER_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)^V(CC|DD|BAT|IN|OUT|BUS)").unwrap(),
        Regex::new(r"(?i)^\+\d+V").unwrap(),         // +3V3, +5V, +12V
        Regex::new(r"(?i)^\d+V\d*").unwrap(),        // 3V3, 5V, 12V
        Regex::new(r"(?i)^PWR").unwrap(),            // PWR, PWR_3V3
        Regex::new(r"(?i)_PWR$").unwrap(),           // SENSOR_PWR
        Regex::new(r"(?i)^VREF").unwrap(),           // VREF
        Regex::new(r"(?i)^AVDD").unwrap(),           // Analog VDD
        Regex::new(r"(?i)^DVDD").unwrap(),           // Digital VDD
    ]
});

/// Patterns for ground net detection
static GROUND_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)^GND").unwrap(),            // GND, GND1, GND_ANALOG
        Regex::new(r"(?i)^V(SS|EE)").unwrap(),       // VSS, VEE
        Regex::new(r"(?i)^DGND").unwrap(),           // Digital ground
        Regex::new(r"(?i)^AGND").unwrap(),           // Analog ground
        Regex::new(r"(?i)^PGND").unwrap(),           // Power ground
        Regex::new(r"(?i)^SGND").unwrap(),           // Signal ground
        Regex::new(r"(?i)_GND$").unwrap(),           // SENSOR_GND
        Regex::new(r"(?i)^0V$").unwrap(),            // 0V
    ]
});

/// Patterns for differential pair detection
static DIFFPAIR_P_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)[_\-]P$").unwrap(),         // USB_D_P, ETH_TX_P
        Regex::new(r"(?i)[_\-]\+$").unwrap(),        // USB_D+
        Regex::new(r"(?i)_DP$").unwrap(),            // USB_DP
        Regex::new(r"(?i)_POS$").unwrap(),           // CLK_POS
    ]
});

static DIFFPAIR_N_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)[_\-]N$").unwrap(),         // USB_D_N, ETH_TX_N
        Regex::new(r"(?i)[_\-]\-$").unwrap(),        // USB_D-
        Regex::new(r"(?i)_DN$").unwrap(),            // USB_DN
        Regex::new(r"(?i)_NEG$").unwrap(),           // CLK_NEG
    ]
});

/// Infer the Zener net type from a KiCad net name
///
/// Examples:
/// - "VCC" → Power
/// - "+3V3" → Power
/// - "GND" → Ground
/// - "AGND" → Ground
/// - "USB_D_P" → DiffPairP
/// - "USB_D_N" → DiffPairN
/// - "SPI_CLK" → Signal
pub fn infer_net_type(name: &str) -> NetType {
    // Skip empty or unconnected nets
    if name.is_empty() || name.starts_with("unconnected-") {
        return NetType::Signal;
    }

    // Check ground patterns FIRST (VSS/VEE would otherwise match power V* pattern)
    for pattern in GROUND_PATTERNS.iter() {
        if pattern.is_match(name) {
            return NetType::Ground;
        }
    }

    // Check power patterns
    for pattern in POWER_PATTERNS.iter() {
        if pattern.is_match(name) {
            return NetType::Power;
        }
    }

    // Check differential pair patterns
    for pattern in DIFFPAIR_P_PATTERNS.iter() {
        if pattern.is_match(name) {
            return NetType::DiffPairP;
        }
    }

    for pattern in DIFFPAIR_N_PATTERNS.iter() {
        if pattern.is_match(name) {
            return NetType::DiffPairN;
        }
    }

    NetType::Signal
}

/// Get the Zener type constructor for a net type
pub fn net_type_constructor(net_type: &NetType) -> &'static str {
    match net_type {
        NetType::Power => "Power",
        NetType::Ground => "Ground",
        NetType::DiffPairP | NetType::DiffPairN => "Net", // DiffPair handled separately
        NetType::Signal => "Net",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_nets() {
        assert_eq!(infer_net_type("VCC"), NetType::Power);
        assert_eq!(infer_net_type("VDD"), NetType::Power);
        assert_eq!(infer_net_type("VBAT"), NetType::Power);
        assert_eq!(infer_net_type("+3V3"), NetType::Power);
        assert_eq!(infer_net_type("+5V"), NetType::Power);
        assert_eq!(infer_net_type("+12V"), NetType::Power);
        assert_eq!(infer_net_type("3V3"), NetType::Power);
        assert_eq!(infer_net_type("5V"), NetType::Power);
        assert_eq!(infer_net_type("AVDD"), NetType::Power);
        assert_eq!(infer_net_type("DVDD"), NetType::Power);
        assert_eq!(infer_net_type("SENSOR_PWR"), NetType::Power);
    }

    #[test]
    fn test_ground_nets() {
        assert_eq!(infer_net_type("GND"), NetType::Ground);
        assert_eq!(infer_net_type("VSS"), NetType::Ground);
        assert_eq!(infer_net_type("VEE"), NetType::Ground);
        assert_eq!(infer_net_type("DGND"), NetType::Ground);
        assert_eq!(infer_net_type("AGND"), NetType::Ground);
        assert_eq!(infer_net_type("PGND"), NetType::Ground);
        assert_eq!(infer_net_type("SENSOR_GND"), NetType::Ground);
        assert_eq!(infer_net_type("0V"), NetType::Ground);
    }

    #[test]
    fn test_diffpair_nets() {
        assert_eq!(infer_net_type("USB_D_P"), NetType::DiffPairP);
        assert_eq!(infer_net_type("USB_D_N"), NetType::DiffPairN);
        assert_eq!(infer_net_type("ETH_TX_P"), NetType::DiffPairP);
        assert_eq!(infer_net_type("ETH_TX_N"), NetType::DiffPairN);
        assert_eq!(infer_net_type("USB_DP"), NetType::DiffPairP);
        assert_eq!(infer_net_type("USB_DN"), NetType::DiffPairN);
    }

    #[test]
    fn test_signal_nets() {
        assert_eq!(infer_net_type("SPI_CLK"), NetType::Signal);
        assert_eq!(infer_net_type("I2C_SDA"), NetType::Signal);
        assert_eq!(infer_net_type("RESET"), NetType::Signal);
        assert_eq!(infer_net_type("LED_OUT"), NetType::Signal);
    }

    #[test]
    fn test_unconnected_nets() {
        assert_eq!(infer_net_type("unconnected-(R1-Pad1)"), NetType::Signal);
        assert_eq!(infer_net_type(""), NetType::Signal);
    }
}
