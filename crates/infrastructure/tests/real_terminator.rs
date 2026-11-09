//! Exerce la vraie fermeture sur un processus enfant créé par le test.

use std::process::{Child, Command};
use std::thread::sleep;
use std::time::{Duration, Instant};

use application::TerminateError;
use domain::ProcessId;

fn spawn_child() -> Child {
    let (program, args): (&str, &[&str]) = if cfg!(windows) {
        ("ping", &["-n", "60", "127.0.0.1"])
    } else {
        ("sleep", &["60"])
    };
    Command::new(program)
        .args(args)
        .spawn()
        .expect("processus enfant")
}

fn identify(pid: u32) -> ProcessId {
    let mut probe = infrastructure::default_probe().expect("sonde");
    probe
        .sample()
        .expect("échantillon")
        .processes
        .into_iter()
        .find(|process| process.id.pid == pid)
        .expect("l'enfant doit être listé")
        .id
}

#[test]
fn termine_le_bon_processus() {
    let mut child = spawn_child();
    let id = identify(child.id());

    infrastructure::default_terminator()
        .terminate(id)
        .expect("fermeture");

    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().expect("état de l'enfant").is_none() {
        assert!(Instant::now() < deadline, "l'enfant aurait dû s'arrêter");
        sleep(Duration::from_millis(20));
    }
}

#[test]
fn refuse_un_pid_reattribue_a_un_autre_processus() {
    let mut child = spawn_child();
    let id = identify(child.id());
    let stale = ProcessId {
        started_at: id.started_at + 1,
        ..id
    };

    let result = infrastructure::default_terminator().terminate(stale);

    assert_eq!(result, Err(TerminateError::ProcessReplaced));
    assert!(
        child.try_wait().expect("état de l'enfant").is_none(),
        "l'enfant doit survivre"
    );
    child.kill().expect("nettoyage");
    child.wait().expect("nettoyage");
}
