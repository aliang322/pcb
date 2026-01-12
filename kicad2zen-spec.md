# kicad2zen: KiCad to Zener Importer

## Motivation

Create `.zen` references/training data from existing KiCad designs and enable users to convert designs they already like back to Zener, e.g. for revision or reference for new desighs. This supports

1. Migration path for existing KiCad projects to Zener file conversion
2. Forward compilation back to KiCad formats via `pcb build` to validate round-trip fidelity

## Existing Work

### Available Infrastructure (pcb repo)

| Crate | Functionality | Reuse |
|-------|--------------|-------|
| `pcb-sexpr` | S-expression parser | Direct reuse for parsing |
| `pcb-eda` | KiCad symbol parsing (`KicadSymbol`, `KicadPin`) | Partial reuse |
| `pcb-sch` | `Schematic`, `Instance`, `Net` data model | Target output format |
| `kicad_schematic.rs` | `.zen` → `.kicad_sch` (forward only) | Reference for field mapping |
| `kicad_netlist.rs` | Netlist export | Reference for net handling |

### Available Infrastructure (stdlib)

| File | Contains | Reuse |
|------|----------|-------|
| `generics/*.zen` | Package→footprint forward maps | Invert for reverse mapping |
| `bom/match_generics.zen` | Component type detection | N/A (forward direction) |
| `interfaces.zen` | `Power`, `Ground`, `DiffPair` net types | Reference for net type inference |

### Requirements

**Reverse mapping.** The stdlib contains forward maps like:
```python
# stdlib/generics/Resistor.zen
Package("0402"): "@kicad-footprints/Resistor_SMD.pretty/R_0402_1005Metric.kicad_mod"
```

We need the inverse:
```
R_0402_1005Metric → package="0402", type="resistor"
```

## Data Extraction

### From `.kicad_sch`

```
(symbol
  (lib_id "Device:R")                    → type inference
  (property "Reference" "R1")            → name
  (property "Value" "10k")               → value (parse to typed)
  (property "Footprint" "R_0402_...")    → package extraction
  (dnp yes)                              → dnp flag
  (in_bom no)                            → skip_bom flag
  (pin "1" (uuid "..."))                 → pin connections
)
```

### From `.kicad_pcb`

```
(footprint "Resistor_SMD:R_0402_1005Metric"
  (at 123.99 98.02 90)                   → placement, rotation
  (path "/uuid")                         → cross-ref to schematic
  (pad "1" ... (net 1 "VCC"))           → net assignments
  (attr smd dnp exclude_from_bom)        → flags
)
(net 1 "VCC")                            → net names
(layers ...)                             → board stackup
(gr_line ... (layer "Edge.Cuts"))        → board outline
```

### From `.kicad_pro`

```json
{
  "net_settings": {
    "classes": [{"name": "Default", "track_width": 0.2, ...}]
  },
  "board": {
    "design_settings": {"rules": {"min_clearance": 0.2}}
  }
}
```

## Mapping Tables

### Symbol Library → Generic Module

| KiCad lib_id | Zener Generic |
|--------------|---------------|
| `Device:R` | `@stdlib/generics/Resistor.zen` |
| `Device:C` | `@stdlib/generics/Capacitor.zen` |
| `Device:C_Polarized` | `@stdlib/generics/Capacitor.zen` (polarized=true) |
| `Device:L` | `@stdlib/generics/Inductor.zen` |
| `Device:D` | `@stdlib/generics/Diode.zen` |
| `Device:LED` | `@stdlib/generics/Led.zen` |
| `Device:Q_NPN_*` | `@stdlib/generics/Bjt.zen` |
| `Device:Q_PNP_*` | `@stdlib/generics/Bjt.zen` |
| `Device:Q_NMOS_*` | `@stdlib/generics/Mosfet.zen` |
| `Device:Q_PMOS_*` | `@stdlib/generics/Mosfet.zen` |
| `Device:Ferrite_Bead` | `@stdlib/generics/FerriteBead.zen` |
| `Device:Crystal` | `@stdlib/generics/Crystal.zen` |
| `Device:Thermistor*` | `@stdlib/generics/Thermistor.zen` |
| `Connector:TestPoint` | `@stdlib/generics/TestPoint.zen` |
| Other | `Component()` with raw symbol |

