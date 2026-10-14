//! The top bar (logo, the address fields, actions), the status bar and toasts.

use egui::{Align, Color32, FontId, Layout, Rect, RichText, Sense, Stroke, StrokeKind, pos2, vec2};
use serde_json::json;

use crate::icons::{self, Icon};
use crate::theme::Tokens;
use crate::widgets::{icon_button, primary_button};
use crate::{CoverApp, Language, Screen};

/// A rounded text field in the top bar with a suggestions popup below it.
fn field(ui: &mut egui::Ui, t: &Tokens, id: &str, value: &mut String, hint: &str, width: f32, focus: &mut bool) -> egui::Response {
    let (rect, _) = ui.allocate_exact_size(vec2(width, 36.0), Sense::hover());
    ui.painter().rect_filled(rect, 10.0, t.panel);
    ui.painter().rect_stroke(rect, 10.0, Stroke::new(1.0, t.border), StrokeKind::Inside);
    let inner = Rect::from_min_max(pos2(rect.left() + 12.0, rect.top() + 4.0), pos2(rect.right() - 10.0, rect.bottom() - 4.0));
    let te = ui.put(
        inner,
        egui::TextEdit::singleline(value)
            .id(egui::Id::new(id))
            .hint_text(hint)
            .frame(egui::Frame::NONE)
            .desired_width(inner.width())
            .font(FontId::proportional(14.0)),
    );
    if *focus {
        te.request_focus();
        *focus = false;
    }
    te
}

pub fn top_bar(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens) {
    let frame = egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(14, 8));
    egui::Panel::top("top").frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            let (rect, resp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
            icons::paint(ui.painter(), rect.shrink(2.0), Icon::Logo, t.accent);
            if resp.on_hover_text("CoverCraft").clicked() {
                app.screen = Screen::Verify;
            }
            ui.label(RichText::new("CoverCraft").size(18.0).strong().color(t.ui_text));
            ui.add_space(14.0);
            let total = (ui.available_width() - 330.0).clamp(420.0, 900.0);
            let w_comune = (total * 0.3).max(130.0);
            let w_via = (total * 0.5).max(180.0);
            let w_civico = (total * 0.2).clamp(80.0, 140.0);
            // Comune, with local completion.
            let mut no_focus = false;
            let hint = app.tr("Type a comune").to_string();
            let comune = field(ui, t, "comune", &mut app.form.comune, &hint, w_comune, &mut no_focus);
            let hits = if comune.has_focus() { app.comune_hits() } else { Vec::new() };
            let exact = hits.iter().any(|c| c.name == core_norm(&app.form.comune));
            if !hits.is_empty() && !exact {
                let popup = egui::Popup::from_response(&comune).open(true).align(egui::RectAlign::BOTTOM_START).show(|ui| {
                    ui.set_min_width(w_comune);
                    for c in &hits {
                        if ui.selectable_label(false, format!("{} ({})", c.name, c.provincia)).clicked() {
                            app.form.comune = c.name.clone();
                            app.form.focus_via = true;
                        }
                    }
                });
                let _ = popup;
            }
            // Street, with suggestions from the database.
            let hint = app.tr("Type a street").to_string();
            let mut focus_via = app.form.focus_via;
            let via = field(ui, t, "via", &mut app.form.via, &hint, w_via, &mut focus_via);
            app.form.focus_via = focus_via;
            let streets = app.session.state().streets.clone();
            let picked = !app.form.street_egon.is_empty();
            if via.has_focus() && !streets.is_empty() && !picked && app.form.via.trim().chars().count() >= 2 {
                egui::Popup::from_response(&via).open(true).align(egui::RectAlign::BOTTOM_START).show(|ui| {
                    ui.set_min_width(w_via);
                    for s in &streets {
                        if ui.selectable_label(false, s.display()).clicked() {
                            app.form.via = s.display();
                            app.form.sent_query = app.form.via.trim().to_string();
                            app.form.street_egon = s.egon.clone();
                            app.form.focus_civico = true;
                            app.run("civico.list", json!({"comune": app.form.comune.trim(), "street_egon": s.egon, "limit": 300}));
                        }
                    }
                });
            }
            // Number, with the street's civici once a street was picked.
            let hint = app.tr("e.g. 10, 10/A").to_string();
            let mut focus_civico = app.form.focus_civico;
            let civico = field(ui, t, "civico", &mut app.form.civico, &hint, w_civico, &mut focus_civico);
            app.form.focus_civico = focus_civico;
            let civici = app.session.state().civici.clone();
            if civico.has_focus() && picked && !app.picking && !civici.is_empty() {
                let typed = crate::core::address::normalize_civico(&app.form.civico);
                let shown: Vec<_> = civici.iter().filter(|c| typed.is_empty() || c.civico.starts_with(&typed)).take(14).collect();
                if !shown.is_empty() && !(shown.len() == 1 && shown[0].civico == typed) {
                    egui::Popup::from_response(&civico).open(true).align(egui::RectAlign::BOTTOM_START).show(|ui| {
                        ui.set_min_width(w_civico);
                        for c in shown {
                            if ui.selectable_label(false, &c.civico).clicked() {
                                app.form.civico = c.civico.clone();
                                app.verify_egon(&c.egon);
                            }
                        }
                    });
                }
            }
            if primary_button(ui, t, Some(Icon::Verify), app.tr("Check coverage")).clicked() {
                app.verify();
            }
            if (!app.form.via.is_empty() || !app.form.civico.is_empty()) && icon_button(ui, t, Icon::Close, app.tr("Clear"), true, false).clicked() {
                app.form = crate::Form { comune: app.form.comune.clone(), ..Default::default() };
                app.picking = false;
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, t, Icon::Settings, app.tr("Settings"), true, app.screen == Screen::Settings).clicked() {
                    app.screen = if app.screen == Screen::Settings { Screen::Verify } else { Screen::Settings };
                }
                let (icon, tip) = if app.dark { (Icon::Sun, "Light theme") } else { (Icon::Moon, "Dark theme") };
                if icon_button(ui, t, icon, app.tr(tip), true, false).clicked() {
                    app.dark = !app.dark;
                    let theme = if app.dark { "dark" } else { "light" };
                    app.run("prefs.set", json!({"theme": theme}));
                }
                let lang = icon_button(ui, t, Icon::Globe, &format!("{}: {}", app.tr("Language"), app.language.name()), true, false);
                egui::Popup::menu(&lang).show(|ui| {
                    for l in Language::ALL {
                        if ui.selectable_label(app.language == l, l.name()).clicked() {
                            app.language = l;
                            app.run("prefs.set", json!({"language": l.code()}));
                            ui.close();
                        }
                    }
                });
                let user = app.session.user();
                if !user.is_empty() {
                    ui.label(RichText::new(&user).color(t.muted));
                }
            });
        });
    });
}

