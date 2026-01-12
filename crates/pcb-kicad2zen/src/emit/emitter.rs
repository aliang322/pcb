//! Unified Zener emitter - uses stdlib generics for all supported components

use crate::mapping::{
    extract_package, infer_component_type_combined, infer_net_type, map_symbol, normalize_value,
    ComponentType, FootprintComponentType, NetType,
};
use crate::parser::{Footprint, KicadPcb, KicadSchematic, SchematicSymbol};
use crate::KicadProject;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use super::OutputMode;

/// Emit Zener code from a KiCad project
///
/// Uses stdlib generics (Resistor, Capacitor, Led, etc.) for known component types.
/// Falls back to raw Component() for unknown types.
pub fn emit_zen(project: &KicadProject, _mode: OutputMode) -> String {
    let mut out = String::new();

    // Header
    emit_header(&mut out, &project.name);

    // Collect nets and modules
    let nets = collect_nets(project);
    let used_modules = collect_used_modules(project);

    // Emit imports and declarations
    emit_imports(&mut out, &nets);
    emit_module_aliases(&mut out, &used_modules);
    emit_net_declarations(&mut out, &nets);

    // Emit components - prefer schematic, fall back to PCB
    if let Some(schematic) = &project.schematic {
        emit_components_from_schematic(&mut out, schematic, project.pcb.as_ref());
    } else if let Some(pcb) = &project.pcb {
        emit_components_from_pcb(&mut out, pcb);
    }

    // Board configuration
    emit_board(&mut out, &project.name);

    out
}

// =============================================================================
// Header and imports
// =============================================================================

fn emit_header(out: &mut String, name: &str) {
    writeln!(out, "# Auto-generated from KiCad project: {}", name).unwrap();
    writeln!(out, "# Import with: pcb import kicad <path>").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "# ```pcb").unwrap();
    writeln!(out, "# [workspace]").unwrap();
    writeln!(out, "# pcb-version = \"0.3\"").unwrap();
    writeln!(out, "# ```").unwrap();
    writeln!(out).unwrap();
}

fn emit_imports(out: &mut String, nets: &HashMap<String, NetType>) {
    writeln!(out, "load(\"@stdlib/board_config.zen\", \"Board\")").unwrap();

    let has_power = nets.values().any(|t| matches!(t, NetType::Power));
    let has_ground = nets.values().any(|t| matches!(t, NetType::Ground));

    if has_power || has_ground {
        let mut imports = Vec::new();
        if has_power {
            imports.push("Power");
        }
        if has_ground {
            imports.push("Ground");
        }
        writeln!(
            out,
            "load(\"@stdlib/interfaces.zen\", {})",
            imports.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ")
        ).unwrap();
    }

    writeln!(out).unwrap();
}

fn emit_module_aliases(out: &mut String, modules: &HashSet<String>) {
    if modules.is_empty() {
        return;
    }

    let mut sorted: Vec<_> = modules.iter().collect();
    sorted.sort();

    for path in sorted {
        let name = path.rsplit('/').next().unwrap_or("").trim_end_matches(".zen");
        writeln!(out, "{} = Module(\"{}\")", name, path).unwrap();
    }

    writeln!(out).unwrap();
}

fn emit_net_declarations(out: &mut String, nets: &HashMap<String, NetType>) {
    if nets.is_empty() {
        return;
    }

    writeln!(out, "# Nets").unwrap();

    let mut names: Vec<_> = nets.keys().collect();
    names.sort();

    for name in names {
        let var = sanitize_name(name);
        match &nets[name] {
            NetType::Power => writeln!(out, "{} = Power(\"{}\")", var, name).unwrap(),
            NetType::Ground => writeln!(out, "{} = Ground(\"{}\")", var, name).unwrap(),
            _ => writeln!(out, "{} = Net(\"{}\")", var, name).unwrap(),
        }
    }

    writeln!(out).unwrap();
}

fn emit_board(out: &mut String, name: &str) {
    writeln!(out, "# Board configuration").unwrap();
    writeln!(out, "Board(").unwrap();
    writeln!(out, "    name = \"{}-imported\",", name).unwrap();
    writeln!(out, "    layers = 4,").unwrap();
    writeln!(out, "    layout_path = \"layout/{}-imported\"", name).unwrap();
    writeln!(out, ")").unwrap();
}

// =============================================================================
// Data collection
// =============================================================================

