//! Settings: the database connection and the preferences.

use egui::{Color32, RichText, Stroke, vec2};
use serde_json::{Value, json};

use crate::CoverApp;
use crate::theme::Tokens;
use crate::widgets::primary_button;

#[derive(Clone, Debug, Default)]
pub struct SettingsForm {
    pub loaded: bool,
    pub kind: String,
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub database: String,
    pub path: String,
    pub result: String,
    pub log_requests: bool,
    pub log_to_database: bool,
    pub reach: Vec<(String, String)>,
    pub max_bridges: String,
    pub cells: bool,
    pub cells_radius: String,
    pub max_cells: String,
    pub rules: String,
    pub geocode_url: String,
    pub tiles_url: String,
    /// The radio parameters as text: (key, label, value).
    pub radio: Vec<(&'static str, &'static str, String)>,
}

impl SettingsForm {
    fn load(&mut self, app: &CoverApp) {
        let c = app.session.config();
        self.kind = if c.db.kind.is_empty() { "demo".into() } else { c.db.kind.clone() };
        self.host = c.db.host.clone();
        self.port = c.db.port.to_string();
        self.user = c.db.user.clone();
        self.database = c.db.database.clone();
        self.path = c.db.path.clone();
        self.password.clear();
        let p = &c.preferences;
        self.log_requests = p.log_requests;
        self.log_to_database = p.log_to_database;
        self.reach =
            ["A", "B", "C", "D"].iter().map(|b| ((*b).to_string(), p.radio_reach_km.get(*b).map(|v| format!("{v}")).unwrap_or_default())).collect();
        self.max_bridges = p.max_bridges.to_string();
        self.cells = p.cells;
        self.cells_radius = p.cells_radius_km.to_string();
        self.max_cells = p.max_cells.to_string();
        self.rules = p.rules.iter().map(|r| format!("{}={}", r.value, r.status.code())).collect::<Vec<_>>().join("\n");
        self.geocode_url = p.geocode_url.clone();
        self.tiles_url = p.tiles_url.clone();
        let r = &p.radio;
        self.radio = vec![
            ("frequency_ghz", "Frequency (GHz)", r.frequency_ghz.to_string()),
            ("channel_mhz", "Channel width (MHz)", r.channel_mhz.to_string()),
            ("tx_power_dbm", "TX power (dBm)", r.tx_power_dbm.to_string()),
            ("bridge_gain_dbi", "Bridge/sector antenna gain (dBi)", r.bridge_gain_dbi.to_string()),
            ("client_gain_dbi", "Customer antenna gain (dBi)", r.client_gain_dbi.to_string()),
            ("cable_loss_db", "Cable losses per side (dB)", r.cable_loss_db.to_string()),
            ("fade_margin_db", "Fade margin (dB)", r.fade_margin_db.to_string()),
            ("min_rx_dbm", "Minimum RX for a usable link (dBm)", r.min_rx_dbm.to_string()),
            ("client_height_m", "Customer antenna height (m)", r.client_height_m.to_string()),
        ];
        self.loaded = true;
    }
}

fn section<R>(ui: &mut egui::Ui, t: &Tokens, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(t.card)
        .stroke(Stroke::new(1.0, t.border))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_width(ui.available_width().min(760.0));
            ui.label(RichText::new(title).size(16.0).strong().color(t.ui_text));
            ui.add_space(8.0);
            add(ui)
        })
        .inner
}

fn text(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str, password: bool) {
    ui.label(label);
    ui.add(egui::TextEdit::singleline(value).hint_text(hint).password(password).desired_width(320.0));
    ui.end_row();
}

