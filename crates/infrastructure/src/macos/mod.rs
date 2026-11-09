//! Sources propres à macOS. `nettop` et `ioreg` sont fournis par l'OS et ne demandent pas root.

mod gpu;
mod network;

pub(crate) use gpu::IoregGpuSource;
pub(crate) use network::NettopSource;
