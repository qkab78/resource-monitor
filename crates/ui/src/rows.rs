//! Transforme les applications en lignes de tableau, indépendamment du rendu.

use std::collections::HashSet;

use domain::{AppId, AppUsage, ProcessSample, SortColumn, Usage};

use crate::format;

pub const METRIC_COLUMNS: [(SortColumn, &str); 5] = [
    (SortColumn::Cpu, "CPU"),
    (SortColumn::Memory, "RAM"),
    (SortColumn::Gpu, "GPU"),
    (SortColumn::Disk, "Disque"),
    (SortColumn::Network, "Réseau"),
];

pub enum Row {
    App { app: AppUsage, expanded: bool },
    Process(ProcessSample),
}

impl Row {
    pub fn usage(&self) -> &Usage {
        match self {
            Row::App { app, .. } => &app.usage,
            Row::Process(process) => &process.usage,
        }
    }
}

pub fn build_rows(apps: Vec<AppUsage>, expanded: &HashSet<AppId>) -> Vec<Row> {
    let mut rows = Vec::with_capacity(apps.len());
    for app in apps {
        let is_expanded = expanded.contains(&app.id);
        let processes = is_expanded.then(|| app.processes.clone());
        rows.push(Row::App {
            app,
            expanded: is_expanded,
        });
        rows.extend(processes.into_iter().flatten().map(Row::Process));
    }
    rows
}

pub fn metric_value(column: SortColumn, usage: &Usage) -> Option<f64> {
    match column {
        SortColumn::Name => None,
        SortColumn::Cpu => usage.cpu_percent,
        SortColumn::Memory => usage.memory_bytes.map(|bytes| bytes as f64),
        SortColumn::Gpu => usage.gpu_percent,
        SortColumn::Disk => usage.disk_bytes_per_sec,
        SortColumn::Network => usage.network_bytes_per_sec,
    }
}

pub fn metric_text(column: SortColumn, usage: &Usage) -> String {
    match column {
        SortColumn::Name => String::new(),
        SortColumn::Cpu => format::percent(usage.cpu_percent),
        SortColumn::Memory => format::bytes(usage.memory_bytes),
        SortColumn::Gpu => format::percent(usage.gpu_percent),
        SortColumn::Disk => format::rate(usage.disk_bytes_per_sec),
        SortColumn::Network => format::rate(usage.network_bytes_per_sec),
    }
}

/// Plus grande valeur d'application par colonne, pour dimensionner les barres.
pub fn column_maxima(apps: &[AppUsage]) -> [f64; METRIC_COLUMNS.len()] {
    METRIC_COLUMNS.map(|(column, _)| {
        apps.iter()
            .filter_map(|app| metric_value(column, &app.usage))
            .fold(0.0, f64::max)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::ProcessId;

    fn app(name: &str, pids: &[u32], memory: u64) -> AppUsage {
        AppUsage {
            id: AppId(name.into()),
            name: name.into(),
            usage: Usage {
                memory_bytes: Some(memory),
                ..Usage::default()
            },
            processes: pids
                .iter()
                .map(|pid| ProcessSample {
                    id: ProcessId {
                        pid: *pid,
                        started_at: 1,
                    },
                    name: format!("p{pid}"),
                    executable: None,
                    usage: Usage::default(),
                })
                .collect(),
            protected: false,
        }
    }

    #[test]
    fn seules_les_applications_depliees_montrent_leurs_processus() {
        let expanded = HashSet::from([AppId("b".into())]);
        let rows = build_rows(vec![app("a", &[1, 2], 1), app("b", &[3, 4], 1)], &expanded);
        let shape: Vec<String> = rows
            .iter()
            .map(|row| match row {
                Row::App { app, expanded } => format!("{}:{expanded}", app.name),
                Row::Process(process) => process.name.clone(),
            })
            .collect();
        assert_eq!(shape, ["a:false", "b:true", "p3", "p4"]);
    }

    #[test]
    fn les_barres_se_mesurent_a_la_plus_grosse_application() {
        let maxima = column_maxima(&[app("a", &[], 10), app("b", &[], 30)]);
        assert_eq!(maxima[1], 30.0);
        assert_eq!(maxima[0], 0.0);
    }
}
