//! Sources propres à Windows. Les compteurs de performance (PDH) ne demandent pas de droits admin.

mod gpu;
mod network;

#[cfg(windows)]
pub(crate) use gpu::PdhGpuSource;
pub(crate) use network::InterfacesNetworkSource;
