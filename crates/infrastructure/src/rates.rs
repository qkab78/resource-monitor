use std::collections::HashMap;
use std::time::Duration;

/// Convertit des compteurs cumulés par PID en débit par seconde.
#[derive(Default)]
pub(crate) struct RateTracker {
    previous: HashMap<u32, u64>,
}

impl RateTracker {
    /// Rend `None` au premier appel. Un compteur nouveau ou qui recule vaut 0 pour ce tour.
    pub(crate) fn update(
        &mut self,
        counters: HashMap<u32, u64>,
        elapsed: Duration,
    ) -> Option<HashMap<u32, f64>> {
        let first = self.previous.is_empty();
        let seconds = elapsed.as_secs_f64().max(0.001);
        let rates = counters
            .iter()
            .map(|(pid, value)| {
                let delta = self
                    .previous
                    .get(pid)
                    .map_or(0, |previous| value.saturating_sub(*previous));
                (*pid, delta as f64 / seconds)
            })
            .collect();
        self.previous = counters;
        (!first).then_some(rates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_debit_vient_de_la_difference_entre_deux_mesures() {
        let mut tracker = RateTracker::default();
        let elapsed = Duration::from_secs(2);
        assert!(tracker.update(HashMap::from([(1, 100)]), elapsed).is_none());

        let rates = tracker
            .update(HashMap::from([(1, 300), (2, 999)]), elapsed)
            .unwrap();
        assert_eq!(rates[&1], 100.0);
        assert_eq!(rates[&2], 0.0, "un nouveau PID n'a pas encore de débit");

        let rates = tracker.update(HashMap::from([(1, 50)]), elapsed).unwrap();
        assert_eq!(
            rates[&1], 0.0,
            "un compteur qui recule ne donne pas de débit négatif"
        );
    }
}
