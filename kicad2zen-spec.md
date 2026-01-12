# kicad2zen: KiCad to Zener Importer

## Overview

Converts KiCad projects to Zener (`.zen`) files, enabling:
1. Migration of existing KiCad designs to Zener
2. Round-trip validation (Zener → KiCad → Zener)
3. Creating training/reference .zen data from real-world KiCad designs

## Usage

```bash
# Import a KiCad project
pcb import kicad <directory>

# Output to stdout instead of file
pcb import kicad <directory> --stdout

# Custom output path
pcb import kicad <directory> -o my-design.zen
```

**Input:** Directory containing `.kicad_sch`, `.kicad_pcb`, and/or `.kicad_pro` files

**Output:** `<project>-imported.zen` (avoids overwriting original `.zen` if present)

## Architecture

```
crates/pcb-kicad2zen/
├── src/
│   ├── lib.rs              # KicadProject struct, parse() and to_zen()
│   ├── parser/
│   │   ├── schematic.rs    # .kicad_sch S-expression parser
│   │   ├── pcb.rs          # .kicad_pcb S-expression parser
│   │   └── project.rs      # .kicad_pro JSON parser
│   ├── mapping/
│   │   ├── symbols.rs      # lib_id → stdlib generic (Device:R → Resistor)
│   │   ├── footprints.rs   # Footprint → package (R_0402_1005Metric → "0402")
│   │   ├── values.rs       # Value normalization (10k → 10kohm)
│   │   └── nets.rs         # Net type inference (VCC → Power)
│   └── emit/
│       ├── mod.rs          # OutputMode enum
│       └── emitter.rs      # Unified Zener code generator
```

## Output Format

The importer generates **buildable Zener code** using stdlib generics:

```python
# Auto-generated from KiCad project: blinky
# Import with: pcb import kicad <path>

# ```pcb
# [workspace]
# pcb-version = "0.3"
# ```

load("@stdlib/board_config.zen", "Board")
load("@stdlib/interfaces.zen", "Power", "Ground")

Resistor = Module("@stdlib/generics/Resistor.zen")
Led = Module("@stdlib/generics/Led.zen")

# Nets
VCC = Power("VCC")
GND = Ground("GND")
LED_ANODE = Net("LED_ANODE")

# Components
Resistor(
    name = "R1",
    value = "1kohm",
    package = "0402",
    P1 = VCC,
    P2 = LED_ANODE,
)

Led(
    name = "D1",
    package = "0402",
    color = "red",
    K = GND,
    A = LED_ANODE,
)

