//! The navigation on the left: verification, history, radio bridges, settings.

use egui::{Align, FontId, Layout, Rect, RichText, Sense, vec2};

use crate::icons::{self, Icon};
use crate::theme::Tokens;
use crate::{CoverApp, Screen};

fn item(ui: &mut egui::Ui, t: &Tokens, icon: Icon, label: &str, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 18.0, t.active);
    } else if resp.hovered() {
        p.rect_filled(rect, 18.0, t.hover);
    }
    let color = if selected { t.accent } else { t.ui_text };
    icons::paint(p, Rect::from_center_size(rect.left_center() + vec2(24.0, 0.0), vec2(18.0, 18.0)), icon, color);
    p.text(rect.left_center() + vec2(46.0, 0.0), egui::Align2::LEFT_CENTER, label, FontId::proportional(14.0), color);
    resp
}

pub fn show(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens) {
    let frame = egui::Frame::NONE.fill(t.chrome).inner_margin(egui::Margin::symmetric(10, 10));
    egui::Panel::left("nav").exact_size(220.0).resizable(false).frame(frame).show(ui, |ui| {
        ui.with_layout(Layout::top_down(Align::Min), |ui| {
            if item(ui, t, Icon::Verify, app.tr("Verification"), app.screen == Screen::Verify).clicked() {
                app.screen = Screen::Verify;
            }
            if item(ui, t, Icon::History, app.tr("History"), app.screen == Screen::History).clicked() {
                app.screen = Screen::History;
                app.reload_history();
            }
            if item(ui, t, Icon::Antenna, app.tr("Radio bridges"), app.screen == Screen::Radio).clicked() {
                app.screen = Screen::Radio;
            }
            if item(ui, t, Icon::Settings, app.tr("Settings"), app.screen == Screen::Settings).clicked() {
                app.screen = Screen::Settings;
            }
            ui.add_space(16.0);
            if let Some(r) = &app.session.state().report {
                ui.label(RichText::new(app.tr("Best option")).small().color(t.muted));
                let (status, text) = match r.best() {
                    Some(b) => (b.status, format!("{}\n{}", b.technology, b.provider.map(|p| p.label()).unwrap_or_default())),
                    None => (crate::core::status::Status::NotCovered, app.tr("Not covered").to_string()),
                };
                ui.horizontal(|ui| {
                    crate::report::light(ui, status, 12.0);
                    ui.label(RichText::new(text).color(t.ui_text));
                });
                if let Some(h) = r.best_radio() {
                    ui.add_space(6.0);
                    ui.label(RichText::new(app.tr("Nearest bridge")).small().color(t.muted));
                    ui.label(RichText::new(format!("{} · {:.1} {}", h.bridge.name, h.distance_m / 1000.0, app.tr("km"))).color(t.ui_text));
                }
            } else {
                ui.label(RichText::new(app.tr("CoverCraft checks the connectivity available at an address from your coverage database: TIM/FiberCop, Open Fiber, Fastweb, your own network, your radio bridges and mobile cells.")).small().color(t.muted));
            }
        });
    });
}
