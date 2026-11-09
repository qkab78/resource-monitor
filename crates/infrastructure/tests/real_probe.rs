//! Exerce la vraie sonde sur la machine qui lance les tests.

use std::process;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn la_sonde_voit_le_processus_courant() {
    let mut probe = infrastructure::default_probe().expect("sonde");
    probe.sample().expect("premier échantillon");
    sleep(Duration::from_millis(300));
    let sample = probe.sample().expect("second échantillon");

    let me = sample
        .processes
        .iter()
        .find(|candidate| candidate.id.pid == process::id())
        .expect("le processus de test doit être listé");
    assert!(me.usage.memory_bytes.unwrap_or(0) > 0);
    assert!(me.usage.cpu_percent.is_some());
    assert!(me.executable.is_some());
    assert!(sample.totals.memory_total_bytes > 0);
    assert!(sample.totals.memory_used_bytes <= sample.totals.memory_total_bytes);
}
