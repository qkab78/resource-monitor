use std::mem::MaybeUninit;

/// Empreinte physique d'un processus : la mesure de la colonne « Mémoire » du Moniteur d'activité.
/// Contrairement au RSS, elle compte les pages compressées et celles parties dans le swap.
/// `None` : l'OS refuse la lecture, ou le processus n'existe plus.
pub(crate) fn phys_footprint(pid: u32) -> Option<u64> {
    let pid = i32::try_from(pid).ok()?;
    let mut info = MaybeUninit::<libc::rusage_info_v0>::uninit();
    // SAFETY : le tampon a la taille exacte de la version demandée, et n'est lu qu'en cas de succès.
    unsafe {
        let status = libc::proc_pid_rusage(pid, libc::RUSAGE_INFO_V0, info.as_mut_ptr().cast());
        (status == 0).then(|| info.assume_init().ri_phys_footprint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_l_empreinte_du_processus_courant() {
        assert!(phys_footprint(std::process::id()).unwrap_or(0) > 0);
    }

    #[test]
    fn compte_la_memoire_ecrite_par_le_processus() {
        // Les autres tests tournent en parallèle dans le même processus : le bloc est assez gros,
        // et la marge assez large, pour que leurs allocations ne faussent pas la mesure.
        const SIZE: usize = 256 * 1024 * 1024;
        let before = phys_footprint(std::process::id()).expect("avant");
        let block = std::hint::black_box(vec![1u8; SIZE]);
        let after = phys_footprint(std::process::id()).expect("après");
        drop(block);
        assert!(
            after.saturating_sub(before) >= (SIZE as u64) / 2,
            "empreinte avant {before}, après {after}"
        );
    }

    #[test]
    fn un_pid_inexistant_ne_donne_pas_de_mesure() {
        assert_eq!(phys_footprint(i32::MAX as u32), None);
        assert_eq!(phys_footprint(u32::MAX), None);
    }
}
