//! CoverCraft's egui front end (layer L6): an address bar, the coverage report, the history,
//! the radio bridges and the settings, on top of the engine. It is thin: it draws the session's
//! state and acts only through [`CoverApp::run`], so every action is also a command for the
//! CLI and the MCP server.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod chrome;
pub mod i18n;
pub mod icons;
mod nav;
mod report;
mod settings;
pub mod theme;
mod widgets;

use std::time::Duration;

pub use covercraft_engine as engine;
pub use covercraft_engine::core;
use covercraft_engine::{Completed, CoverSession, Session};
pub use i18n::Language;
use serde_json::{Value, json};
use web_time::Instant;

/// What the top bar shows underneath.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Verify,
    History,
    Radio,
    Settings,
}

/// Host-only actions the app asks for (served by apps/*).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Request {
    OpenUrl(String),
}

/// The address being typed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Form {
    /// Region and province chosen in the cascade (empty = any).
    pub regione: String,
    pub provincia: String,
    pub comune: String,
    pub via: String,
    pub civico: String,
    /// The street picked from the suggestions (its EGON code), if any.
    pub street_egon: String,
    /// The last street query sent, to send each one once.
    pub sent_query: String,
    pub focus_via: bool,
    pub focus_civico: bool,
    /// The suggestion menu under a field had the pointer on the last frame (so a click on it
    /// lands even though the field lost the focus on the press).
    pub menu_hover: [bool; 3],
}

pub struct CoverApp {
    pub session: Box<dyn CoverSession>,
    pub language: Language,
    pub dark: bool,
    applied_dark: Option<bool>,
    pub screen: Screen,
    pub form: Form,
    pub status: String,
    toast: Option<(String, Instant)>,
    pub requests: Vec<Request>,
    pub settings: settings::SettingsForm,
    /// The candidates of an ambiguous verification are being shown.
    pub picking: bool,
    /// Expanded service rows (technology names).
    pub expanded: Vec<String>,
    pub history_all: bool,
    loaded: Loaded,
}

#[derive(Default)]
struct Loaded {
    comuni: bool,
    history: bool,
    bridges: bool,
}

impl CoverApp {
    pub fn new(session: Box<dyn CoverSession>) -> Self {
        let language = match session.config().preferences.language.as_str() {
            "it" => Language::It,
            "en" => Language::En,
            _ => Language::system(),
        };
        let dark = session.config().preferences.theme == "dark";
        Self {
            session,
            language,
            dark,
            applied_dark: None,
            screen: Screen::Verify,
            form: Form::default(),
            status: String::new(),
            toast: None,
            requests: Vec::new(),
            settings: settings::SettingsForm::default(),
            picking: false,
            expanded: Vec::new(),
            history_all: true,
            loaded: Loaded::default(),
        }
    }

    pub fn demo() -> Self {
        Self::new(Box::new(Session::demo()))
    }

