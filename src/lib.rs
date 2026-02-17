pub mod configuration;
pub mod format_text;
pub mod generation;

pub use format_text::format_text;

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod wasm_shims;

#[cfg(feature = "wasm")]
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
mod wasm_plugin;

#[cfg(feature = "wasm")]
#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub use wasm_plugin::*;