fn collect_nets(project: &KicadProject) -> HashMap<String, NetType> {
    let mut nets = HashMap::new();

    if let Some(pcb) = &project.pcb {
        for (_, name) in &pcb.nets {
            if !name.is_empty() && !name.starts_with("unconnected-") {
                nets.insert(name.clone(), infer_net_type(name));
            }
        }
    }

    nets
}

fn collect_used_modules(project: &KicadProject) -> HashSet<String> {
    let mut modules = HashSet::new();

    if let Some(schematic) = &project.schematic {
        for symbol in &schematic.symbols {
            if let Some(info) = map_symbol(&symbol.lib_id) {
                modules.insert(info.module_path.to_string());
            }
        }
    } else if let Some(pcb) = &project.pcb {
        for fp in &pcb.footprints {
            let comp_type = infer_component_type_combined(&fp.footprint, &fp.reference);
            if let Some(path) = comp_type.module_path() {
                modules.insert(path.to_string());
            }
        }
    }

    modules
}

// =============================================================================
// Component emission from schematic
// =============================================================================

fn emit_components_from_schematic(out: &mut String, schematic: &KicadSchematic, pcb: Option<&KicadPcb>) {
    writeln!(out, "# Components").unwrap();

    // Map schematic UUIDs to PCB footprints for net info
    let uuid_to_fp: HashMap<String, &Footprint> = pcb
        .map(|p| {
            p.footprints
                .iter()
                .map(|fp| (fp.path.trim_start_matches('/').to_string(), fp))
                .collect()
        })
        .unwrap_or_default();

    for symbol in &schematic.symbols {
        if let Some(info) = map_symbol(&symbol.lib_id) {
            // Known component type - use stdlib generic
            emit_generic_component(out, &symbol.reference, info.module_name, |out| {
                emit_value_and_package(out, &symbol.value, &symbol.footprint, &symbol.lib_id);
                for (key, value) in info.flags {
                    writeln!(out, "    {} = {},", key, value).unwrap();
                }
                emit_pin_connections(out, uuid_to_fp.get(&symbol.uuid), Some(info.pin_map));
            });
        } else {
            // Unknown - emit as comment (user needs to handle manually)
            emit_unknown_component(out, symbol, uuid_to_fp.get(&symbol.uuid));
        }
    }
}

fn emit_generic_component<F>(out: &mut String, name: &str, module: &str, emit_body: F)
where
    F: FnOnce(&mut String),
{
    writeln!(out, "{}(", module).unwrap();
    writeln!(out, "    name = \"{}\",", name).unwrap();
    emit_body(out);
    writeln!(out, ")").unwrap();
    writeln!(out).unwrap();
}

fn emit_value_and_package(out: &mut String, value: &str, footprint: &str, lib_id: &str) {
    let comp_type = ComponentType::from_lib_id(lib_id);
    
    if !value.is_empty() && comp_type != ComponentType::Other && !looks_like_mpn(value) {
        let normalized = normalize_value(value, comp_type);
        writeln!(out, "    value = \"{}\",", normalized).unwrap();
    }

    if let Some(pkg) = extract_package(footprint) {
        writeln!(out, "    package = \"{}\",", pkg).unwrap();
    }
}

fn emit_pin_connections(out: &mut String, footprint: Option<&&Footprint>, pin_map: Option<&[(&str, &str)]>) {
    let fp = match footprint {
        Some(fp) => fp,
        None => return,
    };

    let map: HashMap<&str, &str> = pin_map
        .map(|m| m.iter().copied().collect())
        .unwrap_or_default();

    for pad in &fp.pads {
        if pad.net_name.is_empty() || pad.net_name.starts_with("unconnected-") {
            continue;
        }

        let pin = map.get(pad.number.as_str()).copied().unwrap_or(pad.number.as_str());
        let net = sanitize_name(&pad.net_name);
        writeln!(out, "    {} = {},", pin, net).unwrap();
    }
}

fn emit_unknown_component(out: &mut String, symbol: &SchematicSymbol, footprint: Option<&&Footprint>) {
    writeln!(out, "# TODO: Unknown component type '{}' - manual conversion needed", symbol.lib_id).unwrap();
    writeln!(out, "# Reference: {}, Value: {}, Footprint: {}", 
        symbol.reference, symbol.value, symbol.footprint).unwrap();
    
    if let Some(fp) = footprint {
        let nets: Vec<_> = fp.pads.iter()
            .filter(|p| !p.net_name.is_empty() && !p.net_name.starts_with("unconnected-"))
            .map(|p| format!("{}->{}", p.number, p.net_name))
            .collect();
        if !nets.is_empty() {
            writeln!(out, "# Pins: {}", nets.join(", ")).unwrap();
        }
    }
    writeln!(out).unwrap();
}

