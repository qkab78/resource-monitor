use application::{ProcessTerminator, TerminateError};
use domain::ProcessId;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, Signal, System};

pub(crate) struct SysinfoTerminator;

impl ProcessTerminator for SysinfoTerminator {
    fn terminate(&self, process: ProcessId) -> Result<(), TerminateError> {
        let pid = Pid::from_u32(process.pid);
        let mut system = System::new().map_err(|_| TerminateError::Denied)?;
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing(),
        );
        let Some(found) = system.process(pid) else {
            return Ok(());
        };
        if found.start_time() != process.started_at {
            return Err(TerminateError::ProcessReplaced);
        }
        // Arrêt propre quand l'OS le permet ; Windows ne connaît que l'arrêt forcé.
        let stopped = found
            .kill_with(Signal::Term)
            .unwrap_or_else(|| found.kill());
        if stopped {
            Ok(())
        } else {
            Err(TerminateError::Denied)
        }
    }
}
