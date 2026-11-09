use crate::Usage;

/// Identifie un processus de façon sûre : un PID seul peut être réattribué par l'OS.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ProcessId {
    pub pid: u32,
    pub started_at: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessSample {
    pub id: ProcessId,
    pub name: String,
    /// Chemin de l'exécutable, absent quand l'OS refuse de le révéler.
    pub executable: Option<String>,
    pub usage: Usage,
}
