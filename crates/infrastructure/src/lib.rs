//! Adaptateurs vers l'OS : implémentent les ports de la couche application.

// Les parseurs macOS restent testés sur tous les OS, même si seul macOS les utilise.
#[cfg(any(target_os = "macos", test))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code, unused_imports))]
mod macos;
#[cfg(any(target_os = "macos", test))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
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
        footprint: Some(macos::phys_footprint),
        memory_breakdown: Some(macos::memory_breakdown),
    }
}

/// Windows : réseau et GPU ne sont pas encore collectés, ils s'affichent comme indisponibles.
#[cfg(not(target_os = "macos"))]
fn extra_sources() -> sysinfo_probe::ExtraSources {
    sysinfo_probe::ExtraSources {
        network: None,
        gpu: None,
        footprint: None,
        memory_breakdown: None,
    }
}
