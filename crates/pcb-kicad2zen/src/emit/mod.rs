//! Zen code emitters for KiCad projects

mod faithful;
mod idiomatic;

pub use faithful::emit_faithful;
pub use idiomatic::emit_idiomatic;

use crate::{KicadProject, OutputMode};

/// Emit Zener source code from a KiCad project
pub fn emit_zen(project: &KicadProject, mode: OutputMode) -> String {
    match mode {
        OutputMode::Faithful => emit_faithful(project),
        OutputMode::Idiomatic => emit_idiomatic(project),
    }
}
