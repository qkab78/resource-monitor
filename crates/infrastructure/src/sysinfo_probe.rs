use std::collections::HashMap;
use std::time::Instant;

use application::{MemoryBreakdown, ProbeError, SystemProbe, SystemSample, SystemTotals};
use domain::{ProcessId, ProcessSample, Usage};
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

/// Mesure propre à un OS, par PID. `None` : la mesure est indisponible à ce tour.
pub(crate) trait RateSource: Send {
    fn sample(&mut self) -> Option<HashMap<u32, f64>>;
    /// Utilisation globale, quand la source la connaît.
    fn total(&self) -> Option<f64> {
        None
    }
}

pub(crate) struct ExtraSources {
    pub(crate) network: Option<Box<dyn RateSource>>,
    pub(crate) gpu: Option<Box<dyn RateSource>>,
    /// Mémoire d'un processus selon l'OS. Sans source ou sans réponse : le RSS de `sysinfo`.
    pub(crate) footprint: Option<fn(u32) -> Option<u64>>,
    /// Mémoire du système et du compresseur, quand l'OS la fournit.
    pub(crate) memory_breakdown: Option<fn() -> Option<MemoryBreakdown>>,
}

/// Collecte CPU, RAM et disque via `sysinfo`, et complète avec les sources de l'OS.
pub(crate) struct SysinfoProbe {
    system: System,
    last_sample: Option<Instant>,
    extra: ExtraSources,
}

impl SysinfoProbe {
    pub(crate) fn new(extra: ExtraSources) -> Result<Self, ProbeError> {
        Ok(Self {
            system: System::new().map_err(|error| ProbeError(error.to_string()))?,
            last_sample: None,
            extra,
        })
    }
}

impl SystemProbe for SysinfoProbe {
    fn sample(&mut self) -> Result<SystemSample, ProbeError> {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing()
                .with_cpu()
                .with_memory()
                .with_disk_usage()
                .with_exe(UpdateKind::OnlyIfNotSet),
        );
        let now = Instant::now();
        let elapsed = self.last_sample.map(|last| now.duration_since(last));
        self.last_sample = Some(now);

        let network = self
            .extra
            .network
            .as_mut()
            .and_then(|source| source.sample());
        let gpu = self.extra.gpu.as_mut().and_then(|source| source.sample());
        let footprint = self.extra.footprint;
        let cores = self.system.cpus().len().max(1) as f64;

        let processes = self
            .system
            .processes()
            .iter()
            // Sous Linux, sysinfo liste aussi les threads comme des processus.
            .filter(|(_, process)| process.thread_kind().is_none())
            .map(|(pid, process)| {
                let pid = pid.as_u32();
                // Mémoire à 0 : l'OS nous refuse la lecture (processus d'un autre utilisateur).
                let readable = process.memory() > 0;
                let disk = process.disk_usage();
                let disk_bytes = disk.read_bytes + disk.written_bytes;
                ProcessSample {
                    id: ProcessId {
                        pid,
                        started_at: process.start_time(),
                    },
                    name: process.name().to_string_lossy().into_owned(),
                    executable: process
                        .exe()
                        .map(|path| path.to_string_lossy().into_owned()),
                    usage: Usage {
                        cpu_percent: (readable && elapsed.is_some())
                            .then(|| f64::from(process.cpu_usage()) / cores),
                        memory_bytes: readable.then(|| {
                            footprint
                                .and_then(|read| read(pid))
                                .unwrap_or_else(|| process.memory())
                        }),
                        gpu_percent: gpu.as_ref().map(|map| rate_or_zero(map, pid)),
                        disk_bytes_per_sec: elapsed
                            .filter(|_| readable)
                            .map(|elapsed| disk_bytes as f64 / elapsed.as_secs_f64().max(0.001)),
                        network_bytes_per_sec: network.as_ref().map(|map| rate_or_zero(map, pid)),
                    },
                }
            })
            .collect();

        Ok(SystemSample {
            processes,
            totals: SystemTotals {
                cpu_percent: f64::from(self.system.global_cpu_usage()),
                memory_used_bytes: self.system.used_memory(),
                memory_total_bytes: self.system.total_memory(),
                memory_breakdown: self.extra.memory_breakdown.and_then(|read| read()),
                gpu_percent: self.extra.gpu.as_ref().and_then(|source| source.total()),
            },
        })
    }
}

/// La source fonctionne et ne cite pas ce PID : il n'utilise pas la ressource.
fn rate_or_zero(rates: &HashMap<u32, f64>, pid: u32) -> f64 {
    rates.get(&pid).copied().unwrap_or(0.0)
}
