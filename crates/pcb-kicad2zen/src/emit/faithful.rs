//! Faithful mode emitter - preserves exact KiCad data for round-trip fidelity

use crate::mapping::{infer_net_type, NetType};
use crate::parser::SchematicSymbol;
use crate::KicadProject;
use std::collections::HashMap;
use std::fmt::Write;

/// Emit Zener code in faithful mode
///
/// This mode preserves exact KiCad symbol/footprint strings to enable
/// round-trip conversion (KiCad → Zen → KiCad) with minimal data loss.
/// The output is valid, buildable Zener code.
pub fn emit_faithful(project: &KicadProject) -> String {
    let mut out = String::new();

    // Header
    writeln!(out, "# Auto-generated from KiCad project: {}", project.name).unwrap();
    writeln!(out, "# Mode: faithful (preserves exact KiCad data)").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "# ```pcb").unwrap();
    writeln!(out, "# [workspace]").unwrap();
    writeln!(out, "# pcb-version = \"0.3\"").unwrap();
    writeln!(out, "# ```").unwrap();
    writeln!(out).unwrap();

    // Collect all nets from PCB (authoritative source for connectivity)
    let nets = collect_nets(project);

    // Emit imports
    emit_imports(&mut out, &nets);

    // Emit net declarations
    emit_net_declarations(&mut out, &nets);

    // Emit components
    emit_components_faithful(&mut out, project, &nets);

    // Emit Board() call
    emit_board(&mut out, project);

    out
}

/// Collect all unique nets from PCB footprints
fn collect_nets(project: &KicadProject) -> HashMap<String, NetType> {
    let mut nets = HashMap::new();

    if let Some(pcb) = &project.pcb {
        for (_, name) in &pcb.nets {
            if !name.is_empty() && !name.starts_with("unconnected-") {
                let net_type = infer_net_type(name);
                nets.insert(name.clone(), net_type);
            }
        }
    }

    nets
}

/// Emit load() statements for required imports
fn emit_imports(out: &mut String, nets: &HashMap<String, NetType>) {
    // Always need Board
    writeln!(out, "load(\"@stdlib/board_config.zen\", \"Board\")").unwrap();

    // Check if we need Power/Ground imports
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
            imports
                .iter()
                .map(|s| format!("\"{}\"", s))
                .collect::<Vec<_>>()
                .join(", ")
        )
        .unwrap();
    }

    writeln!(out).unwrap();
}

/// Emit net declarations
fn emit_net_declarations(out: &mut String, nets: &HashMap<String, NetType>) {
    if nets.is_empty() {
        return;
    }

    writeln!(out, "# Nets").unwrap();

    // Sort nets for deterministic output
    let mut net_names: Vec<_> = nets.keys().collect();
    net_names.sort();

    for name in net_names {
        let net_type = &nets[name];
        let var_name = sanitize_net_name(name);

        match net_type {
            NetType::Power => {
                writeln!(out, "{} = Power(\"{}\")", var_name, name).unwrap();
            }
            NetType::Ground => {
                writeln!(out, "{} = Ground(\"{}\")", var_name, name).unwrap();
            }
            _ => {
                writeln!(out, "{} = Net(\"{}\")", var_name, name).unwrap();
            }
        }
    }

    writeln!(out).unwrap();
}

/// Emit components in faithful mode using raw Component()
fn emit_components_faithful(
    out: &mut String,
    project: &KicadProject,
    _nets: &HashMap<String, NetType>,
) {
    let schematic = match &project.schematic {
        Some(s) => s,
        None => return,
    };

    let pcb = project.pcb.as_ref();

    writeln!(out, "# Components").unwrap();

    // Build a map from schematic UUID path to PCB footprint for net lookups
    let mut uuid_to_footprint: HashMap<String, &crate::parser::Footprint> = HashMap::new();
    if let Some(pcb) = pcb {
        for fp in &pcb.footprints {
            // PCB path is like "/uuid" - strip leading slash
            let uuid = fp.path.trim_start_matches('/');
            uuid_to_footprint.insert(uuid.to_string(), fp);
        }
    }

    for symbol in &schematic.symbols {
        emit_component_faithful(out, symbol, &uuid_to_footprint);
    }
}

