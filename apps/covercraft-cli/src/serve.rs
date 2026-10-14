//! `covercraft-cli serve`: the server behind the web app. One engine session per user (the
//! framework names the user with `X-Covercraft-User`), a tiny HTTP API (`POST /api/cmd`,
//! `GET /api/snapshot?since=N`, `GET /api/health`) and the web app's files.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use covercraft_engine::Session;
use covercraft_engine::core::config::{Paths, slug};
use covercraft_engine::core::source::SourceFactory;
use serde_json::{Value, json};

pub struct Options {
    pub bind: String,
    /// Required beyond localhost: the framework's service token.
    pub token: String,
    pub static_dir: Option<PathBuf>,
    pub open: bool,
    /// The configuration directory (shared by every user's session).
    pub paths: Option<Paths>,
    pub demo: bool,
    pub lang: Option<String>,
}

type Shared = Arc<Mutex<Session>>;

struct Hub {
    opts: Options,
    factory: SourceFactory,
    sessions: Mutex<HashMap<String, Shared>>,
}

impl Hub {
    fn session_for(&self, user: &str) -> Result<Shared, String> {
        let key = {
            let s = slug(user);
            if s.is_empty() { "default".to_string() } else { s }
        };
        let mut map = self.sessions.lock().map_err(|_| "sessions unavailable".to_string())?;
        if let Some(s) = map.get(&key) {
            return Ok(s.clone());
        }
        if map.len() >= 500 {
            return Err("too many users loaded".into());
        }
        let mut session = if self.opts.demo { Session::demo() } else { Session::new(self.factory.clone(), self.opts.paths.clone()) };
        session.set_user(user);
        if let Some(l) = &self.opts.lang
            && session.config().preferences.language.is_empty()
        {
            let _ = session.execute("prefs.set", &json!({"language": l}));
        }
        let shared = Arc::new(Mutex::new(session));
        map.insert(key, shared.clone());
        Ok(shared)
    }

    fn all_sessions(&self) -> Vec<Shared> {
        self.sessions.lock().map(|m| m.values().cloned().collect()).unwrap_or_default()
    }
}

const MAX_BODY: usize = 4 * 1024 * 1024;

struct Request {
    method: String,
    path: String,
    query: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

fn read_request(stream: &mut TcpStream) -> Option<Request> {
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    if reader.read_line(&mut line).ok()? == 0 {
        return None;
    }
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let target = parts.next()?.to_string();
    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.clone(), String::new()),
    };
    let mut headers = Vec::new();
    let mut length = 0usize;
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h).ok()? == 0 || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            let (k, v) = (k.trim().to_ascii_lowercase(), v.trim().to_string());
            if k == "content-length" {
                length = v.parse().unwrap_or(0);
            }
            headers.push((k, v));
        }
        if headers.len() > 100 {
            return None;
        }
    }
    if length > MAX_BODY {
        return None;
    }
    let mut body = vec![0u8; length];
    if length > 0 {
        reader.read_exact(&mut body).ok()?;
    }
    Some(Request { method, path, query, headers, body })
}

fn header<'a>(req: &'a Request, name: &str) -> Option<&'a str> {
    req.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
}

fn query_param(query: &str, name: &str) -> Option<String> {
    query.split('&').find_map(|kv| kv.split_once('=').filter(|(k, _)| *k == name).map(|(_, v)| url_decode(v)))
}

