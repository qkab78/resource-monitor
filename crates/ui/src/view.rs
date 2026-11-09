use std::collections::HashSet;
use std::ops::Range;
use std::sync::{Arc, Mutex};

use application::{Dashboard, Monitor, ProcessTerminator, SAMPLE_INTERVAL, terminate_app};
use domain::{AppId, History, SortColumn, SortDirection, SortOrder};
use gpui::{
    AppContext as _, Context, FocusHandle, Focusable, KeyDownEvent, PromptLevel, Render, Task,
    Window, div, prelude::*, px, relative, rgb, uniform_list,
};

use crate::format;
use crate::rows::{METRIC_COLUMNS, Row, build_rows, column_maxima, metric_text, metric_value};
use crate::theme;
use crate::tile::{self, Tile};

const ROW_HEIGHT: f32 = 44.;
const METRIC_WIDTH: f32 = 92.;
const ACTION_WIDTH: f32 = 108.;
const CHEVRON_WIDTH: f32 = 44.;

pub struct DashboardView {
    dashboard: Option<Dashboard>,
    filter: String,
    order: SortOrder,
    expanded: HashSet<AppId>,
    notice: Option<String>,
    rows: Vec<Row>,
    maxima: [f64; METRIC_COLUMNS.len()],
    terminator: Arc<dyn ProcessTerminator>,
    focus: FocusHandle,
    _poller: Task<()>,
}

impl DashboardView {
    pub fn new(
        monitor: Monitor,
        terminator: Box<dyn ProcessTerminator>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus = cx.focus_handle();
        window.focus(&focus);
        Self {
            dashboard: None,
            filter: String::new(),
            order: SortOrder::default(),
            expanded: HashSet::new(),
            notice: None,
            rows: Vec::new(),
            maxima: Default::default(),
            terminator: Arc::from(terminator),
            focus,
            _poller: Self::start_polling(monitor, cx),
        }
    }

    /// La collecte lance des sous-processus : elle tourne hors du fil de l'interface.
    fn start_polling(monitor: Monitor, cx: &mut Context<Self>) -> Task<()> {
        let monitor = Arc::new(Mutex::new(monitor));
        cx.spawn(async move |this, cx| {
            loop {
                let monitor = monitor.clone();
                let result = cx
                    .background_spawn(async move {
                        monitor.lock().expect("collecte interrompue").refresh()
                    })
                    .await;
                let updated = this.update(cx, |view, cx| {
                    match result {
                        Ok(dashboard) => view.dashboard = Some(dashboard),
                        Err(error) => {
                            view.notice = Some(format!("Mesure impossible : {}", error.0));
                        }
                    }
                    cx.notify();
                });
                if updated.is_err() {
                    break;
                }
                cx.background_executor().timer(SAMPLE_INTERVAL).await;
            }
        })
    }

    fn on_key(&mut self, event: &KeyDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.platform || keystroke.modifiers.control {
            return;
        }
        match keystroke.key.as_str() {
            "backspace" => {
                self.filter.pop();
            }
            "escape" => self.filter.clear(),
            _ => match keystroke.key_char.as_deref() {
                Some(text) if !text.chars().any(char::is_control) => self.filter.push_str(text),
                _ => return,
            },
        }
        cx.notify();
    }

