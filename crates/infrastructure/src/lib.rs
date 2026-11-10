//! Adaptateurs vers l'OS : implémentent les ports de la couche application.

// Les parseurs de chaque OS restent testés partout, même si un seul OS les utilise.
#[cfg(any(target_os = "macos", test))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code, unused_imports))]
mod macos;
#[cfg(any(target_os = "macos", test))]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
mod rates;
mod sysinfo_probe;
mod terminator;
#[cfg(any(windows, test))]
#[cfg_attr(not(windows), allow(dead_code, unused_imports))]
mod windows;

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

/// Windows : GPU par processus via PDH. Le réseau n'est connu qu'au total, sans détail par processus.
#[cfg(windows)]
fn extra_sources() -> sysinfo_probe::ExtraSources {
    sysinfo_probe::ExtraSources {
        network: Some(Box::new(windows::InterfacesNetworkSource::new())),
        gpu: Some(Box::new(windows::PdhGpuSource::default())),
        footprint: None,
        memory_breakdown: None,
    }
}

/// Autres OS : réseau et GPU ne sont pas collectés, ils s'affichent comme indisponibles.
#[cfg(not(any(target_os = "macos", windows)))]
fn extra_sources() -> sysinfo_probe::ExtraSources {
    sysinfo_probe::ExtraSources {
        network: None,
        gpu: None,
        footprint: None,
        memory_breakdown: None,
    }
}