fn url_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes.get(i) {
            Some(b'%') if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(bytes.get(i + 1..i + 3).unwrap_or_default()).unwrap_or_default();
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(b'%');
                        i += 1;
                    }
                }
            }
            Some(b'+') => {
                out.push(b' ');
                i += 1;
            }
            Some(b) => {
                out.push(*b);
                i += 1;
            }
            None => break,
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn respond(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8], origin: Option<&str>) {
    let cors = format!(
        "Access-Control-Allow-Origin: {}\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nAccess-Control-Allow-Headers: Content-Type, Authorization, X-Covercraft-User\r\nAccess-Control-Allow-Private-Network: true\r\nAccess-Control-Max-Age: 600\r\n",
        origin.unwrap_or("*")
    );
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n{cors}\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn json_response(stream: &mut TcpStream, status: &str, v: &Value, origin: Option<&str>) {
    respond(stream, status, "application/json; charset=utf-8", v.to_string().as_bytes(), origin);
}

fn mime_of(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" => "text/javascript",
        "wasm" => "application/wasm",
        "css" => "text/css",
        "json" => "application/json",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

fn serve_static(stream: &mut TcpStream, dir: &Path, path: &str, origin: Option<&str>) {
    let rel = path.trim_start_matches('/');
    let rel = if rel.is_empty() { "index.html" } else { rel };
    if rel.contains("..") || rel.contains('\\') {
        respond(stream, "400 Bad Request", "text/plain", b"bad path", origin);
        return;
    }
    let file = dir.join(rel);
    match std::fs::read(&file) {
        Ok(bytes) => respond(stream, "200 OK", mime_of(&file), &bytes, origin),
        Err(_) => respond(stream, "404 Not Found", "text/plain; charset=utf-8", b"not found", origin),
    }
}

const WELCOME: &str = "CoverCraft server is running.\n\nStart it with --static DIR to serve the web app (apps/covercraft-web built with trunk).\n\nAPI: POST /api/cmd {\"command\", \"params\"}  ·  GET /api/snapshot?since=N  ·  GET /api/health\nHeaders: Authorization: Bearer TOKEN  ·  X-Covercraft-User: name\n";

fn authenticate(hub: &Hub, req: &Request) -> Result<(), &'static str> {
    if hub.opts.token.is_empty() {
        return Ok(());
    }
    let given = header(req, "authorization")
        .and_then(|a| a.strip_prefix("Bearer "))
        .map(str::trim)
        .map(str::to_string)
        .or_else(|| query_param(&req.query, "token"));
    if given.as_deref() == Some(hub.opts.token.as_str()) { Ok(()) } else { Err("missing or wrong token") }
}

fn handle(mut stream: TcpStream, hub: Arc<Hub>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let Some(req) = read_request(&mut stream) else { return };
    let origin = header(&req, "origin").map(str::to_string);
    let origin = origin.as_deref();
    if req.method == "OPTIONS" {
        respond(&mut stream, "204 No Content", "text/plain", b"", origin);
        return;
    }
    if !req.path.starts_with("/api/") {
        match (req.method.as_str(), &hub.opts.static_dir) {
            ("GET", Some(dir)) => serve_static(&mut stream, dir, &req.path, origin),
            ("GET", None) if req.path == "/" => respond(&mut stream, "200 OK", "text/plain; charset=utf-8", WELCOME.as_bytes(), origin),
            ("GET", None) => respond(&mut stream, "404 Not Found", "text/plain", b"not found", origin),
            _ => respond(&mut stream, "405 Method Not Allowed", "text/plain", b"method not allowed", origin),
        }
        return;
    }
    if req.method == "GET" && req.path == "/api/health" {
        json_response(&mut stream, "200 OK", &json!({"ok": true, "name": "covercraft", "version": env!("CARGO_PKG_VERSION")}), origin);
        return;
    }
    if let Err(e) = authenticate(&hub, &req) {
        json_response(&mut stream, "401 Unauthorized", &json!({"error": e}), origin);
        return;
    }
    let user = header(&req, "x-covercraft-user").map(str::to_string).or_else(|| query_param(&req.query, "user")).unwrap_or_default();
    let session = match hub.session_for(&user) {
        Ok(s) => s,
        Err(e) => {
            json_response(&mut stream, "500 Internal Server Error", &json!({"error": e}), origin);
            return;
        }
    };
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/api/snapshot") => {
            let since: u64 = query_param(&req.query, "since").and_then(|s| s.parse().ok()).unwrap_or(0);
            let snapshot = session.lock().map(|s| s.snapshot(since)).unwrap_or(None);
            match snapshot {
                Some(s) => json_response(&mut stream, "200 OK", &serde_json::to_value(s).unwrap_or(Value::Null), origin),
                None => json_response(&mut stream, "200 OK", &json!({"unchanged": true}), origin),
            }
        }
        ("POST", "/api/cmd") => {
            let body: Value = match serde_json::from_slice(&req.body) {
                Ok(v) => v,
                Err(e) => {
                    json_response(&mut stream, "400 Bad Request", &json!({"error": format!("invalid JSON: {e}")}), origin);
                    return;
                }
            };
            let command = body.get("command").and_then(Value::as_str).unwrap_or_default().to_string();
            let params = body.get("params").cloned().unwrap_or_else(|| json!({}));
            let result = match session.lock() {
                Ok(mut s) => s.execute(&command, &params).map_err(|e| e.to_string()),
                Err(_) => Err("the session is unavailable".to_string()),
            };
            match result {
                Ok(v) => json_response(&mut stream, "200 OK", &json!({"result": v}), origin),
                Err(e) => json_response(&mut stream, "200 OK", &json!({"error": e}), origin),
            }
        }
        _ => json_response(&mut stream, "404 Not Found", &json!({"error": "unknown endpoint"}), origin),
    }
}

fn is_loopback(bind: &str) -> bool {
    bind.starts_with("127.") || bind.starts_with("localhost") || bind.starts_with("[::1]")
}

