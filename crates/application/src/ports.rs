use domain::{ProcessId, ProcessSample};

/// Mesures globales de la machine, indépendantes du détail par processus.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SystemTotals {
    pub cpu_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    /// Part de la mémoire utilisée qui n'est attribuée à aucune application. `None` sur Windows.
    pub memory_breakdown: Option<MemoryBreakdown>,
    pub gpu_percent: Option<f64>,
}

/// Mémoire utilisée hors applications.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MemoryBreakdown {
    /// Pages verrouillées en RAM, surtout par le noyau.
    pub wired_bytes: u64,
    /// RAM occupée par le compresseur, qui y range les pages compressées des applications.
    pub compressor_bytes: u64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemSample {
    pub processes: Vec<ProcessSample>,
    pub totals: SystemTotals,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbeError(pub String);

/// Port de collecte, implémenté par la couche infrastructure pour chaque OS.
pub trait SystemProbe: Send {
    fn sample(&mut self) -> Result<SystemSample, ProbeError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminateError {
    /// Le PID existe, mais il désigne désormais un autre processus.
    ProcessReplaced,
    Denied,
}

/// Port d'action sur les processus.
pub trait ProcessTerminator: Send + Sync {
    /// Termine le processus ; réussit aussi s'il s'est arrêté entre-temps.
    fn terminate(&self, process: ProcessId) -> Result<(), TerminateError>;
}
