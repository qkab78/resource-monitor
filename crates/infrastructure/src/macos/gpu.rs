use std::collections::HashMap;
use std::process::Command;
use std::time::Instant;

use crate::rates::RateTracker;
use crate::sysinfo_probe::RateSource;

/// Utilisation GPU par processus, à partir du temps GPU cumulé exposé par l'IORegistry.
#[derive(Default)]
pub(crate) struct IoregGpuSource {
    tracker: RateTracker,
    last: Option<Instant>,
    device_percent: Option<f64>,
}

impl RateSource for IoregGpuSource {
    fn sample(&mut self) -> Option<HashMap<u32, f64>> {
        self.device_percent = run(&["-r", "-c", "IOAccelerator", "-d", "1", "-w", "0"])
            .as_deref()
            .and_then(parse_device_percent);

        let clients = run(&["-r", "-c", "IOAccelerator", "-l", "-w", "0"])?;
        let counters = parse_gpu_time(&clients);
        if counters.is_empty() {
            return None;
        }
        let now = Instant::now();
        let elapsed = self.last.replace(now).map(|last| now.duration_since(last));
        // Nanosecondes de GPU par seconde : on convertit le débit en pourcentage.
        let rates = self.tracker.update(counters, elapsed.unwrap_or_default())?;
        Some(
            rates
                .into_iter()
                .map(|(pid, ns_per_sec)| (pid, (ns_per_sec / 1e7).min(100.0)))
                .collect(),
        )
    }

    fn total(&self) -> Option<f64> {
        self.device_percent
    }
}

fn run(args: &[&str]) -> Option<String> {
    let output = Command::new("ioreg").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Chaque client GPU est un bloc `+-o` contenant `AppUsage` et `IOUserClientCreator`.
fn parse_gpu_time(ioreg: &str) -> HashMap<u32, u64> {
    let mut counters = HashMap::new();
    for block in ioreg.split("+-o ") {
        let Some(pid) = block.lines().find_map(creator_pid) else {
            continue;
        };
        let Some(usage) = block.lines().find(|line| line.contains("\"AppUsage\" =")) else {
            continue;
        };
        *counters.entry(pid).or_default() += accumulated_time(usage);
    }
    counters
}

fn creator_pid(line: &str) -> Option<u32> {
    let (_, rest) = line.split_once("\"IOUserClientCreator\" = \"pid ")?;
    rest.split(',').next()?.trim().parse().ok()
}

fn accumulated_time(line: &str) -> u64 {
    line.split("\"accumulatedGPUTime\"=")
        .skip(1)
        .filter_map(|part| {
            let end = part
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(part.len());
            part[..end].parse::<u64>().ok()
        })
        .sum()
}

fn parse_device_percent(ioreg: &str) -> Option<f64> {
    let (_, rest) = ioreg.split_once("\"Device Utilization %\"=")?;
    let end = rest.find(|c: char| !c.is_ascii_digit())?;
    rest[..end].parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
+-o AGXDeviceUserClient  <class AGXDeviceUserClient, id 0x1>
  |   {
  |     "AppUsage" = ({"API"="Metal","lastSubmittedTime"=9,"accumulatedGPUTime"=100},{"API"="Metal","accumulatedGPUTime"=20})
  |     "IOUserClientCreator" = "pid 623, WindowServer"
  |   }
+-o AGXDeviceUserClient  <class AGXDeviceUserClient, id 0x2>
  |   {
  |     "AppUsage" = ({"API"="Metal","accumulatedGPUTime"=3})
  |     "IOUserClientCreator" = "pid 623, WindowServer"
  |   }
+-o AGXDeviceUserClient  <class AGXDeviceUserClient, id 0x3>
  |   {
  |     "AppUsage" = ()
  |     "IOUserClientCreator" = "pid 1082, NotificationCent"
  |   }
"#;

    #[test]
    fn additionne_le_temps_gpu_par_processus() {
        let counters = parse_gpu_time(SAMPLE);
        assert_eq!(counters[&623], 123);
        assert_eq!(counters[&1082], 0);
    }

    #[test]
    fn lit_l_utilisation_globale() {
        let line = r#""PerformanceStatistics" = {"Tiler Utilization %"=4,"Device Utilization %"=37,"x"=1}"#;
        assert_eq!(parse_device_percent(line), Some(37.0));
        assert_eq!(parse_device_percent("rien"), None);
    }
}
