//! CoverCraft headless: the CLI, the MCP server and the web app's server.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

mod serve;

use std::process::ExitCode;

use covercraft_engine::core::config::Paths;
use covercraft_engine::{Session, text};
use serde_json::{Value, json};

const USAGE: &str = "Usage:
  covercraft-cli verify COMUNE VIA [CIVICO] [--json] [--user NAME] [--lat L --lon L]
  covercraft-cli verify --egon CODE [--json] [--lat L --lon L]
  covercraft-cli comuni [QUERY]
  covercraft-cli streets COMUNE QUERY
  covercraft-cli civici COMUNE STREET_EGON
  covercraft-cli bridges
  covercraft-cli cell NET AREA CELL [--mcc 222] [--radio LTE]   # where a mobile cell is (MNC, LAC/TAC, CID/ECI)
  covercraft-cli cell NET AREA --enb ENB --sector S [--mcc 222]
  covercraft-cli history [--user NAME] [--all] [--limit N]
  covercraft-cli cells                                   # how many mobile cells cell_db holds and how old
  covercraft-cli cells import FILE.csv[.gz] [--mcc 222] [--net 1,10,88,50] [--db-user U --db-password PW]
  covercraft-cli db [--kind mysql|sqlite|demo] [--host H] [--port P] [--user U] [--password PW] [--database D] [--path FILE]
  covercraft-cli prefs [KEY=VALUE ...]
  covercraft-cli run --cmd 'id={\"json\"}' [--cmd ...]
  covercraft-cli mcp
  covercraft-cli mcp --server URL [--token T] [--user NAME]
  covercraft-cli serve [--bind 127.0.0.1:7979] [--static DIR] [--token SECRET] [--open]
  covercraft-cli commands

Options: --demo (the built-in city), --home DIR (configuration directory), --lang en|it
Environment: COVERCRAFT_HOME, COVERCRAFT_TOKEN, COVERCRAFT_LANG";

fn flag(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn has(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

/// Positional arguments (not flags nor their values).
fn positional(args: &[String]) -> Vec<String> {
    let valued = [
        "--home",
        "--lang",
        "--user",
        "--limit",
        "--egon",
        "--kind",
        "--host",
        "--port",
        "--password",
        "--database",
        "--path",
        "--cmd",
        "--server",
        "--token",
        "--bind",
        "--static",
        "--mcc",
        "--net",
        "--db-user",
        "--db-password",
    ];
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
            continue;
        }
        if valued.contains(&a.as_str()) {
            skip = true;
            continue;
        }
        if a.starts_with("--") {
            continue;
        }
        out.push(a.clone());
    }
    out
}