    pub fn tr<'a>(&self, text: &'a str) -> &'a str {
        self.language.tr(text)
    }

    pub fn italian(&self) -> bool {
        self.language == Language::It
    }

    pub fn window_title(&self) -> String {
        match &self.session.state().report {
            Some(r) => format!("{} · CoverCraft", r.civico.display()),
            None => "CoverCraft".into(),
        }
    }

    /// Runs a command; failures go to the status bar. A local session answers at once, a
    /// remote one through `pump`; both answers reach [`Self::handle_result`].
    pub fn run(&mut self, id: &str, params: Value) -> Option<Value> {
        match self.session.execute(id, &params) {
            Ok(v) => {
                if v.get("queued").is_none() && v.get("job").is_none() {
                    self.handle_result(id, &v);
                }
                Some(v)
            }
            Err(e) => {
                self.status = self.language.tr_message(&e.to_string());
                None
            }
        }
    }

    fn handle_result(&mut self, command: &str, v: &Value) {
        match command {
            "coverage.verify" => {
                self.status.clear();
                if v.get("ambiguous").is_some() {
                    self.picking = true;
                } else {
                    self.picking = false;
                    self.expanded.clear();
                    if let Some(c) = v.get("civico").and_then(|c| c.get("display")).and_then(Value::as_str) {
                        self.toast(self.language.fill("Verified {address}", &[("address", c)]));
                    }
                }
            }
            "history.open" => {
                self.picking = false;
                self.screen = Screen::Verify;
            }
            "db.configure" | "db.test" => {
                self.settings.result = if v["ok"] == true {
                    format!("{}: {}", self.tr("Connected"), v["info"].as_str().unwrap_or_default())
                } else {
                    self.tr("Connection error").to_string()
                };
                self.toast(self.settings.result.clone());
            }
            "prefs.set" => self.toast(self.tr("Saved").to_string()),
            "comune.list" => self.loaded.comuni = true,
            "history.list" => self.loaded.history = true,
            "radio.list" => self.loaded.bridges = true,
            _ => {}
        }
    }

    fn toast(&mut self, text: String) {
        self.toast = Some((text, Instant::now()));
    }

    fn pump(&mut self) {
        let done: Vec<Completed> = self.session.pump();
        for c in done {
            match c.result {
                Ok(v) => self.handle_result(&c.command, &v),
                Err(e) => {
                    let text = self.language.tr_message(&e.to_string());
                    if c.command.starts_with("db.") {
                        self.settings.result = text.clone();
                    }
                    self.status = text;
                }
            }
        }
    }

    /// Starts the verification of the typed address.
    pub fn verify(&mut self) {
        let f = self.form.clone();
        if f.comune.trim().is_empty() || f.via.trim().is_empty() {
            self.status = self.tr("Type comune, street and number above, then Check coverage.").to_string();
            return;
        }
        self.status = self.tr("Checking…").to_string();
        self.run("coverage.verify", json!({"comune": f.comune.trim(), "via": f.via.trim(), "civico": f.civico.trim()}));
    }

    /// Verifies one civico by its EGON code (a candidate, a history entry's address).
    pub fn verify_egon(&mut self, egon: &str) {
        self.status = self.tr("Checking…").to_string();
        self.run("coverage.verify", json!({"egon": egon}));
    }

    /// Asks for street suggestions when the typed street changed.
    fn suggest_streets(&mut self) {
        let q = self.form.via.trim().to_string();
        if self.form.comune.trim().is_empty() || q.chars().count() < 2 || q == self.form.sent_query {
            return;
        }
        self.form.sent_query = q.clone();
        self.form.street_egon.clear();
        self.run("street.search", json!({"comune": self.form.comune.trim(), "query": q}));
    }

    /// The comuni of the chosen region and province.
    pub fn comuni_within(&self) -> Vec<core::model::Comune> {
        let (r, p) = (&self.form.regione, &self.form.provincia);
        self.session
            .state()
            .comuni
            .iter()
            .filter(|c| r.is_empty() || &c.regione == r)
            .filter(|c| p.is_empty() || &c.provincia == p)
            .cloned()
            .collect()
    }

    /// The regions known to the database.
    pub fn regioni(&self) -> Vec<String> {
        let mut out: Vec<String> = self.session.state().comuni.iter().map(|c| c.regione.clone()).filter(|r| !r.is_empty()).collect();
        out.sort();
        out.dedup();
        out
    }

    /// The provinces of the chosen region: (code, name).
    pub fn province(&self) -> Vec<(String, String)> {
        let r = &self.form.regione;
        let mut out: Vec<(String, String)> = self
            .session
            .state()
            .comuni
            .iter()
            .filter(|c| r.is_empty() || &c.regione == r)
            .map(|c| (c.provincia.clone(), c.provincia_name.clone()))
            .collect();
        out.sort();
        out.dedup();
        out
    }

    /// The comuni whose name starts with what is typed (within the region and province; with
    /// nothing typed, the first of the province).
    pub fn comune_hits(&self) -> Vec<core::model::Comune> {
        let q = core::address::normalize(&self.form.comune);
        let within = self.comuni_within();
        if q.is_empty() {
            return if self.form.provincia.is_empty() { Vec::new() } else { within };
        }
        let mut out: Vec<core::model::Comune> = within.iter().filter(|c| c.name.starts_with(&q)).cloned().collect();
        out.extend(within.iter().filter(|c| !c.name.starts_with(&q) && c.name.contains(&q)).cloned());
        out
    }

    fn ensure_loaded(&mut self) {
        if !self.loaded.comuni && self.session.state().comuni.is_empty() && self.session.jobs_pending() == 0 {
            self.loaded.comuni = true;
            self.run("comune.list", json!({"refresh": true}));
        }
        match self.screen {
            Screen::History if !self.loaded.history => {
                self.loaded.history = true;
                self.run("history.list", json!({"all": self.history_all, "limit": 50}));
            }
            Screen::Radio if !self.loaded.bridges => {
                self.loaded.bridges = true;
                self.run("radio.list", json!({}));
            }
            _ => {}
        }
    }

    /// Reloads the history (after a verification, or on request).
    pub fn reload_history(&mut self) {
        self.loaded.history = false;
    }

    fn keyboard(&mut self, ui: &egui::Ui) {
        use egui::{Key, Modifiers};
        if ui.ctx().memory(|m| m.focused().is_some()) {
            if ui.ctx().input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter)) && self.screen == Screen::Verify {
                self.verify();
            }
            return;
        }
        let pressed = |m: Modifiers, k: Key| ui.ctx().input_mut(|i| i.consume_key(m, k));
        if pressed(Modifiers::NONE, Key::Slash) {
            self.form.focus_via = true;
            self.screen = Screen::Verify;
        } else if pressed(Modifiers::NONE, Key::Escape) {
            self.picking = false;
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        let tokens = theme::Tokens::for_dark_mode(self.dark);
        if self.applied_dark != Some(self.dark) {
            theme::apply(ui.ctx(), &tokens);
            self.applied_dark = Some(self.dark);
        }
        self.pump();
        self.ensure_loaded();
        self.suggest_streets();
        if self.session.jobs_pending() > 0 {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
        self.keyboard(ui);
        chrome::top_bar(self, ui, &tokens);
        chrome::status_bar(self, ui, &tokens);
        nav::show(self, ui, &tokens);
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(tokens.panel)).show(ui, |ui| match self.screen {
            Screen::Verify => report::show(self, ui, &tokens),
            Screen::History => report::history(self, ui, &tokens),
            Screen::Radio => report::bridges(self, ui, &tokens),
            Screen::Settings => settings::show(self, ui, &tokens),
        });
        chrome::toasts(self, ui, &tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(app: &mut CoverApp, ctx: &egui::Context, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))),
            events,
            ..Default::default()
        };
        let mut out = ctx.run_ui(input, |ui| app.ui(ui));
        out.textures_delta.clear();
        // Frames until the jobs land, so the app sees their answers.
        for _ in 0..200 {
            if app.session.jobs_pending() == 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
            let input =
                egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| app.ui(ui));
            out.textures_delta.clear();
        }
    }

    #[test]
    fn verifies_from_the_form_and_walks_the_screens() {
        let ctx = egui::Context::default();
        let mut app = CoverApp::demo();
        app.language = Language::It;
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.session.state().comuni.len(), 5);
        app.form.comune = "cag".into();
        assert_eq!(app.comune_hits()[0].name, "CAGLIARI");
        assert_eq!(app.regioni(), vec!["LAZIO".to_string(), "SARDEGNA".to_string()]);
        app.form.regione = "SARDEGNA".into();
        assert_eq!(app.province().len(), 3);
        app.form.provincia = "CA".into();
        app.form.comune.clear();
        assert_eq!(app.comune_hits().len(), 2);
        app.form.regione = "LAZIO".into();
        app.form.provincia.clear();
        app.form.comune = "cag".into();
        assert!(app.comune_hits().is_empty());
        app.form.regione.clear();
        app.form.comune = "Cagliari".into();
        app.form.via = "via ro".into();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.session.state().streets.len(), 1);
        app.verify();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert!(app.picking, "four civici match: the person picks");
        assert_eq!(app.session.state().civici.len(), 4);
        app.verify_egon("100010");
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert!(!app.picking);
        assert_eq!(app.session.state().report.as_ref().map(|r| r.civico.civico.as_str()), Some("10"));
        assert!(app.window_title().starts_with("VIA ROMA 10"));
        for s in [Screen::History, Screen::Radio, Screen::Settings, Screen::Verify] {
            app.screen = s;
            frame(&mut app, &ctx, vec![]);
            frame(&mut app, &ctx, vec![]);
        }
        assert_eq!(app.session.state().history.len(), 1);
        assert_eq!(app.session.state().bridges.len(), 5);
        app.dark = true;
        frame(&mut app, &ctx, vec![]);
        app.form.civico = "7".into();
        app.form.via = "via dante".into();
        app.verify();
        frame(&mut app, &ctx, vec![]);
        frame(&mut app, &ctx, vec![]);
        assert_eq!(app.session.state().report.as_ref().map(|r| r.civico.street.as_str()), Some("DANTE"));
        app.form.via.clear();
        app.verify();
        assert!(app.status.contains("Scrivi"));
    }
}
