//! Mapping utilities for KiCad → Zener translation

mod footprints;
mod nets;
mod symbols;
mod values;

pub use footprints::extract_package;
pub use nets::{infer_net_type, NetType};
pub use symbols::{map_symbol, GenericInfo};
pub use values::normalize_value;
