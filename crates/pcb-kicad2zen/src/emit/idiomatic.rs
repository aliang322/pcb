//! Idiomatic mode emitter - maps to stdlib generics for human-readable output

use crate::mapping::{extract_package, infer_net_type, map_symbol, normalize_value, ComponentType, GenericInfo, NetType};
use crate::parser::SchematicSymbol;
use crate::KicadProject;
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Emit Zener code in idiomatic mode
///
/// This mode uses the mapping engine to output stdlib generics like
/// `Resistor()`, `Capacitor()`, etc. with typed parameters.
pub fn emit_idiomatic(project: &KicadProject) -> String {
    let mut out = String::new();

    // Header
    writeln!(out, "# Auto-generated from KiCad project: {}", project.name).unwrap();
    writeln!(out, "# Mode: idiomatic (uses stdlib generics)").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "# ```pcb").unwrap();
    writeln!(out, "# [workspace]").unwrap();
    writeln!(out, "# pcb-version = \"0.3\"").unwrap();
    writeln!(out, "# ```").unwrap();
    writeln!(out).unwrap();

    // Collect all nets from PCB
    let nets = collect_nets(project);

    // Collect which generic modules are needed
    let used_modules = collect_used_modules(project);

    // Emit imports
    emit_imports(&mut out, &nets, &used_modules);

    // Emit module aliases
    emit_module_aliases(&mut out, &used_modules);

    // Emit net declarations
    emit_net_declarations(&mut out, &nets);

    // Emit components
    emit_components_idiomatic(&mut out, project, &nets);

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

/// Collect which stdlib generic modules are used
fn collect_used_modules(project: &KicadProject) -> HashSet<&'static str> {
    let mut modules = HashSet::new();

    if let Some(schematic) = &project.schematic {
        for symbol in &schematic.symbols {
            if let Some(info) = map_symbol(&symbol.lib_id) {
                modules.insert(info.module_path);
            }
        }
    }

    modules
}

/// Emit load statements and module aliases
fn emit_imports(out: &mut String, nets: &HashMap<String, NetType>, _used_modules: &HashSet<&'static str>) {
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
        writeln!(out, "load(\"@stdlib/interfaces.zen\", {})",
            imports.iter().map(|s| format!("\"{}\"", s)).collect::<Vec<_>>().join(", ")).unwrap();
    }

    writeln!(out).unwrap();
}

