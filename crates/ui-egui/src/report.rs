//! The coverage report, the history and the radio bridges.

use egui::{Align, Color32, FontId, Layout, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};
use serde_json::json;

use crate::CoverApp;
use crate::core::model::{Provider, Report, Service};
use crate::core::status::Status;
use crate::icons::{self, Icon};
use crate::theme::Tokens;
use crate::widgets::icon_text_button;

pub fn status_color(s: Status) -> Color32 {
    match s {
        Status::Covered => Color32::from_rgb(0x22, 0xa0, 0x6b),
        Status::Planned => Color32::from_rgb(0xf0, 0xa2, 0x0c),
        Status::NotCovered => Color32::from_rgb(0xe5, 0x48, 0x4d),
        Status::Unknown => Color32::from_rgb(0x9a, 0x9f, 0xae),
    }
}

/// A traffic-light dot.
pub fn light(ui: &mut egui::Ui, s: Status, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    ui.painter().circle_filled(rect.center(), size / 2.0, status_color(s));
}

fn card<R>(ui: &mut egui::Ui, t: &Tokens, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                ui.label(RichText::new(title).size(15.0).strong().color(t.ui_text));
                ui.add_space(6.0);
            }
            add(ui)
        })
        .inner
}

fn pill(ui: &mut egui::Ui, text: &str, color: Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(12.5), Color32::WHITE);
    let (rect, _) = ui.allocate_exact_size(galley.size() + vec2(16.0, 6.0), Sense::hover());
    ui.painter().rect_filled(rect, 10.0, color);
    ui.painter().galley(rect.min + vec2(8.0, 3.0), galley, Color32::WHITE);
}

fn speed(s: &Service) -> String {
    match (s.speed_down.is_empty(), s.speed_up.is_empty()) {
        (true, true) => String::new(),
        (false, true) => s.speed_down.clone(),
        (true, false) => format!("↑{}", s.speed_up),
        (false, false) => format!("{} / {}", s.speed_down, s.speed_up),
    }
}

fn service_row(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens, s: &Service) {
    let key = format!("{:?}:{}", s.provider, s.technology);
    let expanded = app.expanded.contains(&key);
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, t.hover);
    }
    let p = ui.painter();
    p.circle_filled(rect.left_center() + vec2(10.0, 0.0), 5.5, status_color(s.status));
    p.text(rect.left_center() + vec2(24.0, 0.0), egui::Align2::LEFT_CENTER, &s.technology, FontId::proportional(14.0), t.ui_text);
    let mut x = rect.left() + 190.0_f32.min(rect.width() * 0.4);
    let label = s.status.label(app.italian());
    p.text(pos2(x, rect.center().y), egui::Align2::LEFT_CENTER, label, FontId::proportional(13.0), status_color(s.status));
    x += 96.0;
    if !s.raw.is_empty() && s.raw.to_uppercase() != label.to_uppercase() {
        p.text(pos2(x, rect.center().y), egui::Align2::LEFT_CENTER, format!("[{}]", s.raw), FontId::proportional(12.0), t.muted);
    }
    let mut right: Vec<String> = Vec::new();
    let sp = speed(s);
    if !sp.is_empty() {
        right.push(sp);
    }
    if let Some(d) = s.distance_m {
        right.push(format!("{d} {}", app.tr("m")));
    }
    if !s.planned.is_empty() {
        right.push(format!("{}: {}", app.tr("Planned for"), s.planned));
    }
    let has_details = !s.details.is_empty();
    let text = right.join("  ·  ");
    let galley = p.layout_no_wrap(text, FontId::proportional(12.5), t.muted);
    p.galley(pos2(rect.right() - galley.size().x - (if has_details { 22.0 } else { 4.0 }), rect.center().y - galley.size().y / 2.0), galley, t.muted);
    if has_details {
        icons::paint(
            p,
            Rect::from_center_size(rect.right_center() - vec2(10.0, 0.0), vec2(14.0, 14.0)),
            if expanded { Icon::Chevron } else { Icon::ChevronRight },
            t.muted,
        );
        if resp.clicked() {
            if expanded {
                app.expanded.retain(|k| k != &key);
            } else {
                app.expanded.push(key.clone());
            }
        }
    }
    if expanded {
        ui.horizontal_wrapped(|ui| {
            ui.add_space(24.0);
            for (k, v) in &s.details {
                ui.label(RichText::new(format!("{k}: ")).small().color(t.muted));
                ui.label(RichText::new(v).small().color(t.ui_text));
                ui.add_space(8.0);
            }
        });
    }
}