# Board configuration
Board(
    name = "blinky-imported",
    layers = 4,
    layout_path = "layout/blinky-imported"
)
```

## Component Type Mapping

### From Schematic (lib_id → stdlib generic)

| KiCad lib_id | Zener Module | Pin Map |
|--------------|--------------|---------|
| `Device:R` | `Resistor` | 1→P1, 2→P2 |
| `Device:C` | `Capacitor` | 1→P1, 2→P2 |
| `Device:C_Polarized` | `Capacitor` (polarized=true) | 1→P, 2→N |
| `Device:L` | `Inductor` | 1→P1, 2→P2 |
| `Device:D` | `Diode` | 1→K, 2→A |
| `Device:LED` | `Led` | 1→K, 2→A |
| `Device:Q_NPN_*` | `Bjt` | varies |
| `Device:Q_NMOS_*` | `Mosfet` | varies |
| Unknown | TODO comment | — |

### From PCB Footprint (when no schematic)

| Footprint Prefix | Inferred Type |
|------------------|---------------|
| `R_*`, `Resistor_*` | Resistor |
| `C_*`, `Capacitor_*` | Capacitor |
| `L_*`, `Inductor_*` | Inductor |
| `LED_*` | Led |
| `D_*`, `Diode_*` | Diode |
| Unknown | TODO comment |

### Package Extraction

Regex patterns extract package size from footprint names:
- `R_0402_1005Metric` → `"0402"`
- `LED_0805_2012Metric` → `"0805"`
- `C_0603_1608Metric` → `"0603"`

### Net Type Inference

| Pattern | Zener Type |
|---------|------------|
| `VCC`, `VDD`, `+*V` | `Power()` |
| `GND`, `VSS`, `DGND` | `Ground()` |
| Other | `Net()` |

## Unknown Components

Components that can't be mapped to stdlib generics are emitted as TODO comments:

```python
# TODO: Unknown component type 'Analog_ADC:AD7171' - manual conversion needed
# Reference: U1, Value: AD7171, Footprint: Package_DFN_QFN:DFN-10
# Pins: 1->VCC, 2->GND, 3->DOUT
```


## Limitations

### Current Limitations

1. **Passive components only**: Full mapping support for R, C, L, D, LED, BJT, MOSFET. Complex ICs require manual conversion.

2. **Schematic connectivity ignored**: Net connections are derived from PCB pad-nets, not schematic wires. Works well in practice since PCB is authoritative for physical connectivity.

3. **No hierarchical schematics**: Only single-sheet schematics are parsed.

4. **No board outline**: Board edge cuts and keepouts are not extracted.

### Extensibility for Real-World KiCad Files

The importer is designed to handle arbitrary KiCad files gracefully:

**What works automatically:**
- Any passive component with standard KiCad `Device:*` symbols
- Any footprint following standard naming (R_XXXX, C_XXXX, LED_XXXX)
- Any net name (auto-classified as Power/Ground/Signal)

**What requires mapping table updates:**
- New symbol libraries (e.g., manufacturer-specific symbols)
- Non-standard footprint naming conventions

**What requires manual conversion:**
- Complex ICs (ADCs, microcontrollers, FPGAs)
- Custom/proprietary components
- Components with unusual pin mappings

### Adding New Component Mappings

To support a new component type, update `src/mapping/symbols.rs`:

```rust
// In SYMBOL_MAP static
("Analog_ADC:AD7171", GenericInfo {
    module_path: "@stdlib/generics/Adc.zen",
    module_name: "Adc",
    pin_map: &[("AIN+", "AIN_P"), ("AIN-", "AIN_N"), ...],
    flags: &[],
}),
```

To support new footprint inference, update `src/mapping/footprints.rs`:

```rust
// In infer_component_type()
if fp_name.starts_with("QFN_") || lower.contains("qfn") {
    FootprintComponentType::Ic
}
```

## Round-Trip Workflow

```bash
# 1. Start with a Zener design
pcb build blinky.zen
pcb layout blinky.zen

# 2. KiCad output is in layout/blinky/
ls layout/blinky/
# layout.kicad_pcb, layout.kicad_pro

# 3. Import back to Zener
pcb import kicad layout/blinky
# → blinky-imported.zen

# 4. Build the imported design
pcb build blinky-imported.zen
# → layout/blinky-imported/
```

## Test Examples

```bash
# Test with included KiCad project
pcb import kicad crates/pcb-sch/test/kicad-bom --stdout

# Verify output is buildable
pcb import kicad crates/pcb-sch/test/kicad-bom
pcb build kicad-bom-imported.zen
```

## Future Improvements

### Board & Design Rule Extraction

- **Board outline**: Parse `Edge.Cuts` layer to extract board shape for `Board()` configuration
- **Design rules**: Extract track widths, clearances, via sizes from `.kicad_pro` for `BoardConfig()`

### Module Inference for Non-Zener Projects

**Problem:** Importing KiCad projects not created from Zener produces a flat component list, losing hierarchy and reusability—e.g., an LC matching network appears as separate `Inductor`, `Capacitor`, `Resistor` calls rather than a single `MatchingNetwork` module with `io` ports.

**Approach 1 - KiCad Hierarchy Hints:** Parse hierarchical sheet instances from `.kicad_sch` and detect module prefixes in net names (e.g., `MatchingNetwork.T` implies module "MatchingNetwork" with internal net "T").

**Approach 2 - Topology Pattern Matching:** Build a connectivity graph and match against known subcircuit patterns (Pi/T filters, voltage dividers, decoupling clusters) to identify module boundaries and infer `io()` ports.

**Approach 3 - Agentic Extraction:** Use an LLM agent to reason about circuit function, search existing `.zen` modules via RAG for similar patterns, and generate structured module code—with optional interactive refinement for complex designs.
