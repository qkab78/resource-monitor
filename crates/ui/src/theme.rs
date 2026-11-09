//! Couleurs et polices de la maquette.

pub const GROUND: u32 = 0x101218;
pub const PANEL: u32 = 0x181B22;
pub const NESTED: u32 = 0x14171D;
pub const BORDER: u32 = 0x2A2F3A;
pub const BORDER_STRONG: u32 = 0x3A404D;
pub const TEXT: u32 = 0xEDEFF3;
pub const TEXT_SOFT: u32 = 0xC6CBD5;
pub const MUTED: u32 = 0xA3ABB9;
pub const BAR: u32 = 0x6B7382;
pub const ACCENT: u32 = 0xF2A33A;

#[cfg(target_os = "windows")]
pub const MONO: &str = "Consolas";
#[cfg(not(target_os = "windows"))]
pub const MONO: &str = "Menlo";