    fn confirm_termination(&mut self, id: AppId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(app) = self
            .dashboard
            .as_ref()
            .and_then(|dashboard| dashboard.app(&id))
        else {
            return;
        };
        let title = format!("Terminer {} ?", app.name);
        let detail = format!(
            "{} processus seront fermés. Les modifications non enregistrées peuvent être perdues.",
            app.processes.len()
        );
        let answer = window.prompt(
            PromptLevel::Warning,
            &title,
            Some(&detail),
            &["Annuler", "Terminer"],
            cx,
        );
        cx.spawn(async move |this, cx| {
            if answer.await != Ok(1) {
                return;
            }
            // On repart de la dernière mesure : l'application a pu lancer d'autres processus.
            let Ok(Some((app, terminator))) = this.update(cx, |view, _| {
                let app = view.dashboard.as_ref()?.app(&id)?.clone();
                Some((app, view.terminator.clone()))
            }) else {
                return;
            };
            let name = app.name.clone();
            let report = cx
                .background_spawn(async move { terminate_app(&app, terminator.as_ref()) })
                .await;
            this.update(cx, |view, cx| {
                view.notice = Some(if report.refused {
                    format!("{name} est protégée et n'a pas été terminée.")
                } else if report.failed > 0 {
                    format!(
                        "{name} : {} processus terminés, {} refusés par le système.",
                        report.terminated, report.failed
                    )
                } else {
                    format!("{name} a reçu la demande d'arrêt.")
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn tiles(&self, dashboard: &Dashboard) -> Vec<Tile> {
        let totals = &dashboard.totals;
        let history = &dashboard.history;
        let top = |column: SortColumn| {
            dashboard
                .top_app(column)
                .filter(|app| metric_value(column, &app.usage).is_some())
                .map_or(format::UNAVAILABLE.to_string(), |app| {
                    format!(
                        "Surtout : {} ({})",
                        app.name,
                        metric_text(column, &app.usage)
                    )
                })
        };
        let tile = |label, value, column, history: &History, scale| Tile {
            label,
            value,
            detail: None,
            top: top(column),
            history: history.clone(),
            scale,
        };
        vec![
            tile(
                "Processeur",
                format::percent(Some(totals.cpu_percent)),
                SortColumn::Cpu,
                &history.cpu_percent,
                Some(100.),
            ),
            Tile {
                detail: totals.memory_breakdown.map(format::memory_breakdown),
                ..tile(
                    "Mémoire",
                    format!(
                        "{} / {}",
                        format::bytes(Some(totals.memory_used_bytes)),
                        format::bytes(Some(totals.memory_total_bytes))
                    ),
                    SortColumn::Memory,
                    &history.memory_percent,
                    Some(100.),
                )
            },
            tile(
                "GPU",
                format::percent(totals.gpu_percent),
                SortColumn::Gpu,
                &history.gpu_percent,
                Some(100.),
            ),
            tile(
                "Disque",
                format::rate(dashboard.apps_total.disk_bytes_per_sec),
                SortColumn::Disk,
                &history.disk_bytes_per_sec,
                None,
            ),
            tile(
                "Réseau",
                format::rate(dashboard.apps_total.network_bytes_per_sec),
                SortColumn::Network,
                &history.network_bytes_per_sec,
                None,
            ),
        ]
    }

    fn render_search(&self, count: usize, window: &Window) -> impl IntoElement + use<> {
        let focused = self.focus.is_focused(window);
        let (text, color) = if self.filter.is_empty() {
            ("Tapez le nom d'une application".to_string(), theme::MUTED)
        } else {
            (
                format!("{}{}", self.filter, if focused { "▏" } else { "" }),
                theme::TEXT,
            )
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2p5()
                    .child(div().text_color(rgb(theme::MUTED)).child("Rechercher"))
                    .child(
                        div()
                            .w(px(340.))
                            .h(px(36.))
                            .px_3()
                            .flex()
                            .items_center()
                            .bg(rgb(theme::PANEL))
                            .border_1()
                            .border_color(rgb(if focused {
                                theme::ACCENT
                            } else {
                                theme::BORDER
                            }))
                            .rounded_md()
                            .text_color(rgb(color))
                            .truncate()
                            .child(text),
                    ),
            )
            .child(div().text_color(rgb(theme::MUTED)).child(match count {
                0 | 1 => format!("{count} application"),
                _ => format!("{count} applications"),
            }))
    }

    fn render_header(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let header_cell = |column: SortColumn, label: &'static str, cx: &mut Context<Self>| {
            let active = self.order.column == column;
            let arrow = match (active, self.order.direction) {
                (false, _) => "",
                (true, SortDirection::Descending) => " ↓",
                (true, SortDirection::Ascending) => " ↑",
            };
            div()
                .id(label)
                .h(px(ROW_HEIGHT))
                .flex()
                .items_center()
                .px_3()
                .cursor_pointer()
                .text_color(rgb(if active {
                    theme::ACCENT
                } else {
                    theme::TEXT_SOFT
                }))
                .on_click(cx.listener(move |view, _, _, cx| {
                    view.order = view.order.toggled(column);
                    cx.notify();
                }))
                .child(format!("{label}{arrow}"))
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .child(header_cell(SortColumn::Name, "Application", cx)),
            )
            .children(METRIC_COLUMNS.map(|(column, label)| {
                div()
                    .w(px(METRIC_WIDTH))
                    .flex()
                    .justify_end()
                    .child(header_cell(column, label, cx))
            }))
            .child(div().w(px(ACTION_WIDTH)))
    }

    fn render_rows(&self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<gpui::AnyElement> {
        range
            .filter_map(|index| Some(self.render_row(index, self.rows.get(index)?, cx)))
            .collect()
    }

    fn render_row(&self, index: usize, row: &Row, cx: &mut Context<Self>) -> gpui::AnyElement {
        let name = match row {
            Row::App { app, expanded } => {
                let toggled = app.id.clone();
                let count = app.processes.len();
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .id(("toggle", index))
                            .w(px(CHEVRON_WIDTH))
                            .h(px(ROW_HEIGHT))
                            .flex()
                            .flex_none()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .text_color(rgb(theme::MUTED))
                            .on_click(cx.listener(move |view, _, _, cx| {
                                if !view.expanded.remove(&toggled) {
                                    view.expanded.insert(toggled.clone());
                                }
                                cx.notify();
                            }))
                            .child(if *expanded { "▾" } else { "▸" }),
                    )
                    .child(div().min_w_0().truncate().child(app.name.clone()))
                    .when(count > 1, |this| {
                        this.child(meta_label(format!("{count} proc.")))
                    })
            }
            Row::Process(process) => div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .pl(px(CHEVRON_WIDTH))
                .text_color(rgb(theme::TEXT_SOFT))
                .child(div().min_w_0().truncate().child(process.name.clone()))
                .child(meta_label(format!("pid {}", process.id.pid))),
        };

        let action = match row {
            Row::App { app, .. } if app.protected => div()
                .pr_2p5()
                .text_xs()
                .text_color(rgb(theme::MUTED))
                .child("Protégée")
                .into_any_element(),
            Row::App { app, .. } => {
                let id = app.id.clone();
                div()
                    .id(("terminate", index))
                    .h(px(32.))
                    .mr_2()
                    .px_2p5()
                    .flex()
                    .items_center()
                    .border_1()
                    .border_color(rgb(theme::BORDER_STRONG))
                    .rounded_md()
                    .text_xs()
                    .cursor_pointer()
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.confirm_termination(id.clone(), window, cx);
                    }))
                    .child("Terminer")
                    .into_any_element()
            }
            Row::Process(_) => div().into_any_element(),
        };

        let usage = row.usage();
        div()
            .id(("row", index))
            .w_full()
            .h(px(ROW_HEIGHT))
            .flex()
            .items_center()
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .when(matches!(row, Row::Process(_)), |this| {
                this.bg(rgb(theme::NESTED))
            })
            .child(name)
            .children(
                METRIC_COLUMNS
                    .iter()
                    .enumerate()
                    .map(|(slot, (column, _))| {
                        let fraction = metric_value(*column, usage)
                            .filter(|_| self.maxima[slot] > 0.0)
                            .map_or(0.0, |value| (value / self.maxima[slot]).clamp(0.0, 1.0));
                        let bar = if self.order.column == *column {
                            theme::ACCENT
                        } else {
                            theme::BAR
                        };
                        div()
                            .w(px(METRIC_WIDTH))
                            .px_3()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .font_family(theme::MONO)
                                    .text_xs()
                                    .text_right()
                                    .whitespace_nowrap()
                                    .child(metric_text(*column, usage)),
                            )
                            .child(
                                div()
                                    .h(px(2.))
                                    .w_full()
                                    .flex()
                                    .justify_end()
                                    .bg(rgb(theme::BORDER))
                                    .child(
                                        div().h_full().w(relative(fraction as f32)).bg(rgb(bar)),
                                    ),
                            )
                    }),
            )
            .child(div().w(px(ACTION_WIDTH)).flex().justify_end().child(action))
            .into_any_element()
    }
}

fn meta_label(text: String) -> impl IntoElement {
    div()
        .flex_none()
        .ml_2()
        .font_family(theme::MONO)
        .text_xs()
        .text_color(rgb(theme::MUTED))
        .child(text)
}

impl Focusable for DashboardView {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for DashboardView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let dashboard = self.dashboard.clone();
        let apps = dashboard
            .as_ref()
            .map(|dashboard| dashboard.visible_apps(&self.filter, self.order))
            .unwrap_or_default();
        let count = apps.len();
        self.maxima = column_maxima(&apps);
        self.rows = build_rows(apps, &self.expanded);
        let row_count = self.rows.len();

        let tiles = dashboard
            .as_ref()
            .map(|dashboard| self.tiles(dashboard))
            .unwrap_or_default();
        let search = self.render_search(count, window);
        let header = self.render_header(cx);

        let body = if row_count > 0 {
            uniform_list(
                "apps",
                row_count,
                cx.processor(|view, range: Range<usize>, _, cx| view.render_rows(range, cx)),
            )
            .flex_1()
            .into_any_element()
        } else {
            let message = match (&dashboard, self.filter.is_empty()) {
                (None, _) => "Première mesure en cours…",
                (Some(_), false) => "Aucune application ne correspond à cette recherche.",
                (Some(_), true) => "Aucune application détectée.",
            };
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(rgb(theme::MUTED))
                .child(message)
                .into_any_element()
        };

        div()
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::on_key))
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_5()
            .bg(rgb(theme::GROUND))
            .text_color(rgb(theme::TEXT))
            .text_size(px(13.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_size(px(18.)).child("Resource Monitor"))
                    .child(div().text_color(rgb(theme::MUTED)).child(format!(
                        "Mise à jour toutes les {} s · 5 min d'historique",
                        SAMPLE_INTERVAL.as_secs()
                    ))),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .gap_3()
                    .children(tiles.into_iter().map(tile::render)),
            )
            .child(search)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .bg(rgb(theme::PANEL))
                    .border_1()
                    .border_color(rgb(theme::BORDER))
                    .rounded_lg()
                    .overflow_hidden()
                    .child(header)
                    .child(body),
            )
            .child(div().text_xs().text_color(rgb(theme::MUTED)).child(
                "« — » : mesure indisponible, ce qui ne veut pas dire 0. La RAM d'une application additionne ses processus : selon l'OS, elle inclut la mémoire compressée ou compte plusieurs fois la mémoire partagée. La somme des lignes peut donc différer du total.",
            ))
            .when_some(self.notice.clone(), |this, notice| {
                this.child(div().text_color(rgb(theme::ACCENT)).child(notice))
            })
    }
}
