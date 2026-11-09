use std::ops::Add;

/// Consommation d'un processus ou d'une application.
///
/// `None` signifie « mesure indisponible », ce qui est différent de zéro.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Usage {
    /// Part de la capacité totale de la machine (0–100), tous cœurs confondus.
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
    pub gpu_percent: Option<f64>,
    pub disk_bytes_per_sec: Option<f64>,
    pub network_bytes_per_sec: Option<f64>,
}

impl Add for Usage {
    type Output = Usage;

    fn add(self, other: Usage) -> Usage {
        Usage {
            cpu_percent: sum(self.cpu_percent, other.cpu_percent),
            memory_bytes: sum(self.memory_bytes, other.memory_bytes),
            gpu_percent: sum(self.gpu_percent, other.gpu_percent),
            disk_bytes_per_sec: sum(self.disk_bytes_per_sec, other.disk_bytes_per_sec),
            network_bytes_per_sec: sum(self.network_bytes_per_sec, other.network_bytes_per_sec),
        }
    }
}

fn sum<T: Add<Output = T>>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a + b),
        (value, None) | (None, value) => value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_mesure_indisponible_ne_compte_pas_comme_zero() {
        let known = Usage {
            memory_bytes: Some(10),
            ..Usage::default()
        };
        assert_eq!((known + Usage::default()).memory_bytes, Some(10));
        assert_eq!((Usage::default() + Usage::default()).memory_bytes, None);
        assert_eq!((known + known).memory_bytes, Some(20));
    }
}
