//! Sources propres à macOS. `nettop` et `ioreg` sont fournis par l'OS et ne demandent pas root.

#[cfg(target_os = "macos")]
mod footprint;
mod gpu;
mod network;
#[cfg(target_os = "macos")]
mod vm_stats;

#[cfg(target_os = "macos")]
pub(crate) use footprint::phys_footprint;
pub(crate) use gpu::IoregGpuSource;
pub(crate) use network::NettopSource;
#[cfg(target_os = "macos")]
pub(crate) use vm_stats::memory_breakdown;
