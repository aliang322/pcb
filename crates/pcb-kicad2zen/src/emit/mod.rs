//! Zener code emitter for KiCad projects

mod emitter;

pub use emitter::emit_zen;

/// Output mode for the emitter (kept for API compatibility, but behavior is unified)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputMode {
    /// Use stdlib generics (Resistor, Capacitor, etc.) - recommended
    #[default]
    Idiomatic,
    /// Legacy mode - same as Idiomatic (kept for backwards compatibility)
    Faithful,
}
