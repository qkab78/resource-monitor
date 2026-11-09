use std::cmp::Ordering;

use crate::{AppUsage, Usage};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortColumn {
    Name,
    Cpu,
    Memory,
    Gpu,
    Disk,
    Network,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SortOrder {
    pub column: SortColumn,
    pub direction: SortDirection,
}

impl Default for SortOrder {
    fn default() -> Self {
        Self {
            column: SortColumn::Memory,
            direction: SortDirection::Descending,
        }
    }
}

impl SortOrder {
    /// Clic sur une colonne : inverse le sens, ou la sélectionne avec son sens naturel.
    pub fn toggled(self, column: SortColumn) -> Self {
        if self.column == column {
            let direction = match self.direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
            Self { column, direction }
        } else {
            let direction = match column {
                SortColumn::Name => SortDirection::Ascending,
                _ => SortDirection::Descending,
            };
            Self { column, direction }
        }
    }

    pub fn compare_usage(self, a: &Usage, b: &Usage) -> Ordering {
        match (metric(self.column, a), metric(self.column, b)) {
            (Some(a), Some(b)) => self.apply(a.total_cmp(&b)),
            // Une mesure indisponible reste en bas, quel que soit le sens.
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }
    }

    pub fn compare_apps(self, a: &AppUsage, b: &AppUsage) -> Ordering {
        let by_name = a.name.to_lowercase().cmp(&b.name.to_lowercase());
        match self.column {
            SortColumn::Name => self.apply(by_name),
            // Départage stable : les lignes à égalité ne sautent pas à chaque rafraîchissement.
            _ => self.compare_usage(&a.usage, &b.usage).then(by_name),
        }
    }

    fn apply(self, ordering: Ordering) -> Ordering {
        match self.direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    }
}

fn metric(column: SortColumn, usage: &Usage) -> Option<f64> {
    match column {
        SortColumn::Name => None,
        SortColumn::Cpu => usage.cpu_percent,
        SortColumn::Memory => usage.memory_bytes.map(|bytes| bytes as f64),
        SortColumn::Gpu => usage.gpu_percent,
        SortColumn::Disk => usage.disk_bytes_per_sec,
        SortColumn::Network => usage.network_bytes_per_sec,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppId;

    fn app(name: &str, memory: Option<u64>) -> AppUsage {
        AppUsage {
            id: AppId(name.into()),
            name: name.into(),
            usage: Usage {
                memory_bytes: memory,
                ..Usage::default()
            },
            processes: Vec::new(),
            protected: false,
        }
    }

    fn sorted(order: SortOrder) -> Vec<String> {
        let mut apps = vec![
            app("slack", Some(5)),
            app("inconnue", None),
            app("Discord", Some(9)),
            app("Zed", Some(5)),
        ];
        apps.sort_by(|a, b| order.compare_apps(a, b));
        apps.into_iter().map(|app| app.name).collect()
    }

    #[test]
    fn tri_decroissant_par_defaut_sur_la_memoire() {
        assert_eq!(
            sorted(SortOrder::default()),
            ["Discord", "slack", "Zed", "inconnue"]
        );
    }

    #[test]
    fn les_mesures_indisponibles_restent_en_bas_dans_les_deux_sens() {
        let order = SortOrder {
            column: SortColumn::Memory,
            direction: SortDirection::Ascending,
        };
        assert_eq!(sorted(order), ["slack", "Zed", "Discord", "inconnue"]);
    }

    #[test]
    fn le_clic_sur_une_colonne_alterne_le_sens() {
        let order = SortOrder::default().toggled(SortColumn::Memory);
        assert_eq!(order.direction, SortDirection::Ascending);
        let order = order.toggled(SortColumn::Name);
        assert_eq!(order.direction, SortDirection::Ascending);
        let order = order.toggled(SortColumn::Cpu);
        assert_eq!(order.direction, SortDirection::Descending);
    }
}
