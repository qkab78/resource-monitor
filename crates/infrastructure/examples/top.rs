//! Affiche le classement sans interface : `cargo run -p infrastructure --example top`.

use std::thread::sleep;

use application::{Monitor, SAMPLE_INTERVAL};
use domain::{SortColumn, SortDirection, SortOrder};

fn main() {
    let probe = infrastructure::default_probe().expect("sonde");
    let mut monitor = Monitor::new(probe);
    monitor.refresh().expect("premier échantillon");
    sleep(SAMPLE_INTERVAL);
    monitor.refresh().expect("deuxième échantillon");
    sleep(SAMPLE_INTERVAL);
    let dashboard = monitor.refresh().expect("troisième échantillon");

    println!("totaux OS : {:?}", dashboard.totals);
    println!("somme des apps : {:?}", dashboard.apps_total);
    for column in [SortColumn::Memory, SortColumn::Gpu, SortColumn::Network] {
        println!("--- {column:?}");
        let order = SortOrder {
            column,
            direction: SortDirection::Descending,
        };
        for app in dashboard.visible_apps("", order).iter().take(6) {
            println!(
                "{:<28} {:>3} proc. {:?}",
                app.name,
                app.processes.len(),
                app.usage
            );
        }
    }
}
