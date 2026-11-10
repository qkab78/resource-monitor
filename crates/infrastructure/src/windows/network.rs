use std::collections::HashMap;
use std::time::Instant;

use sysinfo::Networks;

use crate::sysinfo_probe::RateSource;

/// Débit réseau de toute la machine, lu sur les interfaces.
///
/// Windows n'expose pas le débit par processus sans droits admin : seul le total est connu.
pub(crate) struct InterfacesNetworkSource {
    networks: Option<Networks>,
    last: Instant,
    total: Option<f64>,
}

impl InterfacesNetworkSource {
    pub(crate) fn new() -> Self {
        Self {
            networks: Networks::new_with_refreshed_list().ok(),
            last: Instant::now(),
            total: None,
        }
    }
}

impl RateSource for InterfacesNetworkSource {
    /// Aucun détail par processus : la colonne reste « indisponible ».
    fn sample(&mut self) -> Option<HashMap<u32, f64>> {
        self.total = None;
        let networks = self.networks.as_mut()?;
        networks.refresh(true);
        let now = Instant::now();
        let elapsed = now.duration_since(std::mem::replace(&mut self.last, now));
        let bytes: u64 = networks
            .iter()
            .filter(|(name, _)| !is_loopback(&name.to_string_lossy()))
            .map(|(_, data)| data.received() + data.transmitted())
            .sum();
        self.total = Some(bytes as f64 / elapsed.as_secs_f64().max(0.001));
        None
    }

    fn total(&self) -> Option<f64> {
        self.total
    }
}

/// Le trafic local ne quitte pas la machine : on l'exclut, comme le Gestionnaire des tâches.
fn is_loopback(name: &str) -> bool {
    name == "lo" || name.to_ascii_lowercase().contains("loopback")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_l_interface_de_bouclage() {
        assert!(is_loopback("Loopback Pseudo-Interface 1"));
        assert!(is_loopback("lo"));
        assert!(!is_loopback("Wi-Fi"));
        assert!(!is_loopback("Ethernet"));
    }
}
