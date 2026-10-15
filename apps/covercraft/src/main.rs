//! CoverCraft desktop app.
//!
//! Usage: `covercraft [--demo] [--home DIR] [--lang en|it] [--user NAME]`. The configuration lives
//! in the platform configuration directory (or `COVERCRAFT_HOME`); `--demo` shows the built-in city.
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use covercraft_ui_egui::core::config::Paths;
use covercraft_ui_egui::engine::Session;
use covercraft_ui_egui::{CoverApp, Language, Request};

struct App(CoverApp);

fn open_with_system(target: &str) {
    let _ = std::process::Command::new(if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "cmd"
    } else {
        "xdg-open"
    })
    .args(if cfg!(windows) { vec!["/C", "start", "", target] } else { vec![target] })
    .spawn();
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let title = self.0.window_title();
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Title(title));
        self.0.ui(ui);
        for request in std::mem::take(&mut self.0.requests) {
            match request {
                Request::OpenUrl(url) => {
                    if url.starts_with("http://") || url.starts_with("https://") {
                        open_with_system(&url);
                    }
                }
            }
        }
    }
}

const USAGE: &str = "Usage: covercraft [--demo] [--home DIR] [--lang en|it] [--user NAME]";

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut demo = false;
    let mut home: Option<String> = None;
    let mut lang: Option<Language> = None;
    let mut user: Option<String> = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--demo" => demo = true,
            "--home" => home = it.next().cloned(),
            "--lang" => lang = it.next().and_then(|l| Language::parse(l)),
            "--user" => user = it.next().cloned(),
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            _ => return Err(USAGE.to_string()),
        }
    }
    let mut session = if demo {
        Session::demo()
    } else {
        let paths = match home {
            Some(h) => Paths::under(std::path::Path::new(&h)),
            None => Paths::default_dirs(),
        };
        Session::with_lookups(
            covercraft_db::factory(),
            Some(covercraft_db::geocoder_factory()),
            Some(covercraft_db::cell_lookup_factory()),
            Some(paths),
        )
    };
    let user = user.or_else(|| std::env::var("USER").ok()).or_else(|| std::env::var("USERNAME").ok()).unwrap_or_default();
    session.set_user(&user);
    let mut app = CoverApp::new(Box::new(session));
    if let Some(l) = lang {
        app.language = l;
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1380.0, 880.0]).with_min_inner_size([900.0, 600.0]).with_title("CoverCraft"),
        ..Default::default()
    };
    eframe::run_native("CoverCraft", options, Box::new(move |_cc| Ok(Box::new(App(app))))).map_err(|e| e.to_string())
}