/// Runs the server until the process ends.
pub fn run(factory: SourceFactory, opts: Options) -> Result<(), String> {
    if !is_loopback(&opts.bind) && opts.token.is_empty() {
        return Err("binding beyond localhost needs --token SECRET (and an HTTPS reverse proxy in front)".into());
    }
    let listener = TcpListener::bind(&opts.bind).map_err(|e| format!("cannot listen on {}: {e}", opts.bind))?;
    let addr = listener.local_addr().map_err(|e| e.to_string())?;
    let hub = Arc::new(Hub { opts, factory, sessions: Mutex::new(HashMap::new()) });
    {
        let h = hub.clone();
        std::thread::Builder::new()
            .name("covercraft-pump".into())
            .spawn(move || {
                loop {
                    std::thread::sleep(Duration::from_millis(150));
                    for s in h.all_sessions() {
                        if let Ok(mut guard) = s.lock() {
                            let _ = guard.pump();
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
    }
    let url = format!("http://{}{}", addr, if hub.opts.token.is_empty() { String::new() } else { format!("/?token={}", hub.opts.token) });
    eprintln!("CoverCraft server on {url}");
    if let Some(d) = &hub.opts.static_dir {
        eprintln!("serving the web app from {}", d.display());
    }
    if hub.opts.open {
        let _ = std::process::Command::new(if cfg!(target_os = "macos") {
            "open"
        } else if cfg!(windows) {
            "cmd"
        } else {
            "xdg-open"
        })
        .args(if cfg!(windows) { vec!["/C", "start", "", url.as_str()] } else { vec![url.as_str()] })
        .spawn();
    }
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let h = hub.clone();
        let _ = std::thread::Builder::new().name("covercraft-http".into()).spawn(move || handle(stream, h));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(addr: &str, method: &str, path: &str, headers: &str, body: &str) -> (String, Value) {
        let mut s = TcpStream::connect(addr).expect("connect");
        s.write_all(
            format!("{method} {path} HTTP/1.1\r\nHost: x\r\n{headers}Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}", body.len())
                .as_bytes(),
        )
        .expect("write");
        let mut out = String::new();
        s.read_to_string(&mut out).expect("read");
        let (head, body) = out.split_once("\r\n\r\n").expect("response");
        (head.lines().next().unwrap_or_default().to_string(), serde_json::from_str(body).unwrap_or(Value::Null))
    }

    #[test]
    fn api_serves_snapshots_and_commands_per_user() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let addr = listener.local_addr().expect("addr").to_string();
        drop(listener);
        let a = addr.clone();
        std::thread::spawn(move || {
            let _ = run(
                covercraft_db::factory(),
                Options { bind: a, token: "t0k".into(), static_dir: None, open: false, paths: None, demo: true, lang: Some("it".into()) },
            );
        });
        for _ in 0..50 {
            if TcpStream::connect(&addr).is_ok() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(call(&addr, "GET", "/api/snapshot", "", "").0.contains("401"));
        let (st, h) = call(&addr, "GET", "/api/health", "", "");
        assert!(st.contains("200") && h["ok"] == true);
        let auth = "Authorization: Bearer t0k\r\nX-Covercraft-User: Anna Rossi\r\n";
        let (_, snap) = call(&addr, "GET", "/api/snapshot?since=0", auth, "");
        assert_eq!(snap["user"], "Anna Rossi");
        assert_eq!(snap["config"]["preferences"]["language"], "it");
        let rev = snap["revision"].as_u64().unwrap();
        let (_, same) = call(&addr, "GET", &format!("/api/snapshot?since={rev}"), auth, "");
        assert_eq!(same["unchanged"], true);
        let (_, v) = call(&addr, "POST", "/api/cmd", auth, r#"{"command": "coverage.verify", "params": {"egon": "100010"}}"#);
        let job = v["result"]["job"].as_u64().unwrap();
        let mut done = Value::Null;
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(50));
            let (_, s) = call(&addr, "GET", "/api/snapshot?since=0", auth, "");
            if let Some(c) = s["completed"].as_array().and_then(|l| l.iter().find(|c| c["job"] == job)) {
                done = c.clone();
                break;
            }
        }
        assert_eq!(done["result"]["civico"]["civico"], "10", "{done}");
        let (_, luca) = call(&addr, "GET", "/api/snapshot?since=0&user=luca&token=t0k", "", "");
        assert!(luca["state"]["report"].is_null(), "each user has a session");
        let (_, bad) = call(&addr, "POST", "/api/cmd", auth, "{");
        assert!(bad["error"].as_str().unwrap().contains("invalid JSON"));
        let (_, unknown) = call(&addr, "POST", "/api/cmd", auth, r#"{"command": "nope"}"#);
        assert!(unknown["error"].as_str().unwrap().contains("unknown command"));
        assert!(call(&addr, "GET", "/", "", "").0.contains("200"));
        assert!(call(&addr, "GET", "/api/nothing", auth, "").0.contains("404"));
    }
}