/// Emit Module() aliases for each used generic
fn emit_module_aliases(out: &mut String, used_modules: &HashSet<&'static str>) {
    if used_modules.is_empty() {
        return;
    }

    // Sort for deterministic output
    let mut modules: Vec<_> = used_modules.iter().collect();
    modules.sort();

    for path in modules {
        // Extract module name from path: @stdlib/generics/Resistor.zen -> Resistor
        let name = path
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".zen");
        writeln!(out, "{} = Module(\"{}\")", name, path).unwrap();
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

/// Emit components in idiomatic mode using stdlib generics
fn emit_components_idiomatic(out: &mut String, project: &KicadProject, nets: &HashMap<String, NetType>) {
    let schematic = match &project.schematic {
        Some(s) => s,
        None => return,
    };

    let pcb = project.pcb.as_ref();

    writeln!(out, "# Components").unwrap();

    // Build a map from schematic UUID to PCB footprint
    let mut uuid_to_footprint: HashMap<String, &crate::parser::Footprint> = HashMap::new();
    if let Some(pcb) = pcb {
        for fp in &pcb.footprints {
            let uuid = fp.path.trim_start_matches('/');
            uuid_to_footprint.insert(uuid.to_string(), fp);
        }
    }

    for symbol in &schematic.symbols {
        if let Some(info) = map_symbol(&symbol.lib_id) {
            emit_component_idiomatic(out, symbol, info, &uuid_to_footprint, nets);
        } else {
            // Fall back to Component() for unmapped symbols
            emit_component_fallback(out, symbol, &uuid_to_footprint, nets);
        }
    }
}

/// Emit a component using its stdlib generic
fn emit_component_idiomatic(
    out: &mut String,
    symbol: &SchematicSymbol,
    info: &GenericInfo,
    uuid_to_footprint: &HashMap<String, &crate::parser::Footprint>,
    _nets: &HashMap<String, NetType>,
) {
    let footprint = uuid_to_footprint.get(&symbol.uuid);

    writeln!(out, "{}(", info.module_name).unwrap();
    writeln!(out, "    name=\"{}\",", symbol.reference).unwrap();

    // Extract and emit value if applicable
    let comp_type = ComponentType::from_lib_id(&symbol.lib_id);
    if !symbol.value.is_empty() && comp_type != ComponentType::Other {
        let normalized = normalize_value(&symbol.value, comp_type);
        // Only emit if it looks like a value, not an MPN
        if normalized != symbol.value || !looks_like_mpn(&symbol.value) {
            writeln!(out, "    value=\"{}\",", normalized).unwrap();
        }
    }

    // Extract package from footprint
    if let Some(pkg) = extract_package(&symbol.footprint) {
        writeln!(out, "    package=\"{}\",", pkg).unwrap();
    }

    // Emit any flags from the mapping (e.g., polarized=true)
    for (key, value) in info.flags {
        writeln!(out, "    {}={},", key, value).unwrap();
    }

    // DNP and BOM flags
    if symbol.dnp {
        writeln!(out, "    dnp=True,").unwrap();
    }
    if symbol.exclude_from_bom {
        writeln!(out, "    skip_bom=True,").unwrap();
    }

    // Pin connections using mapped pin names
    if let Some(fp) = footprint {
        let pin_map: HashMap<&str, &str> = info.pin_map.iter().copied().collect();

        for pad in &fp.pads {
            if pad.net_name.is_empty() || pad.net_name.starts_with("unconnected-") {
                continue;
            }

            let pin_name = match pin_map.get(pad.number.as_str()) {
                Some(name) => *name,
                None => pad.number.as_str(),
            };
            let net_var = sanitize_net_name(&pad.net_name);
            writeln!(out, "    {}={},", pin_name, net_var).unwrap();
        }
    }

    writeln!(out, ")").unwrap();
    writeln!(out).unwrap();
}

/// Emit a component using raw Component() for unmapped symbols
fn emit_component_fallback(
    out: &mut String,
    symbol: &SchematicSymbol,
    uuid_to_footprint: &HashMap<String, &crate::parser::Footprint>,
    _nets: &HashMap<String, NetType>,
) {
    let footprint = uuid_to_footprint.get(&symbol.uuid);

    writeln!(out, "# Unmapped symbol: {}", symbol.lib_id).unwrap();
    writeln!(out, "Component(").unwrap();
    writeln!(out, "    name=\"{}\",", symbol.reference).unwrap();

    let (lib, sym_name) = split_lib_id(&symbol.lib_id);
    writeln!(out, "    symbol=Symbol(library=\"{}\", name=\"{}\"),", lib, sym_name).unwrap();

    if !symbol.footprint.is_empty() {
        writeln!(out, "    footprint=\"{}\",", symbol.footprint).unwrap();
    }

    if !symbol.value.is_empty() {
        writeln!(out, "    value=\"{}\",", symbol.value).unwrap();
    }

    if symbol.dnp {
        writeln!(out, "    dnp=True,").unwrap();
    }
    if symbol.exclude_from_bom {
        writeln!(out, "    skip_bom=True,").unwrap();
    }

    if let Some(fp) = footprint {
        let pin_nets: Vec<_> = fp.pads.iter()
            .filter(|pad| !pad.net_name.is_empty() && !pad.net_name.starts_with("unconnected-"))
            .map(|pad| (pad.number.clone(), pad.net_name.clone()))
            .collect();

        if !pin_nets.is_empty() {
            writeln!(out, "    pins={{").unwrap();
            for (pin_num, net_name) in &pin_nets {
                let var_name = sanitize_net_name(&net_name);
                writeln!(out, "        \"{}\": {},", pin_num, var_name).unwrap();
            }
            writeln!(out, "    }},").unwrap();
        }
    }

    writeln!(out, ")").unwrap();
    writeln!(out).unwrap();
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

    let lower = result.to_lowercase();
    if matches!(lower.as_str(), "and" | "or" | "not" | "if" | "else" | "for" | "in" | "true" | "false" | "none") {
        result.push('_');
    }

    if result.is_empty() {
        result = "net".to_string();
    }

    result
}

/// Check if a value looks like an MPN
fn looks_like_mpn(value: &str) -> bool {
    if value.contains('-') && value.len() > 4 {
        return true;
    }
    if value.len() > 8 && value.chars().filter(|c| c.is_alphabetic()).count() > 3 {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_mpn() {
        assert!(looks_like_mpn("ERJ-2RKF1003X"));
        assert!(looks_like_mpn("GRM155R71C104KA88D"));
        assert!(!looks_like_mpn("10k"));
        assert!(!looks_like_mpn("100nF"));
    }
}