fn provider_card(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens, r: &Report, p: Provider, missing: Option<&str>) {
    let services: Vec<Service> = r.of(p).cloned().collect();
    if services.is_empty() && missing.is_none() {
        return;
    }
    card(ui, t, p.label(), |ui| {
        if services.is_empty() {
            ui.label(RichText::new(app.tr(missing.unwrap_or_default())).small().color(t.muted));
        }
        for s in &services {
            service_row(app, ui, t, s);
        }
    });
    ui.add_space(10.0);
}

fn header(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens, r: &Report) {
    card(ui, t, "", |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(r.civico.display()).size(20.0).strong().color(t.ui_text));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| match r.best() {
                Some(b) => {
                    let text =
                        format!("{} · {} {}", b.status.label(app.italian()), b.provider.map(Provider::label).unwrap_or_default(), b.technology);
                    pill(ui, &text, status_color(b.status));
                }
                None => pill(ui, app.tr("Not covered"), status_color(Status::NotCovered)),
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(format!("{} {}", app.tr("EGON"), r.civico.egon)).small().color(t.muted));
            ui.label(RichText::new("·").small().color(t.muted));
            let coords = match r.point {
                Some(p) => {
                    let src = match r.point_source.as_str() {
                        "openfiber" => app.tr("from Open Fiber"),
                        "polyline" => app.tr("from the street"),
                        _ => "",
                    };
                    format!("{}: {:.5}, {:.5} ({src})", app.tr("Coordinates"), p.lat, p.lon)
                }
                None => format!("{}: {}", app.tr("Coordinates"), app.tr("not known")),
            };
            ui.label(RichText::new(coords).small().color(t.muted));
            ui.label(RichText::new("·").small().color(t.muted));
            ui.label(RichText::new(format!("{}: {}", app.tr("Source"), r.civico.source)).small().color(t.muted));
            ui.label(RichText::new("·").small().color(t.muted));
            ui.label(RichText::new(crate::core::format_time(r.time)).small().color(t.muted));
        });
        if !r.info.is_empty() {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                for (k, v) in &r.info {
                    ui.label(RichText::new(format!("{k}: ")).small().color(t.muted));
                    ui.label(RichText::new(v).small().color(t.ui_text));
                    ui.add_space(6.0);
                }
            });
        }
        let notes: Vec<String> = r.notes.iter().map(|n| crate::engine::text::note_text(n, app.italian())).collect();
        if !notes.is_empty() {
            ui.add_space(4.0);
            ui.label(RichText::new(notes.join(" · ")).small().color(Color32::from_rgb(0xf0, 0xa2, 0x0c)));
        }
    });
    ui.add_space(10.0);
}

fn openfiber_card(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens, r: &Report) {
    if r.openfiber.is_empty() {
        return;
    }
    let title = format!("Open Fiber · {} {}", r.openfiber.len(), app.tr("Staircases").to_lowercase());
    card(ui, t, &title, |ui| {
        egui::Grid::new("of-scale").striped(true).spacing(vec2(12.0, 4.0)).show(ui, |ui| {
            for h in [app.tr("Staircase"), app.tr("Building"), app.tr("Status"), app.tr("POP"), app.tr("RFA actual"), app.tr("RFC planned")] {
                ui.label(RichText::new(h).small().color(t.muted));
            }
            ui.end_row();
            for s in &r.openfiber {
                ui.horizontal(|ui| {
                    light(ui, s.scala_status.max(s.building_status), 10.0);
                    ui.label(RichText::new(format!("{} {}", s.row.id_scala, s.row.scala)).small());
                });
                ui.label(RichText::new(&s.building_text).small());
                ui.label(RichText::new(&s.scala_text).small());
                ui.label(RichText::new(format!("{}{}", s.row.pop, if s.xgspon { " XGS" } else { "" })).small());
                ui.label(RichText::new(&s.row.rfa_actual).small());
                ui.label(RichText::new(if s.row.rfc_planned.is_empty() { &s.row.rfa_planned } else { &s.row.rfc_planned }).small());
                ui.end_row();
            }
        });
    });
    ui.add_space(10.0);
}

