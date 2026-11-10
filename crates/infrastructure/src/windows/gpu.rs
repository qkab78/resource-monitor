use std::collections::HashMap;

#[cfg(windows)]
use crate::sysinfo_probe::RateSource;

/// Utilisation GPU par processus, lue sur le compteur `\GPU Engine(*)\Utilization Percentage`,
/// celui qu'utilise le Gestionnaire des tâches.
#[cfg(windows)]
pub(crate) struct PdhGpuSource {
    query: Option<pdh::Query>,
    total: Option<f64>,
}

#[cfg(windows)]
impl Default for PdhGpuSource {
    fn default() -> Self {
        Self {
            query: pdh::Query::open(r"\GPU Engine(*)\Utilization Percentage"),
            total: None,
        }
    }
}

#[cfg(windows)]
impl RateSource for PdhGpuSource {
    fn sample(&mut self) -> Option<HashMap<u32, f64>> {
        self.total = None;
        let instances = self.query.as_mut()?.collect()?;
        let (rates, total) = aggregate(
            instances
                .iter()
                .map(|(name, value)| (name.as_str(), *value)),
        );
        self.total = total;
        Some(rates)
    }

    fn total(&self) -> Option<f64> {
        self.total
    }
}

/// Une instance se nomme `pid_1234_luid_0x0_0xD1B1_phys_0_eng_3_engtype_3D` :
/// on en tire le PID et le moteur (adaptateur, GPU physique, numéro).
fn parse_instance(name: &str) -> Option<(u32, &str)> {
    let (pid, rest) = name.strip_prefix("pid_")?.split_once("_luid_")?;
    let engine = rest
        .split_once("_engtype_")
        .map_or(rest, |(engine, _)| engine);
    Some((pid.parse().ok()?, engine))
}

/// Comme le Gestionnaire des tâches : un processus vaut son moteur le plus chargé,
/// et le GPU vaut le moteur le plus chargé tous processus confondus.
fn aggregate<'a>(
    instances: impl IntoIterator<Item = (&'a str, f64)>,
) -> (HashMap<u32, f64>, Option<f64>) {
    let mut per_process: HashMap<(u32, &str), f64> = HashMap::new();
    let mut per_engine: HashMap<&str, f64> = HashMap::new();
    for (name, value) in instances {
        let Some((pid, engine)) = parse_instance(name) else {
            continue;
        };
        *per_process.entry((pid, engine)).or_default() += value;
        *per_engine.entry(engine).or_default() += value;
    }

    let mut rates: HashMap<u32, f64> = HashMap::new();
    for ((pid, _), value) in per_process {
        let rate = rates.entry(pid).or_default();
        *rate = rate.max(value.min(100.0));
    }
    let total = per_engine
        .into_values()
        .reduce(f64::max)
        .map(|value| value.min(100.0));
    (rates, total)
}

#[cfg(windows)]
mod pdh {
    use std::ptr;

    use windows_sys::Win32::System::Performance::{
        PDH_CSTATUS_NEW_DATA, PDH_CSTATUS_VALID_DATA, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE,
        PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PDH_NO_DATA, PdhAddEnglishCounterW, PdhCloseQuery,
        PdhCollectQueryData, PdhGetFormattedCounterArrayW, PdhOpenQueryW,
    };

    const SUCCESS: u32 = 0;

    /// Requête PDH sur un compteur à instances multiples (`*`).
    pub(super) struct Query {
        query: PDH_HQUERY,
        counter: PDH_HCOUNTER,
    }

    // SAFETY: les handles PDH ne sont pas liés à un thread, et `&mut self` empêche tout accès concurrent.
    unsafe impl Send for Query {}

    impl Query {
        /// `None` si le compteur n'existe pas, par exemple sans pilote GPU récent.
        pub(super) fn open(path: &str) -> Option<Self> {
            let path: Vec<u16> = path.encode_utf16().chain([0]).collect();
            let mut query = ptr::null_mut();
            // SAFETY: pointeurs valides pendant l'appel ; la requête est fermée par `Drop`.
            if unsafe { PdhOpenQueryW(ptr::null(), 0, &mut query) } != SUCCESS {
                return None;
            }
            let mut counter = ptr::null_mut();
            // Nom anglais : le nom localisé changerait selon la langue de Windows.
            let added = unsafe { PdhAddEnglishCounterW(query, path.as_ptr(), 0, &mut counter) };
            let query = Self { query, counter };
            if added != SUCCESS {
                return None;
            }
            // Un pourcentage se calcule entre deux collectes : la première sert de référence.
            unsafe { PdhCollectQueryData(query.query) };
            Some(query)
        }

