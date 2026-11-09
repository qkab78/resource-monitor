use std::collections::HashMap;

use crate::{ProcessSample, Usage};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AppId(pub String);

/// Une application : un ou plusieurs processus présentés comme un seul logiciel.
#[derive(Clone, Debug, PartialEq)]
pub struct AppUsage {
    pub id: AppId,
    pub name: String,
    pub usage: Usage,
    pub processes: Vec<ProcessSample>,
    /// Une application protégée ne peut pas être terminée depuis l'interface.
    pub protected: bool,
}

/// Regroupe les processus par application. `own_pid` protège notre propre application.
pub fn group_by_app(processes: Vec<ProcessSample>, own_pid: u32) -> Vec<AppUsage> {
    let mut apps: HashMap<AppId, AppUsage> = HashMap::new();
    for process in processes {
        let (id, name) = identify(&process);
        let protected =
            process.id.pid == own_pid || is_protected_executable(process.executable.as_deref());
        let app = apps.entry(id.clone()).or_insert_with(|| AppUsage {
            id,
            name,
            usage: Usage::default(),
            processes: Vec::new(),
            protected: false,
        });
        app.usage = app.usage + process.usage;
        app.protected |= protected;
        app.processes.push(process);
    }
    apps.into_values().collect()
}

/// Vrai pour les composants de l'OS, et quand l'exécutable est inconnu : dans le doute, on protège.
pub fn is_protected_executable(executable: Option<&str>) -> bool {
    let Some(path) = executable else {
        return true;
    };
    let path = path.replace('\\', "/").to_lowercase();
    if path.starts_with("/usr/local/") {
        return false;
    }
    [
        "/system/",
        "/usr/",
        "/bin/",
        "/sbin/",
        "/library/apple/",
        "c:/windows/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

/// Filtre insensible à la casse, sur le nom de l'application ou d'un de ses processus.
pub fn matches_filter(app: &AppUsage, filter: &str) -> bool {
    let filter = filter.trim().to_lowercase();
    filter.is_empty()
        || app.name.to_lowercase().contains(&filter)
        || app
            .processes
            .iter()
            .any(|process| process.name.to_lowercase().contains(&filter))
}

fn identify(process: &ProcessSample) -> (AppId, String) {
    let Some(path) = process.executable.as_deref() else {
        return (
            AppId(format!("name:{}", process.name.to_lowercase())),
            process.name.clone(),
        );
    };
    let components: Vec<&str> = path.split(['/', '\\']).collect();

    // macOS : le premier bundle `.app` englobe tous les helpers imbriqués.
    if let Some(index) = components
        .iter()
        .position(|part| part.to_lowercase().ends_with(".app"))
    {
        let bundle = components[index];
        let name = &bundle[..bundle.len() - ".app".len()];
        return (
            AppId(format!("bundle:{}", components[..=index].join("/"))),
            name.to_string(),
        );
    }

    // Sinon (Windows, binaires isolés) : les processus de même exécutable forment une application.
    let file = components.last().copied().unwrap_or(&process.name);
    let name = strip_exe(file);
    (
        AppId(format!("exe:{}", name.to_lowercase())),
        name.to_string(),
    )
}

fn strip_exe(file: &str) -> &str {
    match file.len().checked_sub(4) {
        Some(end) if file.is_char_boundary(end) && file[end..].eq_ignore_ascii_case(".exe") => {
            &file[..end]
        }
        _ => file,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProcessId;

    fn process(pid: u32, name: &str, executable: Option<&str>, memory: u64) -> ProcessSample {
        ProcessSample {
            id: ProcessId { pid, started_at: 1 },
            name: name.into(),
            executable: executable.map(Into::into),
            usage: Usage {
                memory_bytes: Some(memory),
                ..Usage::default()
            },
        }
    }

    fn find<'a>(apps: &'a [AppUsage], name: &str) -> &'a AppUsage {
        apps.iter().find(|app| app.name == name).unwrap()
    }

    #[test]
    fn les_helpers_macos_rejoignent_leur_application() {
        let apps = group_by_app(
            vec![
                process(
                    1,
                    "Discord",
                    Some("/Applications/Discord.app/Contents/MacOS/Discord"),
                    100,
                ),
                process(
                    2,
                    "Discord Helper (GPU)",
                    Some(
                        "/Applications/Discord.app/Contents/Frameworks/Discord Helper (GPU).app/Contents/MacOS/Discord Helper (GPU)",
                    ),
                    50,
                ),
                process(
                    3,
                    "zed",
                    Some("/Applications/Zed.app/Contents/MacOS/zed"),
                    10,
                ),
            ],
            0,
        );
        assert_eq!(apps.len(), 2);
        let discord = find(&apps, "Discord");
        assert_eq!(discord.processes.len(), 2);
        assert_eq!(discord.usage.memory_bytes, Some(150));
        assert!(!discord.protected);
    }

    #[test]
    fn les_processus_windows_de_meme_exe_sont_regroupes() {
        let path = r"C:\Users\moi\AppData\Local\Discord\app-1.0\Discord.exe";
        let apps = group_by_app(
            vec![
                process(1, "Discord.exe", Some(path), 100),
                process(2, "Discord.exe", Some(path), 50),
            ],
            0,
        );
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "Discord");
        assert_eq!(apps[0].usage.memory_bytes, Some(150));
    }

    #[test]
    fn sans_chemin_on_regroupe_par_nom_et_on_protege() {
        let apps = group_by_app(vec![process(1, "kernel_task", None, 1)], 0);
        assert_eq!(apps[0].name, "kernel_task");
        assert!(apps[0].protected);
    }

    #[test]
    fn les_composants_systeme_sont_proteges() {
        for path in [
            "/System/Library/x",
            "/usr/libexec/x",
            "C:\\Windows\\System32\\x.exe",
        ] {
            assert!(is_protected_executable(Some(path)), "{path}");
        }
        for path in [
            "/Applications/Zed.app/z",
            "/usr/local/bin/x",
            "/opt/homebrew/bin/x",
        ] {
            assert!(!is_protected_executable(Some(path)), "{path}");
        }
    }

    #[test]
    fn notre_propre_application_est_protegee() {
        let apps = group_by_app(
            vec![process(
                7,
                "resource-monitor",
                Some("/Users/moi/resource-monitor"),
                1,
            )],
            7,
        );
        assert!(apps[0].protected);
    }

    #[test]
    fn le_filtre_cherche_aussi_dans_les_processus() {
        let apps = group_by_app(
            vec![process(
                1,
                "rust-analyzer",
                Some("/Applications/Zed.app/ra"),
                1,
            )],
            0,
        );
        assert!(matches_filter(&apps[0], " ZED "));
        assert!(matches_filter(&apps[0], "analyzer"));
        assert!(matches_filter(&apps[0], ""));
        assert!(!matches_filter(&apps[0], "discord"));
    }
}
