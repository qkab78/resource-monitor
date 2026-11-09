//! Cas d'usage et ports : ce que l'application fait, sans savoir comment l'OS ou l'écran le font.

mod monitor;
mod ports;
mod terminate;

pub use monitor::{Dashboard, GlobalHistory, Monitor, SAMPLE_INTERVAL};
pub use ports::{
    ProbeError, ProcessTerminator, SystemProbe, SystemSample, SystemTotals, TerminateError,
};
pub use terminate::{TerminationReport, terminate_app};