/// Bridges around the address, as seen from above: rings every few km, dots by bearing.
fn radar(ui: &mut egui::Ui, t: &Tokens, r: &Report, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    let radius = size / 2.0 - 6.0;
    let farthest = r.radio.iter().filter(|h| h.in_reach).map(|h| h.distance_m).fold(0.0_f64, f64::max).max(2000.0);
    let scale_m = (farthest * 1.25).max(r.radio.first().map_or(0.0, |h| h.distance_m * 1.15));
    let ring_km = if scale_m > 40_000.0 {
        20.0
    } else if scale_m > 16_000.0 {
        10.0
    } else if scale_m > 8_000.0 {
        5.0
    } else {
        2.0
    };
    let per_m = radius as f64 / scale_m;
    let mut km = ring_km;
    while km * 1000.0 <= scale_m {
        let rr = (km * 1000.0 * per_m) as f32;
        p.circle_stroke(c, rr, Stroke::new(1.0, t.border));
        p.text(c + vec2(2.0, -rr), egui::Align2::LEFT_BOTTOM, format!("{km:.0} km"), FontId::proportional(10.0), t.muted);
        km += ring_km;
    }
    p.text(c + vec2(0.0, -radius - 2.0), egui::Align2::CENTER_BOTTOM, "N", FontId::proportional(11.0), t.muted);
    p.circle_filled(c, 4.0, t.accent);
    for h in &r.radio {
        let d = (h.distance_m * per_m) as f32;
        if d > radius {
            continue;
        }
        let a = (h.bearing_deg - 90.0).to_radians();
        let pos = c + vec2(a.cos() as f32, a.sin() as f32) * d;
        let color = if h.in_reach { Color32::from_rgb(0x22, 0xa0, 0x6b) } else { t.muted };
        p.circle_filled(pos, 5.0, color);
        p.text(pos + vec2(7.0, 0.0), egui::Align2::LEFT_CENTER, &h.bridge.name, FontId::proportional(11.0), t.ui_text);
    }
}

fn radio_card(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens, r: &Report) {
    card(ui, t, app.tr("Radio bridges"), |ui| {
        if r.radio.is_empty() {
            ui.label(
                RichText::new(app.tr(if r.point.is_none() {
                    "No coordinates: radio bridges and cells not computed"
                } else {
                    "No radio bridge with coordinates"
                }))
                .small()
                .color(t.muted),
            );
            return;
        }
        ui.horizontal_top(|ui| {
            radar(ui, t, r, 220.0);
            ui.add_space(8.0);
            ui.vertical(|ui| {
                egui::Grid::new("radio").spacing(vec2(10.0, 3.0)).show(ui, |ui| {
                    for h in &r.radio {
                        light(ui, if h.in_reach { Status::Covered } else { Status::NotCovered }, 10.0);
                        ui.label(RichText::new(&h.bridge.name).small().color(t.ui_text));
                        ui.label(RichText::new(format!("{:.1} {}", h.distance_m / 1000.0, app.tr("km"))).small());
                        ui.label(RichText::new(format!("{}° {}", h.bearing_deg.round(), h.compass)).small().color(t.muted));
                        ui.label(RichText::new(format!("{} {}", app.tr("Band"), h.bridge.band)).small().color(t.muted));
                        ui.label(RichText::new(&h.bridge.state).small().color(t.muted));
                        ui.end_row();
                    }
                });
            });
        });
    });
    ui.add_space(10.0);
}

fn cells_card(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens, r: &Report) {
    if r.cells.is_empty() {
        return;
    }
    card(ui, t, app.tr("Mobile cells"), |ui| {
        egui::Grid::new("cells").striped(true).spacing(vec2(12.0, 3.0)).show(ui, |ui| {
            for h in [app.tr("Operator"), app.tr("Radio"), app.tr("Distance"), app.tr("Range")] {
                ui.label(RichText::new(h).small().color(t.muted));
            }
            ui.end_row();
            for c in &r.cells {
                ui.horizontal(|ui| {
                    light(ui, if c.in_range { Status::Covered } else { Status::Unknown }, 10.0);
                    ui.label(RichText::new(&c.operator).small());
                });
                ui.label(RichText::new(&c.cell.radio).small());
                ui.label(RichText::new(format!("{:.0} {}", c.distance_m, app.tr("m"))).small());
                ui.label(RichText::new(format!("{} {}", c.cell.range, app.tr("m"))).small().color(t.muted));
                ui.end_row();
            }
        });
    });
}