### Footprint → Package

Extract package from footprint name via regex:
```
R_0402_1005Metric     → "0402"
C_0603_1608Metric     → "0603"
LED_0805_2012Metric   → "0805"
L_1206_3216Metric     → "1206"
```

Pattern: `[RCL]_(\d{4})_\d+Metric` or `LED_(\d{4})_\d+Metric`

### Net Name → Net Type

| Pattern | Zener Type |
|---------|------------|
| `VCC`, `VDD`, `+*V`, `*_PWR` | `Power(name)` |
| `GND`, `VSS`, `DGND`, `AGND` | `Ground(name)` |
| `*_P`, `*_N` pairs | `DiffPair` candidate |
| Other | `Net(name)` |

### Value Parsing

| Input | Output |
|-------|--------|
| `10k`, `10K` | `"10kohm"` |
| `4k7`, `4K7` | `"4.7kohm"` |
| `100n`, `100nF` | `"100nF"` |
| `10u`, `10uF` | `"10uF"` |
| `1M` (resistor context) | `"1Mohm"` |

## Output Modes

### Mode 1: Faithful (Default)

Preserves exact KiCad data for round-trip fidelity:

```python
Component(
    name = "R1",
    symbol = Symbol(library="Device.kicad_sym", name="R"),
    footprint = "Resistor_SMD:R_0402_1005Metric",
    mpn = "ERJ-2RKF1003X",
    pins = {"1": net_vcc, "2": net_out},
    dnp = False,
    skip_bom = False,
)
```

### Mode 2: Idiomatic (--idiomatic flag)

Maps to stdlib generics:

```python
Resistor = Module("@stdlib/generics/Resistor.zen")
Resistor(
    name = "R1",
    value = "100kohm",
    package = "0402",
    mpn = "ERJ-2RKF1003X",
    P1 = vcc,
    P2 = out,
)
```

## Implementation

### Phase 1: Core Parser

Create `pcb-kicad2zen` crate:

```rust
pub struct KicadProject {
    schematic: KicadSchematic,
    pcb: Option<KicadPcb>,
    project: Option<KicadPro>,
}

pub struct KicadSchematic {
    symbols: Vec<SchematicSymbol>,
    wires: Vec<Wire>,
    labels: Vec<Label>,
    lib_symbols: HashMap<String, LibSymbol>,
}

pub struct KicadPcb {
    footprints: Vec<Footprint>,
    nets: Vec<Net>,
    layers: Vec<Layer>,
    board_outline: Option<Polygon>,
}

impl KicadProject {
    pub fn parse(dir: &Path) -> Result<Self>;
    pub fn to_zen(&self, mode: OutputMode) -> String;
}
```

### Phase 2: Mapping Engine

```rust
pub struct MappingEngine {
    symbol_map: HashMap<String, GenericInfo>,
    footprint_regex: Vec<(Regex, String)>,  // pattern → package
    net_patterns: Vec<(Regex, NetType)>,
}

impl MappingEngine {
    pub fn infer_generic(&self, lib_id: &str) -> Option<&GenericInfo>;
    pub fn extract_package(&self, footprint: &str) -> Option<String>;
    pub fn infer_net_type(&self, name: &str) -> NetType;
    pub fn parse_value(&self, value: &str, comp_type: &str) -> String;
}
```

### Phase 3: Zen Emitter

```rust
pub fn emit_zen(project: &KicadProject, mode: OutputMode) -> String {
    let mut out = String::new();
    
    // Header
    writeln!(out, "# Auto-generated from {}", project.name);
    writeln!(out, "# pcb-version = \"0.3\"");
    
    // Imports
    emit_imports(&mut out, &project.used_generics);
    
    // Nets
    emit_nets(&mut out, &project.nets);
    
    // Components
    emit_components(&mut out, &project.components, mode);
    
    // Board config (if PCB present)
    if let Some(pcb) = &project.pcb {
        emit_board_config(&mut out, pcb);
    }
    
    out
}
```