fn session(args: &[String]) -> Session {
    let mut s = if has(args, "--demo") {
        Session::demo()
    } else {
        let paths = match flag(args, "--home") {
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
    if let Some(l) = flag(args, "--lang").or_else(|| std::env::var("COVERCRAFT_LANG").ok()) {
        let _ = s.execute("prefs.set", &json!({"language": l}));
    }
    if let Some(u) = flag(args, "--user") {
        s.set_user(&u);
    }
    s
}

/// An OpenCelliD `updated` stamp (Unix seconds) as a date, else as it is.
fn stamp(s: &str) -> String {
    match s.trim().parse::<i64>() {
        Ok(secs) if secs > 0 => {
            let days = secs.div_euclid(86_400);
            // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
            let z = days + 719_468;
            let era = z.div_euclid(146_097);
            let doe = z.rem_euclid(146_097);
            let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
            let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
            let mp = (5 * doy + 2) / 153;
            let d = doy - (153 * mp + 2) / 5 + 1;
            let m = if mp < 10 { mp + 3 } else { mp - 9 };
            let y = yoe + era * 400 + i64::from(m <= 2);
            format!("{y:04}-{m:02}-{d:02}")
        }
        _ => s.to_string(),
    }
}

fn print_json(v: &Value) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

fn run(args: &[String]) -> Result<(), String> {
    let pos = positional(args);
    let json_out = has(args, "--json");
    match pos.first().map(String::as_str) {
        None | Some("--help") | Some("-h") | Some("help") => {
            println!("{USAGE}");
            Ok(())
        }
        Some("commands") => {
            for c in covercraft_engine::COMMANDS {
                println!("{:<18} {:<34} {}", c.id, c.label, c.params);
            }
            Ok(())
        }
        Some("verify") => {
            let mut s = session(args);
            let italian = s.italian();
            let mut params = json!({"user": flag(args, "--user").unwrap_or_default(), "log": !has(args, "--no-log")});
            if let (Some(lat), Some(lon)) = (flag(args, "--lat"), flag(args, "--lon")) {
                params["lat"] = json!(lat);
                params["lon"] = json!(lon);
            }
            if let Some(e) = flag(args, "--egon") {
                params["egon"] = json!(e);
            } else {
                let (comune, via) = (pos.get(1), pos.get(2));
                let (Some(comune), Some(via)) = (comune, via) else { return Err(USAGE.to_string()) };
                params["comune"] = json!(comune);
                params["via"] = json!(via);
                params["civico"] = json!(pos.get(3).cloned().unwrap_or_default());
            }
            let v = s.run_blocking("coverage.verify", params).map_err(|e| e.to_string())?;
            if json_out {
                print_json(&v);
            } else if v.get("ambiguous").is_some() {
                println!(
                    "{}",
                    if italian {
                        "Più civici corrispondono: ripeti con --egon CODICE"
                    } else {
                        "More than one civico matches: repeat with --egon CODE"
                    }
                );
                for c in v["candidates"].as_array().into_iter().flatten() {
                    println!("  {:<10} {}", c["egon"].as_str().unwrap_or_default(), c["display"].as_str().unwrap_or_default());
                }
            } else if let Some(r) = covercraft_engine::report_from_json(&v) {
                print!("{}", text::report_text(&r, italian));
            }
            Ok(())
        }
        Some("comuni") => {
            let mut s = session(args);
            let v =
                s.run_blocking("comune.list", json!({"query": pos.get(1).cloned().unwrap_or_default(), "limit": 500})).map_err(|e| e.to_string())?;
            if json_out {
                print_json(&v);
            } else {
                for c in v["comuni"].as_array().into_iter().flatten() {
                    println!(
                        "{:<40} {:<4} {}",
                        c["name"].as_str().unwrap_or_default(),
                        c["provincia"].as_str().unwrap_or_default(),
                        if c["has_tim"] == true { "TIM" } else { "" }
                    );
                }
            }
            Ok(())
        }
        Some("streets") => {
            let (Some(comune), Some(query)) = (pos.get(1), pos.get(2)) else { return Err(USAGE.to_string()) };
            let mut s = session(args);
            let v = s.run_blocking("street.search", json!({"comune": comune, "query": query, "limit": 100})).map_err(|e| e.to_string())?;
            if json_out {
                print_json(&v);
            } else {
                for st in v["streets"].as_array().into_iter().flatten() {
                    println!("{:<12} {}", st["egon"].as_str().unwrap_or_default(), st["display"].as_str().unwrap_or_default());
                }
            }
            Ok(())
        }
        Some("civici") => {
            let (Some(comune), Some(street)) = (pos.get(1), pos.get(2)) else { return Err(USAGE.to_string()) };
            let mut s = session(args);
            let v = s.run_blocking("civico.list", json!({"comune": comune, "street_egon": street, "limit": 500})).map_err(|e| e.to_string())?;
            if json_out {
                print_json(&v);
            } else {
                for c in v["civici"].as_array().into_iter().flatten() {
                    println!("{:<12} {}", c["egon"].as_str().unwrap_or_default(), c["display"].as_str().unwrap_or_default());
                }
            }
            Ok(())
        }
        Some("bridges") => {
            let mut s = session(args);
            let v = s.run_blocking("radio.list", json!({})).map_err(|e| e.to_string())?;
            if json_out {
                print_json(&v);
            } else {
                for b in v["bridges"].as_array().into_iter().flatten() {
                    println!(
                        "{:>4} {:<24} {:>10} {:>10} {:>5} {:<2} {}",
                        b["id"],
                        b["name"].as_str().unwrap_or_default(),
                        b["lat"].as_str().unwrap_or_default(),
                        b["lon"].as_str().unwrap_or_default(),
                        b["alt"].as_str().unwrap_or_default(),
                        b["band"].as_str().unwrap_or_default(),
                        b["state"].as_str().unwrap_or_default()
                    );
                }
            }
            Ok(())
        }
        Some("cell") => {
            let mut s = session(args);
            let mut p = json!({"net": pos.get(1).cloned().unwrap_or_default(), "area": pos.get(2).cloned().unwrap_or_default()});
            if let Some(c) = pos.get(3) {
                p["cell"] = json!(c);
            }
            for (k, f) in [("mcc", "--mcc"), ("enb", "--enb"), ("sector", "--sector"), ("radio", "--radio")] {
                if let Some(v) = flag(args, f) {
                    p[k] = json!(v);
                }
            }
            let v = s.run_blocking("cell.find", p).map_err(|e| e.to_string())?;
            if json_out {
                print_json(&v);
            } else {
                println!("{}  {}", v["query"].as_str().unwrap_or_default(), v["note"].as_str().unwrap_or_default());
                for c in v["cells"].as_array().into_iter().flatten() {
                    println!(
                        "  {:<5} lat {:<10} lon {:<10} range {:>6} m  samples {:>5}  {}  {}",
                        c["radio"].as_str().unwrap_or_default(),
                        c["lat"].as_str().unwrap_or_default(),
                        c["lon"].as_str().unwrap_or_default(),
                        c["range"].as_str().unwrap_or_default(),
                        c["samples"].as_str().unwrap_or_default(),
                        c["source"].as_str().unwrap_or_default(),
                        c["map_url"].as_str().unwrap_or_default()
                    );
                }
            }
            Ok(())
        }
        Some("cells") => {
            let s = session(args);
            let mut db = s.config().db.clone();
            if let Some(u) = flag(args, "--db-user") {
                db.user = u;
            }
            let password =
                flag(args, "--db-password").or_else(|| std::env::var("COVERCRAFT_DB_PASSWORD").ok()).unwrap_or_else(|| s.db_password().to_string());
            let mut src = covercraft_db::open_sql(&db, &password).map_err(|e| e.to_string())?;
            match pos.get(1).map(String::as_str) {
                Some("import") => {
                    let file = pos.get(2).ok_or("cells import: give the OpenCelliD export (FILE.csv or FILE.csv.gz)")?;
                    let mcc = flag(args, "--mcc").unwrap_or_else(|| "222".into());
                    let nets: Vec<String> = flag(args, "--net").map(|n| n.split(',').map(str::to_string).collect()).unwrap_or_default();
                    let reader = covercraft_db::cells::open_export(file).map_err(|e| e.to_string())?;
                    let r = covercraft_db::cells::import(&mut src, reader, &mcc, &nets).map_err(|e| e.to_string())?;
                    if json_out {
                        print_json(&json!({"read": r.read, "written": r.written, "deleted": r.deleted, "skipped": r.skipped}));
                    } else {
                        println!(
                            "cell_db: {} rows of MCC {mcc} replaced by {} from {file} ({} lines read, {} skipped)",
                            r.deleted, r.written, r.read, r.skipped
                        );
                    }
                }
                Some(other) => return Err(format!("cells: unknown action {other:?} (import)")),
                None => {
                    let (count, oldest, newest) = covercraft_db::cells::summary(&mut src).map_err(|e| e.to_string())?;
                    if json_out {
                        print_json(&json!({"count": count, "oldest": oldest, "newest": newest}));
                    } else {
                        println!("cell_db: {count} cells, updated between {} and {}", stamp(&oldest), stamp(&newest));
                    }
                }
            }
            Ok(())
        }
        Some("history") => {
            let mut s = session(args);
            let limit: u64 = flag(args, "--limit").and_then(|l| l.parse().ok()).unwrap_or(20);
            let v = s
                .run_blocking("history.list", json!({"user": flag(args, "--user").unwrap_or_default(), "all": has(args, "--all"), "limit": limit}))
                .map_err(|e| e.to_string())?;
            if json_out {
                print_json(&v);
            } else {
                for h in v["history"].as_array().into_iter().flatten() {
                    println!(
                        "{:>6} {:<17} {:<12} {}",
                        h["id"],
                        h["date"].as_str().unwrap_or_default(),
                        h["user"].as_str().unwrap_or_default(),
                        h["summary"].as_str().unwrap_or_default()
                    );
                }
            }
            Ok(())
        }
        Some("db") => {
            let mut s = session(args);
            let mut params = json!({});
            for k in ["kind", "host", "port", "user", "password", "database", "path"] {
                if let Some(v) = flag(args, &format!("--{k}")) {
                    params[k] = json!(v);
                }
            }
            let v = if params.as_object().is_some_and(|o| o.is_empty()) {
                s.run_blocking("db.test", json!({})).map_err(|e| e.to_string())?;
                s.run_blocking("db.get", json!({})).map_err(|e| e.to_string())?
            } else {
                s.run_blocking("db.configure", params).map_err(|e| e.to_string())?
            };
            print_json(&v);
            Ok(())
        }
        Some("prefs") => {
            let mut s = session(args);
            let mut params = json!({});
            for kv in pos.iter().skip(1) {
                if let Some((k, v)) = kv.split_once('=') {
                    params[k] = serde_json::from_str(v).unwrap_or_else(|_| json!(v));
                }
            }
            let v = if params.as_object().is_some_and(|o| o.is_empty()) {
                s.run_blocking("prefs.get", json!({}))
            } else {
                s.run_blocking("prefs.set", params)
            };
            print_json(&v.map_err(|e| e.to_string())?);
            Ok(())
        }
        Some("run") => {
            let mut s = session(args);
            let mut i = 0;
            while let Some(p) = args.iter().skip(i).position(|a| a == "--cmd") {
                let spec = args.get(i + p + 1).ok_or("--cmd needs a value")?;
                i += p + 2;
                let (id, params) = match spec.split_once('=') {
                    Some((id, raw)) => (id.to_string(), serde_json::from_str::<Value>(raw).map_err(|e| format!("{id}: {e}"))?),
                    None => (spec.clone(), json!({})),
                };
                match s.run_blocking(&id, params) {
                    Ok(v) => print_json(&json!({"command": id, "result": v})),
                    Err(e) => print_json(&json!({"command": id, "error": e.to_string()})),
                }
            }
            Ok(())
        }
        Some("mcp") => {
            let mut server = match flag(args, "--server") {
                Some(url) => covercraft_mcp::Server::new(covercraft_remote::RemoteSession::new(
                    &url,
                    &flag(args, "--token").unwrap_or_default(),
                    &flag(args, "--user").unwrap_or_default(),
                )),
                None => covercraft_mcp::Server::new(session(args)),
            };
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            server.serve(stdin.lock(), stdout.lock()).map_err(|e| e.to_string())
        }
        Some("serve") => {
            let static_dir = flag(args, "--static").map(std::path::PathBuf::from).or_else(|| {
                let candidates = [std::path::PathBuf::from("dist/web"), std::path::PathBuf::from("/app/web")];
                candidates.into_iter().find(|p| p.join("index.html").is_file())
            });
            let paths = match flag(args, "--home") {
                Some(h) => Paths::under(std::path::Path::new(&h)),
                None => Paths::default_dirs(),
            };
            serve::run(
                covercraft_db::factory(),
                serve::Options {
                    bind: flag(args, "--bind").unwrap_or_else(|| "127.0.0.1:7979".into()),
                    token: flag(args, "--token").or_else(|| std::env::var("COVERCRAFT_TOKEN").ok()).unwrap_or_default(),
                    static_dir,
                    open: has(args, "--open"),
                    paths: Some(paths),
                    demo: has(args, "--demo"),
                    lang: flag(args, "--lang").or_else(|| std::env::var("COVERCRAFT_LANG").ok()),
                },
            )
        }
        Some(other) => Err(format!("unknown command `{}`\n{USAGE}", other.chars().take(40).collect::<String>())),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