        /// Valeur de chaque instance depuis la collecte précédente.
        pub(super) fn collect(&mut self) -> Option<Vec<(String, f64)>> {
            // SAFETY: la requête est ouverte tant que `self` existe.
            if unsafe { PdhCollectQueryData(self.query) } != SUCCESS {
                return None;
            }
            // Des processus peuvent apparaître entre la mesure de la taille et la lecture.
            for _ in 0..3 {
                let mut size = 0u32;
                let mut count = 0u32;
                let status = unsafe {
                    PdhGetFormattedCounterArrayW(
                        self.counter,
                        PDH_FMT_DOUBLE,
                        &mut size,
                        &mut count,
                        ptr::null_mut(),
                    )
                };
                // Aucune instance (session sans GPU, pilote réinitialisé) : GPU au repos, pas inconnu.
                if status == SUCCESS || status == PDH_NO_DATA {
                    return Some(Vec::new());
                }
                if status != PDH_MORE_DATA {
                    return None;
                }

                // Le tampon contient les éléments, suivis des noms d'instance qu'ils pointent.
                let item_size = size_of::<PDH_FMT_COUNTERVALUE_ITEM_W>();
                let mut buffer = vec![
                    PDH_FMT_COUNTERVALUE_ITEM_W::default();
                    (size as usize).div_ceil(item_size)
                ];
                let mut size = (buffer.len() * item_size) as u32;
                let status = unsafe {
                    PdhGetFormattedCounterArrayW(
                        self.counter,
                        PDH_FMT_DOUBLE,
                        &mut size,
                        &mut count,
                        buffer.as_mut_ptr(),
                    )
                };
                if status == PDH_MORE_DATA {
                    continue;
                }
                if status != SUCCESS {
                    return None;
                }
                let values = buffer[..count as usize]
                    .iter()
                    .filter(|item| {
                        matches!(
                            item.FmtValue.CStatus,
                            PDH_CSTATUS_VALID_DATA | PDH_CSTATUS_NEW_DATA
                        )
                    })
                    // SAFETY: `szName` pointe dans `buffer`, toujours vivant ; format demandé : double.
                    .map(|item| unsafe {
                        (
                            wide_to_string(item.szName),
                            item.FmtValue.Anonymous.doubleValue,
                        )
                    })
                    .collect();
                return Some(values);
            }
            None
        }
    }

    impl Drop for Query {
        fn drop(&mut self) {
            // SAFETY: fermée une seule fois ; ferme aussi le compteur.
            unsafe { PdhCloseQuery(self.query) };
        }
    }

    /// SAFETY: `text` est nul ou pointe sur une chaîne UTF-16 terminée par 0.
    unsafe fn wide_to_string(text: *const u16) -> String {
        if text.is_null() {
            return String::new();
        }
        let mut len = 0;
        while unsafe { *text.add(len) } != 0 {
            len += 1;
        }
        String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, len) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_le_pid_et_le_moteur_d_une_instance() {
        assert_eq!(
            parse_instance("pid_1234_luid_0x00000000_0x0000D1B1_phys_0_eng_3_engtype_3D"),
            Some((1234, "0x00000000_0x0000D1B1_phys_0_eng_3"))
        );
        assert_eq!(parse_instance("_Total"), None);
        assert_eq!(parse_instance("pid_abc_luid_0x0"), None);
    }

    #[test]
    fn un_processus_vaut_son_moteur_le_plus_charge() {
        let (rates, total) = aggregate([
            ("pid_1_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 30.0),
            ("pid_1_luid_0x0_0x1_phys_0_eng_1_engtype_Copy", 5.0),
            ("pid_2_luid_0x0_0x1_phys_0_eng_0_engtype_3D", 20.0),
            ("pid_2_luid_0x0_0x1_phys_0_eng_2_engtype_VideoDecode", 40.0),
        ]);
        assert_eq!(rates[&1], 30.0);
        assert_eq!(rates[&2], 40.0);
        assert_eq!(total, Some(50.0), "le moteur 3D cumule 30 + 20");
    }

    #[test]
    fn sans_instance_le_total_est_inconnu() {
        let (rates, total) = aggregate([]);
        assert!(rates.is_empty());
        assert_eq!(total, None);
    }
}
