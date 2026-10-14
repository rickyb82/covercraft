//! Repository tasks: `cargo xtask ci` (fmt, clippy, tests, layers, wasm), `cargo xtask layers`
//! and `cargo xtask wasm`.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// Each crate and the internal crates / UI toolkits it may depend on. Lower layers never reach up.
const LAYERS: &[(&str, &[&str])] = &[
    ("crates/core", &[]),
    ("crates/db", &["covercraft-core", "mysql", "rusqlite"]),
    ("crates/engine", &["covercraft-core"]),
    ("crates/ui-egui", &["covercraft-engine", "egui"]),
    ("crates/mcp", &["covercraft-engine"]),
    ("crates/remote", &["covercraft-engine"]),
    ("apps/covercraft", &["covercraft-ui-egui", "covercraft-db", "egui", "eframe", "rfd"]),
    ("apps/covercraft-cli", &["covercraft-engine", "covercraft-db", "covercraft-mcp", "covercraft-remote"]),
    ("apps/covercraft-web", &["covercraft-ui-egui", "covercraft-remote", "egui", "eframe", "rfd"]),
];
const WATCHED: &[&str] = &[
    "covercraft-core",
    "covercraft-db",
    "covercraft-mcp",
    "covercraft-engine",
    "covercraft-ui-egui",
    "egui",
    "eframe",
    "winit",
    "rfd",
    "mysql",
    "rusqlite",
];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().map(Path::to_path_buf).unwrap_or_default()
}

fn layers() -> Result<(), String> {
    let mut bad = Vec::new();
    for (krate, allowed) in LAYERS {
        let manifest = root().join(krate).join("Cargo.toml");
        let text = std::fs::read_to_string(&manifest).map_err(|e| format!("{}: {e}", manifest.display()))?;
        let mut in_deps = false;
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                in_deps = line == "[dependencies]";
                continue;
            }
            let name = line.split(['=', ' ', '.']).next().unwrap_or_default();
            if in_deps && WATCHED.contains(&name) && !allowed.contains(&name) {
                bad.push(format!("{krate} must not depend on {name}"));
            }
        }
    }
    if bad.is_empty() { Ok(()) } else { Err(bad.join("\n")) }
}

fn cargo(args: &[&str]) -> Result<(), String> {
    eprintln!("$ cargo {}", args.join(" "));
    let status = Command::new(env!("CARGO")).args(args).current_dir(root()).status().map_err(|e| e.to_string())?;
    if status.success() { Ok(()) } else { Err(format!("cargo {} failed", args.join(" "))) }
}

fn ci() -> Result<(), String> {
    cargo(&["fmt", "--all", "--", "--check"])?;
    cargo(&["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])?;
    cargo(&["test", "--workspace"])?;
    layers()?;
    wasm()
}

/// The web app and everything under it build for wasm32 with clippy clean.
fn wasm() -> Result<(), String> {
    cargo(&["clippy", "-p", "covercraft-web", "--target", "wasm32-unknown-unknown", "--", "-D", "warnings"])
}

fn main() -> ExitCode {
    let task = std::env::args().nth(1).unwrap_or_default();
    let result = match task.as_str() {
        "ci" => ci(),
        "layers" => layers(),
        "wasm" => wasm(),
        _ => Err("usage: cargo xtask <ci|layers|wasm>".to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