### Phase 4: CLI Integration

```bash
# Convert single project
pcb import kicad ./project/

# Convert with idiomatic output
pcb import kicad ./project/ --idiomatic

# Batch convert for dataset
pcb import kicad ./projects/**/ --output ./zen-dataset/
```

## Validation

Round-trip test:
```bash
pcb import kicad ./original/
pcb build ./imported.zen
diff ./original/layout.kicad_pcb ./imported/layout/layout.kicad_pcb
```

Expected deltas (acceptable):
- UUID regeneration
- Whitespace/formatting
- Property ordering

Errors (not acceptable):
- Missing components
- Wrong net connections
- Different footprints
- Missing DNP/BOM flags

## Files

```
pcb/crates/pcb-kicad2zen/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── parser/
│   │   ├── mod.rs
│   │   ├── schematic.rs    # .kicad_sch parsing
│   │   ├── pcb.rs          # .kicad_pcb parsing
│   │   └── project.rs      # .kicad_pro parsing
│   ├── mapping/
│   │   ├── mod.rs
│   │   ├── symbols.rs      # lib_id → generic
│   │   ├── footprints.rs   # footprint → package
│   │   ├── nets.rs         # net type inference
│   │   └── values.rs       # value parsing
│   └── emit/
│       ├── mod.rs
│       ├── faithful.rs     # Mode 1 output
│       └── idiomatic.rs    # Mode 2 output
```

## Dependencies

```toml
[dependencies]
pcb-sexpr = { path = "../pcb-sexpr" }
pcb-sch = { path = "../pcb-sch" }
regex = "1"
anyhow = "1"
```

## Checklist

### 1. Crate scaffold
- [x] **Create `pcb-kicad2zen` crate with Cargo.toml and lib.rs stub.** Establishes the new crate in the workspace so subsequent commits can add functionality incrementally.

### 2. KiCad file parsers
- [x] **Parse `.kicad_sch`, `.kicad_pcb`, `.kicad_pro` into structs.** Extracts symbols/properties/wires from schematic, footprint placements/pad-nets from PCB, and net classes/design rules from project JSON. Uses `pcb-sexpr` for S-expressions and `serde_json` for project file.

### 3. Mapping engine
- [x] **Symbol/footprint/value/net mapping utilities.** Translates KiCad conventions to Zener equivalents so idiomatic mode can emit stdlib generics with correct parameters. Creates `src/mapping/` module with:
  - `lib_id` → stdlib generic table (`Device:R` → `Resistor.zen`)
  - Footprint → package regex (`R_0402_1005Metric` → `"0402"`)
  - Value normalization (`10k` → `10kohm`, `4k7` → `4.7kohm`)
  - Net type inference (`VCC`/`GND` → `Power`/`Ground`)

### 4. Zen emitters
- [ ] **Emit `.zen` in faithful and idiomatic modes.** Transforms parsed `KicadProject` into valid Zener source code. Faithful mode preserves exact KiCad symbol/footprint strings for round-trip fidelity; idiomatic mode uses mapping engine to output `Resistor()`, `Capacitor()`, etc. Creates `src/emit/` module.

### 5. CLI integration
- [ ] **Add `pcb import kicad` subcommand.** Entry point for users to run the importer. Wires parser and emitter into `pcb` binary with `--idiomatic` and `--output` flags; adds to `pcb/src/main.rs` command dispatch.

### 6. Round-trip tests
- [ ] **Add integration tests for round-trip validation.** Verifies the importer produces correct output by importing test KiCad projects, running `pcb build` on the result, and diffing against original. Catches regressions in component/net/footprint handling.

## Changelog

### 2025-01-11: Crate scaffold (checklist #1)

