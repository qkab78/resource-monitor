use std::mem::MaybeUninit;
use std::sync::OnceLock;

use application::MemoryBreakdown;

/// Part de la mémoire utilisée qui n'appartient à aucune application :
/// pages verrouillées (« wired », surtout le noyau) et pages occupées par le compresseur.
/// `None` : l'OS n'a pas répondu.
pub(crate) fn memory_breakdown() -> Option<MemoryBreakdown> {
    let page = u64::try_from(unsafe { libc::sysconf(libc::_SC_PAGESIZE) }).ok()?;
    let mut stats = MaybeUninit::<libc::vm_statistics64>::uninit();
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY : `count` annonce la taille exacte du tampon, qui n'est lu qu'en cas de succès.
    let stats = unsafe {
        let status = libc::host_statistics64(
            host(),
            libc::HOST_VM_INFO64,
            stats.as_mut_ptr().cast(),
            &mut count,
        );
        (status == libc::KERN_SUCCESS).then(|| stats.assume_init())?
    };
    Some(MemoryBreakdown {
        wired_bytes: u64::from(stats.wire_count) * page,
        compressor_bytes: u64::from(stats.compressor_page_count) * page,
    })
}

/// Chaque appel à `mach_host_self` ajoute une référence au port : on le demande une seule fois.
fn host() -> libc::mach_port_t {
    static HOST: OnceLock<libc::mach_port_t> = OnceLock::new();
    // `libc` renvoie vers le crate `mach2`, qu'on évite d'ajouter pour un seul appel.
    #[allow(deprecated)]
    *HOST.get_or_init(|| unsafe { libc::mach_host_self() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_repartition_tient_dans_la_memoire_installee() {
        let breakdown = memory_breakdown().expect("statistiques de la mémoire virtuelle");
        let mut size = 0u64;
        let mut len = size_of::<u64>();
        // SAFETY : `hw.memsize` est un entier de 64 bits.
        let status = unsafe {
            libc::sysctlbyname(
                c"hw.memsize".as_ptr(),
                (&raw mut size).cast(),
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        assert_eq!(status, 0);
        assert!(breakdown.wired_bytes > 0);
        assert!(breakdown.wired_bytes + breakdown.compressor_bytes <= size);
    }
}
