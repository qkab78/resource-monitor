//! Mise en forme des mesures, à la française.

use application::MemoryBreakdown;

pub const UNAVAILABLE: &str = "—";
const KIB: f64 = 1024.0;

pub fn percent(value: Option<f64>) -> String {
    value.map_or(UNAVAILABLE.into(), |value| format!("{} %", decimal(value)))
}

pub fn bytes(value: Option<u64>) -> String {
    value.map_or(UNAVAILABLE.into(), |value| scaled(value as f64))
}

/// Mémoire utilisée qui n'apparaît dans aucune ligne d'application.
pub fn memory_breakdown(breakdown: MemoryBreakdown) -> String {
    format!(
        "dont système {} · compresseur {}",
        scaled(breakdown.wired_bytes as f64),
        scaled(breakdown.compressor_bytes as f64)
    )
}

pub fn rate(value: Option<f64>) -> String {
    value.map_or(UNAVAILABLE.into(), |value| format!("{}/s", scaled(value)))
}

fn scaled(value: f64) -> String {
    if value >= KIB * KIB * KIB {
        format!("{} Go", decimal(value / (KIB * KIB * KIB)))
    } else if value >= KIB * KIB {
        format!("{} Mo", decimal(value / (KIB * KIB)))
    } else if value >= KIB {
        format!("{:.0} Ko", value / KIB)
    } else {
        format!("{value:.0} o")
    }
}

fn decimal(value: f64) -> String {
    format!("{value:.1}").replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_unites_s_adaptent_a_la_valeur() {
        assert_eq!(bytes(Some(512)), "512 o");
        assert_eq!(bytes(Some(2048)), "2 Ko");
        assert_eq!(bytes(Some(5 * 1024 * 1024)), "5,0 Mo");
        assert_eq!(bytes(Some(3 * 1024 * 1024 * 1024 / 2)), "1,5 Go");
        assert_eq!(rate(Some(1024.0 * 1024.0)), "1,0 Mo/s");
        assert_eq!(percent(Some(12.34)), "12,3 %");
    }

    #[test]
    fn une_mesure_indisponible_s_affiche_avec_un_tiret() {
        assert_eq!(percent(None), "—");
        assert_eq!(bytes(None), "—");
        assert_eq!(rate(None), "—");
    }

    #[test]
    fn la_memoire_hors_applications_est_detaillee() {
        let breakdown = MemoryBreakdown {
            wired_bytes: 2 * 1024 * 1024 * 1024,
            compressor_bytes: 512 * 1024 * 1024,
        };
        assert_eq!(
            memory_breakdown(breakdown),
            "dont système 2,0 Go · compresseur 512,0 Mo"
        );
    }
}