// =============================================================================
// Component emission from PCB (when no schematic)
// =============================================================================

fn emit_components_from_pcb(out: &mut String, pcb: &KicadPcb) {
    writeln!(out, "# Components (from PCB footprints)").unwrap();

    for fp in &pcb.footprints {
        let comp_type = infer_component_type_combined(&fp.footprint, &fp.reference);

        if let Some(module_name) = comp_type.module_name() {
            emit_pcb_generic_component(out, fp, module_name, &comp_type);
        } else {
            emit_pcb_unknown_component(out, fp);
        }
    }
}

fn emit_pcb_generic_component(out: &mut String, fp: &Footprint, module: &str, comp_type: &FootprintComponentType) {
    writeln!(out, "{}(", module).unwrap();
    writeln!(out, "    name = \"{}\",", fp.reference).unwrap();

    if let Some(pkg) = extract_package(&fp.footprint) {
        writeln!(out, "    package = \"{}\",", pkg).unwrap();
    }

    if !fp.value.is_empty() && !looks_like_mpn(&fp.value) {
        writeln!(out, "    value = \"{}\",", fp.value).unwrap();
    }

    // Type-specific required params
    if matches!(comp_type, FootprintComponentType::Led) {
        writeln!(out, "    color = \"red\",").unwrap();
    }

    // Pin connections
    let pin_map: HashMap<&str, &str> = comp_type.pin_map().iter().copied().collect();
    for pad in &fp.pads {
        if pad.net_name.is_empty() || pad.net_name.starts_with("unconnected-") {
            continue;
        }
        let pin = pin_map.get(pad.number.as_str()).copied().unwrap_or(pad.number.as_str());
        let net = sanitize_name(&pad.net_name);
        writeln!(out, "    {} = {},", pin, net).unwrap();
    }

    writeln!(out, ")").unwrap();
    writeln!(out).unwrap();
}

fn emit_pcb_unknown_component(out: &mut String, fp: &Footprint) {
    writeln!(out, "# TODO: Unknown footprint '{}' - manual conversion needed", fp.footprint).unwrap();
    writeln!(out, "# Reference: {}, Value: {}", fp.reference, fp.value).unwrap();
    
    let nets: Vec<_> = fp.pads.iter()
        .filter(|p| !p.net_name.is_empty() && !p.net_name.starts_with("unconnected-"))
        .map(|p| format!("{}->{}", p.number, p.net_name))
        .collect();
    if !nets.is_empty() {
        writeln!(out, "# Pins: {}", nets.join(", ")).unwrap();
    }
    writeln!(out).unwrap();
}

// =============================================================================
// Utilities
// =============================================================================

/// Sanitize a name to be a valid Zener identifier
fn sanitize_name(name: &str) -> String {
    let mut result = String::new();

    for (i, ch) in name.chars().enumerate() {
        if ch.is_alphanumeric() || ch == '_' {
            if i == 0 && ch.is_numeric() {
                result.push('_');
            }
            result.push(ch);
        } else {
            result.push('_');
        }
    }

    // Handle reserved words
    if matches!(
        result.to_lowercase().as_str(),
        "and" | "or" | "not" | "if" | "else" | "for" | "in" | "true" | "false" | "none"
    ) {
        result.push('_');
    }

    if result.is_empty() {
        "net".to_string()
    } else {
        result
    }
}

/// Check if a value looks like an MPN rather than a component value
fn looks_like_mpn(value: &str) -> bool {
    (value.contains('-') && value.len() > 4)
        || (value.len() > 8 && value.chars().filter(|c| c.is_alphabetic()).count() > 3)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("VCC"), "VCC");
        assert_eq!(sanitize_name("GND"), "GND");
        assert_eq!(sanitize_name("+3V3"), "_3V3");
        assert_eq!(sanitize_name("USB_D+"), "USB_D_");
    }

    #[test]
    fn test_looks_like_mpn() {
        assert!(looks_like_mpn("ERJ-2RKF1003X"));
        assert!(looks_like_mpn("GRM155R71C104KA88D"));
        assert!(!looks_like_mpn("10k"));
        assert!(!looks_like_mpn("100nF"));
    }
}