pub fn show(app: &mut CoverApp, ui: &mut egui::Ui, t: &Tokens) {
    if !app.settings.loaded {
        let mut f = app.settings.clone();
        f.load(app);
        app.settings = f;
    }
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::NONE.inner_margin(egui::Margin::same(18)).show(ui, |ui| {
            let mut save_db = false;
            let mut test_db = false;
            section(ui, t, app.tr("Database"), |ui| {
                egui::Grid::new("db").spacing(vec2(12.0, 8.0)).show(ui, |ui| {
                    ui.label(app.tr("Kind"));
                    let kinds =
                        [("demo", "Demo data (built-in city)"), ("mysql", "MySQL (the netmap database)"), ("sqlite", "SQLite file (same schema)")];
                    let current = kinds.iter().find(|(k, _)| *k == app.settings.kind).map(|(_, l)| app.tr(l)).unwrap_or("?").to_string();
                    egui::ComboBox::from_id_salt("db-kind").selected_text(current).width(320.0).show_ui(ui, |ui| {
                        for (k, l) in kinds {
                            if ui.selectable_label(app.settings.kind == k, app.tr(l)).clicked() {
                                app.settings.kind = k.into();
                            }
                        }
                    });
                    ui.end_row();
                    match app.settings.kind.as_str() {
                        "mysql" => {
                            text(ui, app.tr("Host"), &mut app.settings.host, "192.168.10.24", false);
                            text(ui, app.tr("Port"), &mut app.settings.port, "3306", false);
                            text(ui, app.tr("User"), &mut app.settings.user, "netmap", false);
                            let hint = if app.session.has_password() { app.tr("leave empty to keep it") } else { "" }.to_string();
                            text(ui, app.tr("Password"), &mut app.settings.password, &hint, true);
                            text(ui, app.tr("Database"), &mut app.settings.database, "netmap", false);
                        }
                        "sqlite" => text(ui, app.tr("Path"), &mut app.settings.path, "/data/netmap.sqlite", false),
                        _ => {}
                    }
                });
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if primary_button(ui, t, None, app.tr("Save and connect")).clicked() {
                        save_db = true;
                    }
                    if ui.button(app.tr("Test connection")).clicked() {
                        test_db = true;
                    }
                    let (state, color) = match app.session.state().db_state.as_str() {
                        "connected" => (app.tr("Connected"), Color32::from_rgb(0x22, 0xa0, 0x6b)),
                        "error" => (app.tr("Connection error"), Color32::from_rgb(0xe5, 0x48, 0x4d)),
                        _ => (app.tr("Not tested"), t.muted),
                    };
                    ui.label(RichText::new(state).color(color));
                    ui.label(
                        RichText::new(if app.session.has_password() { app.tr("Password set") } else { app.tr("Password not set") })
                            .small()
                            .color(t.muted),
                    );
                });
                if !app.settings.result.is_empty() {
                    ui.label(RichText::new(&app.settings.result).small().color(t.muted));
                }
                if !app.session.state().db_message.is_empty() && app.settings.result.is_empty() {
                    ui.label(RichText::new(&app.session.state().db_message).small().color(t.muted));
                }
            });
            if save_db {
                let f = &app.settings;
                let mut params = json!({"kind": f.kind, "host": f.host, "port": f.port, "user": f.user, "database": f.database, "path": f.path});
                if !f.password.is_empty() {
                    params["password"] = json!(f.password);
                }
                app.settings.result = app.tr("Checking…").to_string();
                app.run("db.configure", params);
                app.settings.password.clear();
            }
            if test_db {
                app.settings.result = app.tr("Checking…").to_string();
                app.run("db.test", json!({}));
            }
            ui.add_space(12.0);
            let mut save_prefs = false;
            section(ui, t, app.tr("Preferences"), |ui| {
                let l = app.tr("Keep a local history of verifications").to_string();
                ui.checkbox(&mut app.settings.log_requests, l);
                let l = app.tr("Also write them to the database log (requests_log; needs INSERT rights)").to_string();
                ui.checkbox(&mut app.settings.log_to_database, l);
                ui.add_space(6.0);
                ui.label(RichText::new(app.tr("Radio reach by band (km)")).color(t.muted));
                ui.horizontal(|ui| {
                    for (band, km) in &mut app.settings.reach {
                        ui.label(band.as_str());
                        ui.add(egui::TextEdit::singleline(km).desired_width(56.0));
                    }
                });
                egui::Grid::new("prefs").spacing(vec2(12.0, 8.0)).show(ui, |ui| {
                    ui.label(app.tr("Bridges listed"));
                    ui.add(egui::TextEdit::singleline(&mut app.settings.max_bridges).desired_width(80.0));
                    ui.end_row();
                    let l = app.tr("Mobile cells around the address").to_string();
                    ui.checkbox(&mut app.settings.cells, l);
                    ui.end_row();
                    ui.label(app.tr("Cells radius (km)"));
                    ui.add(egui::TextEdit::singleline(&mut app.settings.cells_radius).desired_width(80.0));
                    ui.end_row();
                    ui.label(app.tr("Cells listed"));
                    ui.add(egui::TextEdit::singleline(&mut app.settings.max_cells).desired_width(80.0));
                    ui.end_row();
                });
                ui.add_space(6.0);
                egui::Grid::new("geo").spacing(vec2(12.0, 8.0)).show(ui, |ui| {
                    ui.label(app.tr("Geocoder (Nominatim URL)"));
                    ui.add_sized([360.0, 22.0], egui::TextEdit::singleline(&mut app.settings.geocode_url).hint_text("http://nominatim:8080"));
                    ui.end_row();
                    ui.label(app.tr("Map tiles URL"));
                    ui.add_sized([360.0, 22.0], egui::TextEdit::singleline(&mut app.settings.tiles_url).hint_text("https://tile.openstreetmap.org/{z}/{x}/{y}.png"));
                    ui.end_row();
                });
                ui.label(RichText::new(app.tr("The geocoder is asked only when no table places the civico; the answer is remembered.")).small().color(t.muted));
                ui.add_space(6.0);
                ui.label(RichText::new(app.tr("Wording rules")).color(t.muted));
                ui.label(
                    RichText::new(app.tr("One per line: VALUE=covered|planned|not_covered|unknown (VALUE* for a prefix)")).small().color(t.muted),
                );
                ui.add(egui::TextEdit::multiline(&mut app.settings.rules).desired_rows(4).desired_width(480.0));
                ui.add_space(8.0);
                if primary_button(ui, t, None, app.tr("Save preferences")).clicked() {
                    save_prefs = true;
                }
            });
            ui.add_space(12.0);
            let mut save_radio = false;
            section(ui, t, app.tr("Radio links (5 GHz)"), |ui| {
                ui.label(RichText::new(app.tr("The equipment on both ends; the sectors on the bridges use the same figures. Presets: 5.5 GHz, 40 MHz, 23 dBm, 23 dBi.")).small().color(t.muted));
                ui.add_space(6.0);
                egui::Grid::new("radio-params").spacing(vec2(12.0, 8.0)).show(ui, |ui| {
                    for (_, label, value) in &mut app.settings.radio {
                        ui.label(app.language.tr(label));
                        ui.add(egui::TextEdit::singleline(value).desired_width(100.0));
                        ui.end_row();
                    }
                });
                ui.add_space(8.0);
                if primary_button(ui, t, None, app.tr("Save radio settings")).clicked() {
                    save_radio = true;
                }
            });
            if save_radio {
                let radio: serde_json::Map<String, Value> = app.settings.radio.iter().map(|(k, _, v)| ((*k).to_string(), json!(v.trim()))).collect();
                if app.run("prefs.set", json!({"radio": radio})).is_some() {
                    app.settings.loaded = false;
                }
            }
            if save_prefs {
                let f = &app.settings;
                let reach: serde_json::Map<String, Value> =
                    f.reach.iter().filter(|(_, v)| !v.trim().is_empty()).map(|(b, v)| (b.clone(), json!(v.trim()))).collect();
                let rules: Vec<Value> =
                    f.rules.lines().filter_map(|l| l.split_once('=').map(|(v, s)| json!({"value": v.trim(), "status": s.trim()}))).collect();
                let params = json!({
                    "log_requests": f.log_requests, "log_to_database": f.log_to_database, "radio_reach_km": reach, "max_bridges": f.max_bridges.trim(), "cells": f.cells,
                    "cells_radius_km": f.cells_radius.trim(), "max_cells": f.max_cells.trim(), "rules": rules,
                    "geocode_url": f.geocode_url.trim(), "tiles_url": f.tiles_url.trim(),
                });
                if app.run("prefs.set", params).is_some() {
                    app.settings.loaded = false;
                }
            }
        });
    });
}
