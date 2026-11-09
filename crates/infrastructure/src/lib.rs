//! Adaptateurs vers l'OS : implémentent les ports de la couche application.

#[cfg(any(target_os = "macos", test))]
mod macos;
mod rates;
mod sysinfo_probe;
mod terminator;

use application::{ProbeError, ProcessTerminator, SystemProbe};

/// Sonde adaptée à l'OS courant.
pub fn default_probe() -> Result<Box<dyn SystemProbe>, ProbeError> {
    Ok(Box::new(sysinfo_probe::SysinfoProbe::new(extra_sources())?))
}

pub fn default_terminator() -> Box<dyn ProcessTerminator> {
    Box::new(terminator::SysinfoTerminator)
}

#[cfg(target_os = "macos")]
fn extra_sources() -> sysinfo_probe::ExtraSources {
    sysinfo_probe::ExtraSources {
        network: Some(Box::new(macos::NettopSource::default())),
        gpu: Some(Box::new(macos::IoregGpuSource::default())),
    }
}

/// Windows : réseau et GPU ne sont pas encore collectés, ils s'affichent comme indisponibles.
#[cfg(not(target_os = "macos"))]
fn extra_sources() -> sysinfo_probe::ExtraSources {
    sysinfo_probe::ExtraSources {
        network: None,
        gpu: None,
    }
}
