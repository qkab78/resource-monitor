use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

use crate::rates::RateTracker;
use crate::sysinfo_probe::RateSource;

/// Débit réseau par processus, à partir des compteurs cumulés de `nettop`.
#[derive(Default)]
pub(crate) struct NettopSource {
    tracker: RateTracker,
    last: Option<Instant>,
}

impl RateSource for NettopSource {
    fn sample(&mut self) -> Option<HashMap<u32, f64>> {
        let output = Command::new("nettop")
            .args(["-P", "-L", "1", "-n", "-x", "-J", "bytes_in,bytes_out"])
            .output()
            .ok()
            .filter(|output| output.status.success())?;
        let counters = parse(&String::from_utf8_lossy(&output.stdout));
        let now = Instant::now();
        let elapsed = self.last.replace(now).map(|last| now.duration_since(last));
        self.tracker.update(counters, elapsed.unwrap_or_default())
    }
}

/// Lignes de la forme `nom.avec.points.123,octets_recus,octets_envoyes,`.
fn parse(csv: &str) -> HashMap<u32, u64> {
    csv.lines()
        .filter_map(|line| {
            let mut fields = line.rsplitn(4, ',');
            fields.next()?; // virgule finale
            let sent: u64 = fields.next()?.trim().parse().ok()?;
            let received: u64 = fields.next()?.trim().parse().ok()?;
            let (_, pid) = fields.next()?.rsplit_once('.')?;
            Some((pid.parse().ok()?, sent + received))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_les_compteurs_de_nettop() {
        let csv =
            ",bytes_in,bytes_out,\nlaunchd.1,1393634,154880,\ncom.apple.Web.42,10,5,\nabimé\n";
        let counters = parse(csv);
        assert_eq!(counters.len(), 2);
        assert_eq!(counters[&1], 1_548_514);
        assert_eq!(counters[&42], 15);
    }
}
