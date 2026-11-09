#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

//! Racine de composition : le seul endroit qui connaît à la fois l'infrastructure et l'interface.

use std::process::ExitCode;

use application::Monitor;

fn main() -> ExitCode {
    match infrastructure::default_probe() {
        Ok(probe) => {
            ui::run(Monitor::new(probe), infrastructure::default_terminator());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("Impossible d'initialiser la collecte : {}", error.0);
            ExitCode::FAILURE
        }
    }
}
