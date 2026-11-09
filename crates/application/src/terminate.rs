use domain::AppUsage;

use crate::{ProcessTerminator, TerminateError};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TerminationReport {
    pub terminated: usize,
    pub failed: usize,
    /// L'application est protégée : aucun processus n'a été touché.
    pub refused: bool,
}

/// Termine tous les processus d'une application, sauf si elle est protégée.
pub fn terminate_app(app: &AppUsage, terminator: &dyn ProcessTerminator) -> TerminationReport {
    if app.protected {
        return TerminationReport {
            refused: true,
            ..TerminationReport::default()
        };
    }
    let mut report = TerminationReport::default();
    for process in &app.processes {
        match terminator.terminate(process.id) {
            Ok(()) => report.terminated += 1,
            Err(TerminateError::ProcessReplaced | TerminateError::Denied) => report.failed += 1,
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{AppId, ProcessId, ProcessSample, Usage};
    use std::sync::Mutex;

    struct FakeTerminator {
        denied_pid: u32,
        calls: Mutex<Vec<u32>>,
    }

    impl ProcessTerminator for FakeTerminator {
        fn terminate(&self, process: ProcessId) -> Result<(), TerminateError> {
            self.calls.lock().unwrap().push(process.pid);
            if process.pid == self.denied_pid {
                Err(TerminateError::Denied)
            } else {
                Ok(())
            }
        }
    }

    fn app(protected: bool) -> AppUsage {
        let process = |pid| ProcessSample {
            id: ProcessId { pid, started_at: 1 },
            name: "p".into(),
            executable: None,
            usage: Usage::default(),
        };
        AppUsage {
            id: AppId("a".into()),
            name: "a".into(),
            usage: Usage::default(),
            processes: vec![process(1), process(2)],
            protected,
        }
    }

    #[test]
    fn une_application_protegee_n_est_jamais_touchee() {
        let terminator = FakeTerminator {
            denied_pid: 0,
            calls: Mutex::default(),
        };
        let report = terminate_app(&app(true), &terminator);
        assert!(report.refused);
        assert!(terminator.calls.lock().unwrap().is_empty());
    }

    #[test]
    fn les_echecs_partiels_sont_rapportes() {
        let terminator = FakeTerminator {
            denied_pid: 2,
            calls: Mutex::default(),
        };
        let report = terminate_app(&app(false), &terminator);
        assert_eq!(report.terminated, 1);
        assert_eq!(report.failed, 1);
    }
}