/// Emit a single component in faithful mode
fn emit_component_faithful(
    out: &mut String,
    symbol: &SchematicSymbol,
    uuid_to_footprint: &HashMap<String, &crate::parser::Footprint>,
) {
    // Get the corresponding PCB footprint for net assignments
    let footprint = uuid_to_footprint.get(&symbol.uuid);

    writeln!(out, "Component(").unwrap();
    writeln!(out, "    name = \"{}\",", symbol.reference).unwrap();

    // Symbol - use @kicad-symbols path format
    let (lib, sym_name) = split_lib_id(&symbol.lib_id);
    let lib_path = format!("@kicad-symbols/{}.kicad_sym", lib);
    writeln!(
        out,
        "    symbol = Symbol(library = \"{}\", name = \"{}\"),",
        lib_path, sym_name
    )
    .unwrap();

    // Footprint - use KiCad library:name format directly
    if !symbol.footprint.is_empty() {
        writeln!(out, "    footprint = \"{}\",", symbol.footprint).unwrap();
    }

    // Pin connections from PCB footprint
    if let Some(fp) = footprint {
        let pin_nets: Vec<_> = fp
            .pads
            .iter()
            .filter(|pad| !pad.net_name.is_empty() && !pad.net_name.starts_with("unconnected-"))
            .map(|pad| (pad.number.clone(), pad.net_name.clone()))
            .collect();

        if !pin_nets.is_empty() {
            writeln!(out, "    pins = {{").unwrap();
            for (pin_num, net_name) in &pin_nets {
                let var_name = sanitize_net_name(&net_name);
                writeln!(out, "        \"{}\": {},", pin_num, var_name).unwrap();
            }
            writeln!(out, "    }},").unwrap();
        }
    }

    // Properties dict for value, DNP, etc.
    let mut properties = Vec::new();
    if !symbol.value.is_empty() {
        properties.push(format!("\"Value\": \"{}\"", symbol.value));
    }

    // DNP and BOM flags as properties
    if symbol.dnp {
        properties.push("\"dnp\": True".to_string());
    }
    if symbol.exclude_from_bom {
        properties.push("\"exclude_from_bom\": True".to_string());
    }

    if !properties.is_empty() {
        writeln!(out, "    properties = {{").unwrap();
        for prop in &properties {
            writeln!(out, "        {},", prop).unwrap();
        }
        writeln!(out, "    }},").unwrap();
    }

    writeln!(out, ")").unwrap();
    writeln!(out).unwrap();
}

/// Emit Board() call at the end
fn emit_board(out: &mut String, project: &KicadProject) {
    writeln!(out, "# Board configuration").unwrap();
    writeln!(out, "Board(").unwrap();
    writeln!(out, "    name = \"{}\",", project.name).unwrap();
    writeln!(out, "    layers = 4,").unwrap();
    writeln!(out, "    layout_path = \"layout/{}\"", project.name).unwrap();
    writeln!(out, ")").unwrap();
}

/// Split a lib_id like "Device:R" into ("Device", "R")
fn split_lib_id(lib_id: &str) -> (&str, &str) {
    if let Some(idx) = lib_id.find(':') {
        (&lib_id[..idx], &lib_id[idx + 1..])
    } else {
        ("", lib_id)
    }
}

/// Sanitize a net name to be a valid Python/Zener identifier
fn sanitize_net_name(name: &str) -> String {
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
    let lower = result.to_lowercase();
    if matches!(
        lower.as_str(),
        "and" | "or" | "not" | "if" | "else" | "for" | "in" | "true" | "false" | "none"
    ) {
        result.push('_');
    }

    if result.is_empty() {
        result = "net".to_string();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_net_name() {
        assert_eq!(sanitize_net_name("VCC"), "VCC");
        assert_eq!(sanitize_net_name("GND"), "GND");
        assert_eq!(sanitize_net_name("+3V3"), "_3V3");
        assert_eq!(sanitize_net_name("USB_D+"), "USB_D_");
        assert_eq!(sanitize_net_name("Net-(R1-Pad1)"), "Net__R1_Pad1_");
    }

    #[test]
    fn test_split_lib_id() {
        assert_eq!(split_lib_id("Device:R"), ("Device", "R"));
        assert_eq!(split_lib_id("Device:C_Polarized"), ("Device", "C_Polarized"));
        assert_eq!(split_lib_id("NoColon"), ("", "NoColon"));
    }
}
