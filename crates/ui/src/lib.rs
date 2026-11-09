//! Interface GPUI : affiche le tableau de bord et traduit les gestes de l'utilisateur en cas d'usage.

mod format;
mod rows;
mod theme;
mod tile;
mod view;

use application::{Monitor, ProcessTerminator};
use gpui::{
    App, AppContext as _, Application, Bounds, KeyBinding, Menu, MenuItem, TitlebarOptions,
    WindowBounds, WindowOptions, actions, px, size,
};

use view::DashboardView;

actions!(resource_monitor, [Quit]);

/// Ouvre la fenêtre et bloque jusqu'à la fermeture de l'application.
pub fn run(monitor: Monitor, terminator: Box<dyn ProcessTerminator>) {
    Application::new().run(move |cx: &mut App| {
        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus(vec![Menu {
            name: "Resource Monitor".into(),
            items: vec![MenuItem::action("Quitter", Quit)],
        }]);

        let bounds = Bounds::centered(None, size(px(1280.), px(860.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(980.), px(600.))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Resource Monitor".into()),
                    ..TitlebarOptions::default()
                }),
                ..WindowOptions::default()
            },
            |window, cx| cx.new(|cx| DashboardView::new(monitor, terminator, window, cx)),
        )
        .expect("ouverture de la fenêtre");

        cx.on_window_closed(|cx| cx.quit()).detach();
        cx.activate(true);
    });
}