fn candidates(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens) {
    let list = app.session.state().civici.clone();
    card(ui, t, app.tr("Choose the civico"), |ui| {
        ui.label(RichText::new(app.tr("More than one civico matches the address: pick one.")).small().color(t.muted));
        ui.add_space(6.0);
        for c in &list {
            let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 8.0, t.hover);
            }
            ui.painter().text(rect.left_center() + vec2(10.0, 0.0), egui::Align2::LEFT_CENTER, c.display(), FontId::proportional(14.0), t.ui_text);
            ui.painter().text(
                rect.right_center() - vec2(10.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                format!("EGON {} · {}", c.egon, c.source),
                FontId::proportional(12.0),
                t.muted,
            );
            if resp.clicked() {
                app.verify_egon(&c.egon);
            }
        }
    });
}

pub fn show(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin::same(18)).show(ui, |ui| {
            if app.picking {
                candidates(app, ui, t);
                return;
            }
            let Some(r) = app.session.state().report.clone() else {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
                    icons::paint(ui.painter(), rect, Icon::Verify, t.muted);
                    ui.label(RichText::new(app.tr("No verification yet")).size(18.0).color(t.ui_text));
                    ui.label(RichText::new(app.tr("Type comune, street and number above, then Check coverage.")).color(t.muted));
                });
                return;
            };
            header(app, ui, t, &r);
            let wide = ui.available_width() > 1000.0;
            if wide {
                let col = (ui.available_width() - 14.0) / 2.0;
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(col);
                        ui.set_max_width(col);
                        provider_card(app, ui, t, &r, Provider::Tim, Some("No TIM/FiberCop row for this civico"));
                        provider_card(app, ui, t, &r, Provider::Fastweb, Some("No Fastweb row for this civico"));
                        provider_card(app, ui, t, &r, Provider::Own, None);
                    });
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.set_width(col);
                        ui.set_max_width(col);
                        provider_card(app, ui, t, &r, Provider::OpenFiber, Some("No Open Fiber row for this civico"));
                        openfiber_card(app, ui, t, &r);
                        radio_card(app, ui, t, &r);
                        cells_card(app, ui, t, &r);
                    });
                });
            } else {
                provider_card(app, ui, t, &r, Provider::Tim, Some("No TIM/FiberCop row for this civico"));
                provider_card(app, ui, t, &r, Provider::OpenFiber, Some("No Open Fiber row for this civico"));
                openfiber_card(app, ui, t, &r);
                provider_card(app, ui, t, &r, Provider::Fastweb, Some("No Fastweb row for this civico"));
                provider_card(app, ui, t, &r, Provider::Own, None);
                radio_card(app, ui, t, &r);
                cells_card(app, ui, t, &r);
            }
        });
    });
}

pub fn history(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin::same(18)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(app.tr("Past verifications")).size(18.0).strong().color(t.ui_text));
                ui.add_space(12.0);
                if ui.selectable_label(app.history_all, app.tr("Everyone's")).clicked() {
                    app.history_all = true;
                    app.reload_history();
                }
                if ui.selectable_label(!app.history_all, app.tr("Mine")).clicked() {
                    app.history_all = false;
                    app.reload_history();
                }
                if icon_text_button(ui, t, Icon::Sync, app.tr("Refresh"), true).clicked() {
                    app.reload_history();
                }
            });
            ui.add_space(8.0);
            let list = app.session.state().history.clone();
            if list.is_empty() {
                ui.label(RichText::new(app.tr("No verification recorded yet")).color(t.muted));
                return;
            }
            card(ui, t, "", |ui| {
                egui::Grid::new("history").striped(true).spacing(vec2(14.0, 6.0)).show(ui, |ui| {
                    for h in [app.tr("Date"), app.tr("User"), app.tr("Summary"), ""] {
                        ui.label(RichText::new(h).small().color(t.muted));
                    }
                    ui.end_row();
                    for h in &list {
                        ui.label(RichText::new(&h.date).small());
                        ui.label(RichText::new(&h.user).small());
                        let summary = h
                            .answer
                            .get(if app.italian() { "summary_it" } else { "summary" })
                            .and_then(|s| s.as_str())
                            .unwrap_or_default()
                            .to_string();
                        let status = h.answer.get("best").and_then(|b| b.get("status")).and_then(|s| s.as_str()).unwrap_or("unknown");
                        let st = match status {
                            "covered" => Status::Covered,
                            "planned" => Status::Planned,
                            "not_covered" => Status::NotCovered,
                            _ => Status::Unknown,
                        };
                        ui.horizontal(|ui| {
                            light(ui, st, 10.0);
                            ui.label(RichText::new(summary).small());
                        });
                        if ui.small_button(app.tr("Open")).clicked() {
                            app.run("history.open", json!({"id": h.id}));
                        }
                        ui.end_row();
                    }
                });
            });
        });
    });
}

