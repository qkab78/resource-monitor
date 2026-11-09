use std::process;
use std::time::Duration;

use domain::{AppId, AppUsage, History, SortOrder, Usage, group_by_app, matches_filter};

use crate::{ProbeError, SystemProbe, SystemTotals};

pub const SAMPLE_INTERVAL: Duration = Duration::from_secs(2);
/// 150 échantillons à 2 s : 5 minutes d'historique, en mémoire uniquement.
const HISTORY_LENGTH: usize = 150;

#[derive(Clone, Debug, PartialEq)]
pub struct GlobalHistory {
    pub cpu_percent: History,
    pub memory_percent: History,
    pub gpu_percent: History,
    pub disk_bytes_per_sec: History,
    pub network_bytes_per_sec: History,
}

impl GlobalHistory {
    fn new() -> Self {
        let empty = History::new(HISTORY_LENGTH);
        Self {
            cpu_percent: empty.clone(),
            memory_percent: empty.clone(),
            gpu_percent: empty.clone(),
            disk_bytes_per_sec: empty.clone(),
            network_bytes_per_sec: empty,
        }
    }
}

/// État complet présenté à l'utilisateur après un échantillon.
#[derive(Clone, Debug, PartialEq)]
pub struct Dashboard {
    pub apps: Vec<AppUsage>,
    pub totals: SystemTotals,
    /// Somme des applications : le disque et le réseau n'ont pas de total côté OS.
    pub apps_total: Usage,
    pub history: GlobalHistory,
}

impl Dashboard {
    fn empty() -> Self {
        Self {
            apps: Vec::new(),
            totals: SystemTotals::default(),
            apps_total: Usage::default(),
            history: GlobalHistory::new(),
        }
    }

    /// Applications à afficher, filtrées puis triées (processus compris).
    pub fn visible_apps(&self, filter: &str, order: SortOrder) -> Vec<AppUsage> {
        let mut apps: Vec<AppUsage> = self
            .apps
            .iter()
            .filter(|app| matches_filter(app, filter))
            .cloned()
            .collect();
        apps.sort_by(|a, b| order.compare_apps(a, b));
        for app in &mut apps {
            app.processes
                .sort_by(|a, b| order.compare_usage(&a.usage, &b.usage));
        }
        apps
    }

    pub fn app(&self, id: &AppId) -> Option<&AppUsage> {
        self.apps.iter().find(|app| &app.id == id)
    }

    /// Application la plus consommatrice selon le critère donné.
    pub fn top_app(&self, column: domain::SortColumn) -> Option<&AppUsage> {
        let order = SortOrder {
            column,
            direction: domain::SortDirection::Descending,
        };
        self.apps.iter().min_by(|a, b| order.compare_apps(a, b))
    }
}

/// Cas d'usage principal : transforme les échantillons bruts en tableau de bord.
pub struct Monitor {
    probe: Box<dyn SystemProbe>,
    dashboard: Dashboard,
    own_pid: u32,
}

impl Monitor {
    pub fn new(probe: Box<dyn SystemProbe>) -> Self {
        Self {
            probe,
            dashboard: Dashboard::empty(),
            own_pid: process::id(),
        }
    }

    pub fn refresh(&mut self) -> Result<Dashboard, ProbeError> {
        let sample = self.probe.sample()?;
        let apps = group_by_app(sample.processes, self.own_pid);
        let apps_total = apps
            .iter()
            .fold(Usage::default(), |total, app| total + app.usage);
        let gpu_percent = sample.totals.gpu_percent.or(apps_total.gpu_percent);

        let history = &mut self.dashboard.history;
        history.cpu_percent.push(sample.totals.cpu_percent);
        if sample.totals.memory_total_bytes > 0 {
            history.memory_percent.push(
                sample.totals.memory_used_bytes as f64 / sample.totals.memory_total_bytes as f64
                    * 100.0,
            );
        }
        if let Some(value) = gpu_percent {
            history.gpu_percent.push(value);
        }
        if let Some(value) = apps_total.disk_bytes_per_sec {
            history.disk_bytes_per_sec.push(value);
        }
        if let Some(value) = apps_total.network_bytes_per_sec {
            history.network_bytes_per_sec.push(value);
        }

        self.dashboard.apps = apps;
        self.dashboard.apps_total = apps_total;
        self.dashboard.totals = SystemTotals {
            gpu_percent,
            ..sample.totals
        };
        Ok(self.dashboard.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SystemSample;
    use domain::{ProcessId, ProcessSample, SortColumn, SortDirection};

    struct FakeProbe(Vec<Result<SystemSample, ProbeError>>);

    impl SystemProbe for FakeProbe {
        fn sample(&mut self) -> Result<SystemSample, ProbeError> {
            self.0.remove(0)
        }
    }

    fn process(pid: u32, path: &str, memory: u64, network: Option<f64>) -> ProcessSample {
        ProcessSample {
            id: ProcessId { pid, started_at: 1 },
            name: path.rsplit('/').next().unwrap().into(),
            executable: Some(path.into()),
            usage: Usage {
                memory_bytes: Some(memory),
                network_bytes_per_sec: network,
                ..Usage::default()
            },
        }
    }

    fn sample() -> SystemSample {
        SystemSample {
            processes: vec![
                process(
                    1,
                    "/Applications/Discord.app/Contents/MacOS/Discord",
                    100,
                    Some(5.0),
                ),
                process(
                    2,
                    "/Applications/Discord.app/Contents/Helper",
                    300,
                    Some(1.0),
                ),
                process(3, "/Applications/Zed.app/Contents/MacOS/zed", 200, None),
            ],
            totals: SystemTotals {
                cpu_percent: 12.0,
                memory_used_bytes: 50,
                memory_total_bytes: 100,
                memory_breakdown: None,
                gpu_percent: None,
            },
        }
    }

    #[test]
    fn un_echantillon_devient_un_classement_par_application() {
        let mut monitor = Monitor::new(Box::new(FakeProbe(vec![Ok(sample())])));
        let dashboard = monitor.refresh().unwrap();

        let apps = dashboard.visible_apps("", SortOrder::default());
        assert_eq!(apps[0].name, "Discord");
        assert_eq!(apps[0].usage.memory_bytes, Some(400));
        assert_eq!(apps[0].processes[0].usage.memory_bytes, Some(300));
        assert_eq!(dashboard.apps_total.network_bytes_per_sec, Some(6.0));
        assert_eq!(
            dashboard.top_app(SortColumn::Memory).unwrap().name,
            "Discord"
        );

        let by_name = SortOrder {
            column: SortColumn::Name,
            direction: SortDirection::Descending,
        };
        assert_eq!(dashboard.visible_apps("", by_name)[0].name, "Zed");
        assert_eq!(dashboard.visible_apps("zed", SortOrder::default()).len(), 1);
    }

    #[test]
    fn l_historique_ignore_une_mesure_indisponible_et_survit_a_une_erreur() {
        let mut monitor = Monitor::new(Box::new(FakeProbe(vec![
            Ok(sample()),
            Err(ProbeError("panne".into())),
            Ok(sample()),
        ])));
        monitor.refresh().unwrap();
        assert!(monitor.refresh().is_err());
        let history = monitor.refresh().unwrap().history;

        assert_eq!(
            history.memory_percent.values().collect::<Vec<_>>(),
            [50.0, 50.0]
        );
        assert_eq!(history.gpu_percent.values().len(), 0);
    }
}