**Files created:**
- `crates/pcb-kicad2zen/Cargo.toml` - Crate manifest with `pcb-sexpr`, `anyhow`, `log` deps
- `crates/pcb-kicad2zen/src/lib.rs` - `OutputMode` enum, `KicadProject` struct with stub methods
- `crates/pcb-kicad2zen/src/parser/mod.rs` - Module re-exports
- `crates/pcb-kicad2zen/src/parser/schematic.rs` - Stub `KicadSchematic`
- `crates/pcb-kicad2zen/src/parser/pcb.rs` - Stub `KicadPcb`
- `crates/pcb-kicad2zen/src/parser/project.rs` - Stub `KicadPro`

**Files modified:**
- `Cargo.toml` (workspace) - Added `pcb-kicad2zen` to workspace dependencies

### 2025-01-11: KiCad file parsers (checklist #2)

**Files modified:**
- `crates/pcb-kicad2zen/Cargo.toml` - Added `serde`, `serde_json` dependencies
- `crates/pcb-kicad2zen/src/lib.rs` - Added `KicadProject::parse()` implementation, re-exports parser types
- `crates/pcb-kicad2zen/src/parser/mod.rs` - Re-exports all parser types

**Files rewritten:**
- `crates/pcb-kicad2zen/src/parser/schematic.rs` - Full `.kicad_sch` parser
  - `KicadSchematic`: version, uuid, lib_symbols, symbols
  - `LibSymbol`: name, properties, pins
  - `SchematicSymbol`: uuid, lib_id, at, reference, value, footprint, dnp, exclude_from_bom, pins
- `crates/pcb-kicad2zen/src/parser/pcb.rs` - Full `.kicad_pcb` parser
  - `KicadPcb`: version, thickness, layers, nets, footprints
  - `Layer`: number, name, layer_type
  - `Footprint`: uuid, footprint, layer, at, path, reference, value, attrs, pads
  - `Pad`: number, pad_type, shape, at, net_id, net_name
- `crates/pcb-kicad2zen/src/parser/project.rs` - Full `.kicad_pro` JSON parser
  - `KicadPro`: net_classes, design_rules
  - `NetClass`: name, track_width, clearance, via_diameter, via_drill, diff_pair_width/gap
  - `DesignRules`: min_clearance, min_track_width, min_via_diameter, etc.

**Tests added:** 3 (test_parse_schematic, test_parse_pcb, test_parse_project)

### 2025-01-11: Mapping engine (checklist #3)

**Files created:**
- `crates/pcb-kicad2zen/src/mapping/mod.rs` - Module exports
- `crates/pcb-kicad2zen/src/mapping/symbols.rs` - `lib_id` → stdlib generic mapping
  - `GenericInfo`: module_path, module_name, pin_map, flags
  - `map_symbol()`: Maps Device:R → Resistor.zen, Device:C → Capacitor.zen, etc.
  - Supports: R, C, L, D, LED, Ferrite_Bead, Crystal, Thermistor, BJT, MOSFET, TestPoint
- `crates/pcb-kicad2zen/src/mapping/footprints.rs` - Footprint → package extraction
  - `extract_package()`: R_0402_1005Metric → "0402", LED_0805_2012Metric → "0805"
  - Regex patterns for SMD, Crystal, SOD, SOT, QFN, SOIC, TSSOP packages
- `crates/pcb-kicad2zen/src/mapping/values.rs` - Value normalization
  - `normalize_value()`: 10k → 10kohm, 4k7 → 4.7kohm, 100n → 100nF
  - `ComponentType` inference from lib_id
  - MPN detection to avoid mangling part numbers
- `crates/pcb-kicad2zen/src/mapping/nets.rs` - Net type inference
  - `infer_net_type()`: VCC → Power, GND → Ground, USB_D_P → DiffPairP
  - Regex patterns for power/ground/diffpair detection

**Files modified:**
- `crates/pcb-kicad2zen/Cargo.toml` - Added `regex` dependency
- `crates/pcb-kicad2zen/src/lib.rs` - Added `pub mod mapping`

**Tests added:** 25 (symbols: 6, footprints: 6, values: 7, nets: 6)