fn core_norm(s: &str) -> String {
    crate::core::address::normalize(s)
}

pub fn status_bar(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens) {
    let frame = egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(14, 4));
    egui::Panel::bottom("status").frame(frame).show(ui, |ui| {
        ui.horizontal(|ui| {
            let red = Color32::from_rgb(0xe5, 0x48, 0x4d);
            let problem = app.session.problem().filter(|p| p != "connecting").map(|p| {
                app.language.fill(
                    "Server unreachable ({url}): {error}. Start it with: covercraft-cli serve",
                    &[("url", &app.session.server()), ("error", &p)],
                )
            });
            let db_error = (app.session.state().db_state == "error").then(|| app.language.tr_message(&app.session.state().db_message));
            match problem.or(db_error) {
                Some(e) => {
                    let (r, _) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                    icons::paint(ui.painter(), r, Icon::Warning, red);
                    ui.label(RichText::new(e).small().color(red));
                }
                None => {
                    let text = if app.status.is_empty() { app.tr("Ready").to_string() } else { app.status.clone() };
                    ui.label(RichText::new(text).small().color(t.muted));
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let pending = app.session.jobs_pending();
                if pending > 0 {
                    ui.add(egui::Spinner::new().size(12.0).color(t.accent));
                    ui.label(RichText::new(app.language.fill("Jobs pending: {n}", &[("n", &pending.to_string())])).small().color(t.muted));
                }
                let db = app.session.config().db.display();
                let state = app.session.state().db_state.clone();
                let color = match state.as_str() {
                    "connected" => Color32::from_rgb(0x22, 0xa0, 0x6b),
                    "error" => red,
                    _ => t.muted,
                };
                let (r, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                icons::paint(ui.painter(), r, Icon::Database, color);
                ui.label(RichText::new(db).small().color(t.muted));
            });
        });
    });
}

pub fn toasts(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens) {
    let Some((text, since)) = app.toast.clone() else { return };
    if since.elapsed() > Duration::from_secs(4) {
        app.toast = None;
        return;
    }
    ui.ctx().request_repaint_after(Duration::from_millis(250));
    let screen = ui.ctx().content_rect();
    let galley = ui.painter().layout_no_wrap(text, FontId::proportional(13.0), t.on_accent);
    let size = galley.size() + vec2(32.0, 18.0);
    let rect = Rect::from_min_size(pos2(screen.center().x - size.x / 2.0, screen.bottom() - 70.0), size);
    let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("toast")));
    painter.rect_filled(rect, 10.0, Color32::from_rgb(0x32, 0x33, 0x3a));
    painter.galley(rect.min + vec2(16.0, 9.0), galley, t.on_accent);
}

use std::time::Duration;
