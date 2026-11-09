//! Tuile du tableau de bord : valeur globale, courbe des dernières minutes, application principale.

use domain::History;
use gpui::{IntoElement, ParentElement, PathBuilder, Styled, canvas, div, point, px, rgb};

use crate::theme;

pub struct Tile {
    pub label: &'static str,
    pub value: String,
    pub top: String,
    pub history: History,
    /// Haut de l'échelle ; `None` s'adapte au pic observé (disque, réseau).
    pub scale: Option<f64>,
}

pub fn render(tile: Tile) -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_1p5()
        .p_3()
        .bg(rgb(theme::PANEL))
        .border_1()
        .border_color(rgb(theme::BORDER))
        .rounded_lg()
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::MUTED))
                .child(tile.label),
        )
        .child(
            div()
                .font_family(theme::MONO)
                .text_size(px(20.))
                .whitespace_nowrap()
                .child(tile.value),
        )
        .child(sparkline(tile.history, tile.scale))
        .child(
            div()
                .text_xs()
                .text_color(rgb(theme::MUTED))
                .truncate()
                .child(tile.top),
        )
}

fn sparkline(history: History, scale: Option<f64>) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let count = history.values().len();
            if count < 2 {
                return;
            }
            let top = scale
                .unwrap_or_else(|| history.values().fold(0.0, f64::max))
                .max(f64::EPSILON);
            // Les valeurs récentes sont calées à droite ; la courbe se remplit vers la gauche.
            let step = bounds.size.width / (history.capacity() - 1) as f32;
            let first = history.capacity() - count;
            let mut builder = PathBuilder::stroke(px(1.5));
            for (index, value) in history.values().enumerate() {
                let height = (value / top).clamp(0.0, 1.0) as f32;
                let at = point(
                    bounds.origin.x + step * (first + index) as f32,
                    bounds.origin.y + bounds.size.height * (1.0 - height),
                );
                if index == 0 {
                    builder.move_to(at);
                } else {
                    builder.line_to(at);
                }
            }
            if let Ok(path) = builder.build() {
                window.paint_path(path, rgb(theme::ACCENT));
            }
        },
    )
    .w_full()
    .h(px(36.))
}