/// The bridges on a plain lat/lon plot, with the list.
pub fn bridges(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin::same(18)).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(app.tr("Radio bridges")).size(18.0).strong().color(t.ui_text));
                if icon_text_button(ui, t, Icon::Sync, app.tr("Refresh"), true).clicked() {
                    app.run("radio.list", json!({}));
                }
            });
            ui.add_space(8.0);
            let list = app.session.state().bridges.clone();
            if list.is_empty() {
                ui.label(RichText::new(app.tr("No radio bridge in the database")).color(t.muted));
                return;
            }
            let points: Vec<(crate::core::geo::Point, &crate::core::model::Bridge)> = list.iter().filter_map(|b| b.point().map(|p| (p, b))).collect();
            if points.len() >= 2 {
                card(ui, t, "", |ui| {
                    let w = ui.available_width().min(760.0);
                    let (rect, _) = ui.allocate_exact_size(vec2(w, 360.0), Sense::hover());
                    let (mut lat0, mut lat1, mut lon0, mut lon1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
                    for (p, _) in &points {
                        lat0 = lat0.min(p.lat);
                        lat1 = lat1.max(p.lat);
                        lon0 = lon0.min(p.lon);
                        lon1 = lon1.max(p.lon);
                    }
                    let (dlat, dlon) = ((lat1 - lat0).max(0.01), (lon1 - lon0).max(0.01));
                    let inner = rect.shrink(24.0);
                    let p = ui.painter();
                    p.rect_filled(rect, 8.0, t.panel);
                    for (pt, b) in &points {
                        let x = inner.left() + ((pt.lon - lon0) / dlon) as f32 * inner.width();
                        let y = inner.bottom() - ((pt.lat - lat0) / dlat) as f32 * inner.height();
                        let color = match b.band.to_ascii_uppercase().as_str() {
                            "A" => Color32::from_rgb(0x22, 0xa0, 0x6b),
                            "B" => Color32::from_rgb(0x1f, 0x9a, 0xb8),
                            "C" => Color32::from_rgb(0xf0, 0xa2, 0x0c),
                            _ => t.muted,
                        };
                        p.circle_filled(pos2(x, y), 6.0, color);
                        p.text(pos2(x + 8.0, y), egui::Align2::LEFT_CENTER, &b.name, FontId::proportional(11.0), t.ui_text);
                    }
                    p.rect_stroke(rect, 8.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
                });
                ui.add_space(10.0);
            }
            card(ui, t, "", |ui| {
                egui::Grid::new("bridges").striped(true).spacing(vec2(14.0, 4.0)).show(ui, |ui| {
                    for h in [app.tr("Name"), app.tr("Band"), app.tr("Coordinates"), app.tr("Altitude"), app.tr("State"), app.tr("Reach")] {
                        ui.label(RichText::new(h).small().color(t.muted));
                    }
                    ui.end_row();
                    for b in &list {
                        ui.label(RichText::new(&b.name).small().color(t.ui_text));
                        ui.label(RichText::new(&b.band).small());
                        ui.label(
                            RichText::new(if b.lat.is_empty() { "—".to_string() } else { format!("{}, {}", b.lat, b.lon) }).small().color(t.muted),
                        );
                        ui.label(RichText::new(&b.alt).small().color(t.muted));
                        ui.label(RichText::new(&b.state).small().color(t.muted));
                        ui.label(
                            RichText::new(format!("{:.0} {}", app.session.config().preferences.reach_m(&b.band) / 1000.0, app.tr("km")))
                                .small()
                                .color(t.muted),
                        );
                        ui.end_row();
                    }
                });
            });
        });
    });
}
