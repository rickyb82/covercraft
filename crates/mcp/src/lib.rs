//! CoverCraft's MCP server (layer L6): the Model Context Protocol over stdio (newline-delimited
//! JSON-RPC 2.0), so an assistant can verify the coverage of an address in plain language.
//! Every tool is an engine command; the database password is never returned.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::io::{BufRead, Write};

use covercraft_engine::{CoverSession, Session, text};
use serde_json::{Value, json};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const SUPPORTED_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];
pub const REPORT_URI: &str = "covercraft://report";
pub const BRIDGES_URI: &str = "covercraft://bridges";
const MAX_LINE: usize = 8 * 1024 * 1024;

const INSTRUCTIONS: &str = "CoverCraft verifies the connectivity available at an Italian address from the operator's own coverage database: \
TIM/FiberCop (FTTH, FTTC, ADSL, SHDSL, ULL, dedicated fibre), Open Fiber (FTTH by staircase, XGS-PON POPs), Fastweb (FTTH, FTTS, VULA), the operator's own \
network, radio bridges by distance and bearing, and mobile cells. Workflow: verify_coverage with comune, via and civico (or an EGON civico code). \
If it answers with candidates, show them and verify again with the chosen egon. Use search_comune / search_street / list_civici to help the person \
spell the address as the tables do. Every status keeps the raw cell next to the traffic light: when in doubt, show the raw value. \
Verifications are kept in a local history with the user's name; pass `user` when you know who is asking.";

type ToolResult = Result<Value, String>;

pub struct Server {
    session: Box<dyn CoverSession>,
}

fn tools() -> Value {
    let comune = json!({"type": "string", "description": "Comune name as in the tables (upper case accepted), optionally with the province: \"CAGLIARI\", \"Cagliari (CA)\""});
    let language = json!({"type": "string", "enum": ["en", "it"], "description": "Language of the text answer (default: the preference)"});
    let limit = json!({"type": "integer", "minimum": 1, "description": "Optional cap on the rows returned (default: all)"});
    json!([
        {"name": "verify_coverage", "description": "The coverage report for a civico: every provider's technologies with a traffic light (covered / planned / not covered / unknown) and the raw cell, speeds and distances, Open Fiber staircases, radio bridges by distance and bearing, mobile cells. Give comune + via + civico, or egon. If more than one civico matches, the answer lists candidates: ask the person and call again with the chosen egon.",
         "inputSchema": {"type": "object", "properties": {"comune": comune, "via": {"type": "string", "description": "Street, with or without VIA/PIAZZA"}, "civico": {"type": "string", "description": "House number, e.g. 10, 10/A"}, "egon": {"type": "string", "description": "EGON civico code (skips the address search)"}, "user": {"type": "string", "description": "Who is asking (written in the log)"}, "log": {"type": "boolean", "description": "Record the verification (default: the preference)"}, "language": language}}},
        {"name": "search_comune", "description": "Comuni known to the database whose name contains the text, with region, province and whether TIM's table exists. regioni=true lists the regions, province=true the provinces (of a region).",
         "inputSchema": {"type": "object", "properties": {"query": {"type": "string"}, "regione": {"type": "string"}, "provincia": {"type": "string", "description": "Province code or name"}, "regioni": {"type": "boolean"}, "province": {"type": "boolean"}, "limit": limit}}},
        {"name": "search_street", "description": "Streets of a comune whose name contains the words, with their EGON street code.",
         "inputSchema": {"type": "object", "properties": {"comune": comune, "query": {"type": "string"}, "limit": limit}, "required": ["comune", "query"]}},
        {"name": "list_civici", "description": "The civici of a street (by EGON street code from search_street), each with its EGON civico code.",
         "inputSchema": {"type": "object", "properties": {"comune": comune, "street_egon": {"type": "string"}, "limit": limit}, "required": ["comune", "street_egon"]}},
        {"name": "radio_bridges", "description": "The operator's radio bridges: name, coordinates, altitude, state, band.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "history", "description": "Past verifications from the local history, newest first (summary, user, date).",
         "inputSchema": {"type": "object", "properties": {"user": {"type": "string"}, "all": {"type": "boolean", "description": "Everyone's (default: the session's user)"}, "limit": limit}}},
        {"name": "open_history", "description": "The full report of a past verification (id from history).",
         "inputSchema": {"type": "object", "properties": {"id": {"type": "integer"}, "language": language}, "required": ["id"]}},
        {"name": "database_settings", "description": "The database the reports come from (never the password) and the connection state.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "configure_database", "description": "Point CoverCraft at the coverage database (MySQL: host, port, user, password, database; or an SQLite file; or demo) and test the connection.",
         "inputSchema": {"type": "object", "properties": {"kind": {"type": "string", "enum": ["mysql", "sqlite", "demo"]}, "host": {"type": "string"}, "port": {"type": "integer"}, "user": {"type": "string"}, "password": {"type": "string"}, "database": {"type": "string"}, "path": {"type": "string"}}}},
        {"name": "test_database", "description": "Test the database connection.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "get_preferences", "description": "Preferences: language, theme, logging, radio reach by band (km), limits, wording rules.",
         "inputSchema": {"type": "object", "properties": {}}},
        {"name": "set_preferences", "description": "Change preferences (only the fields given). Wording rules map a raw cell value (or PREFIX*) to covered / planned / not_covered / unknown.",
         "inputSchema": {"type": "object", "properties": {"language": {"type": "string", "enum": ["en", "it", "system"]}, "theme": {"type": "string", "enum": ["light", "dark", "system"]}, "log_requests": {"type": "boolean", "description": "Keep a local history of verifications"}, "log_to_database": {"type": "boolean", "description": "Also write them to the database requests_log (needs INSERT rights; default off)"}, "radio_reach_km": {"type": "object", "description": "Band → km, e.g. {\"A\": 20, \"D\": 4}"}, "max_bridges": limit, "cells": {"type": "boolean"}, "cells_radius_km": {"type": "number"}, "max_cells": limit, "rules": {"type": "array", "items": {"type": "object", "properties": {"value": {"type": "string"}, "status": {"type": "string", "enum": ["covered", "planned", "not_covered", "unknown"]}}, "required": ["value", "status"]}}}}},
    ])
}

fn prompts() -> Value {
    json!([
        {"name": "check_address", "description": "Verify the connectivity at an address and explain the options to a customer", "arguments": [
            {"name": "address", "description": "The address in plain words (street, number, comune)", "required": true},
            {"name": "language", "description": "en or it", "required": false}]},
    ])
}

fn prompt_text(name: &str, args: &Value) -> Option<(&'static str, String)> {
    let language = args.get("language").and_then(Value::as_str).unwrap_or("it");
    match name {
        "check_address" => {
            let address = args.get("address").and_then(Value::as_str)?;
            Some((
                "Verify an address",
                format!(
                    "Verify the connectivity available at: {address}\n\n1. Split it into comune, via and civico; if unsure of the spelling, use search_comune and search_street.\n2. Call verify_coverage. If it returns candidates, list them and ask which one.\n3. Answer in {language}: the best option first (technology, provider, speed), then the alternatives, planned services with their dates, and the nearest radio bridge in reach if nothing wired is available. Quote the raw status when the traffic light is unknown. Keep it short."
                ),
            ))
        }
        _ => None,
    }
}

fn pass_through(args: &Value, keys: &[&str]) -> Value {
    let mut v = json!({});
    for k in keys {
        if let Some(x) = args.get(k) {
            v[k] = x.clone();
        }
    }
    v
}

impl Default for Server {
    fn default() -> Self {
        Self::new(Session::demo())
    }
}

impl Server {
    pub fn new(session: impl CoverSession + 'static) -> Self {
        Self { session: Box::new(session) }
    }

    pub fn session(&self) -> &dyn CoverSession {
        self.session.as_ref()
    }

    fn run(&mut self, command: &str, params: Value) -> ToolResult {
        self.session.run_blocking(command, params).map_err(|e| e.to_string())
    }

    fn italian(&self, args: &Value) -> bool {
        args.get("language").and_then(Value::as_str).map_or(self.session.italian(), |l| l.eq_ignore_ascii_case("it"))
    }

    fn with_text(&self, mut v: Value, italian: bool) -> Value {
        if let Some(r) = covercraft_engine::report_from_json(&v) {
            v["text"] = json!(text::report_text(&r, italian));
        }
        v
    }

    fn call(&mut self, name: &str, args: &Value) -> ToolResult {
        let italian = self.italian(args);
        match name {
            "verify_coverage" => {
                let v = self.run("coverage.verify", pass_through(args, &["comune", "via", "civico", "egon", "user", "log"]))?;
                if v.get("ambiguous").is_some() {
                    let list: Vec<String> = v["candidates"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|c| Some(format!("{}  (egon {})", c["display"].as_str()?, c["egon"].as_str()?)))
                        .collect();
                    let head = if italian {
                        "Più civici corrispondono: chiedi quale e richiama con `egon`"
                    } else {
                        "More than one civico matches: ask which and call again with `egon`"
                    };
                    let mut out = v;
                    out["text"] = json!(format!("{head}\n{}", list.join("\n")));
                    return Ok(out);
                }
                Ok(self.with_text(v, italian))
            }
            "search_comune" => self.run("comune.list", pass_through(args, &["query", "regione", "provincia", "regioni", "province", "limit"])),
            "search_street" => self.run("street.search", pass_through(args, &["comune", "query", "limit"])),
            "list_civici" => self.run("civico.list", pass_through(args, &["comune", "street_egon", "limit"])),
            "radio_bridges" => self.run("radio.list", json!({})),
            "history" => self.run("history.list", pass_through(args, &["user", "all", "limit"])),
            "open_history" => {
                let v = self.run("history.open", pass_through(args, &["id"]))?;
                Ok(self.with_text(v, italian))
            }
            "database_settings" => self.run("db.get", json!({})),
            "configure_database" => self.run("db.configure", pass_through(args, &["kind", "host", "port", "user", "password", "database", "path"])),
            "test_database" => self.run("db.test", json!({})),
            "get_preferences" => self.run("prefs.get", json!({})),
            "set_preferences" => self.run(
                "prefs.set",
                pass_through(
                    args,
                    &["language", "theme", "log_requests", "radio_reach_km", "max_bridges", "cells", "cells_radius_km", "max_cells", "rules"],
                ),
            ),
            _ => Err(format!("unknown tool `{}`", name.chars().take(40).collect::<String>())),
        }
    }

    fn report_text(&self) -> String {
        let italian = self.session.italian();
        match &self.session.state().report {
            Some(r) => text::report_text(r, italian),
            None => {
                if italian {
                    "Nessuna verifica ancora".into()
                } else {
                    "No verification yet".into()
                }
            }
        }
    }

    fn bridges_text(&self) -> String {
        self.session
            .state()
            .bridges
            .iter()
            .map(|b| format!("{}\t{}\t{}\t{}\t{}\t{}\t{}", b.id, b.name, b.lat, b.lon, b.alt, b.band, b.state))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Handles one JSON-RPC message; `None` for notifications.
    pub fn handle(&mut self, msg: &Value) -> Option<Value> {
        let id = msg.get("id").cloned()?;
        let method = msg.get("method").and_then(Value::as_str).unwrap_or_default();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").and_then(Value::as_str).filter(|v| SUPPORTED_VERSIONS.contains(v)).unwrap_or(PROTOCOL_VERSION),
                "capabilities": {"tools": {"listChanged": false}, "resources": {"listChanged": false, "subscribe": false}, "prompts": {"listChanged": false}},
                "serverInfo": {"name": "covercraft", "title": "CoverCraft", "version": env!("CARGO_PKG_VERSION")},
                "instructions": INSTRUCTIONS,
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({"tools": tools()})),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
                let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                Ok(match self.call(name, &args) {
                    Ok(v) => {
                        let text = match v.get("text").and_then(Value::as_str) {
                            Some(t) => t.to_string(),
                            None => serde_json::to_string_pretty(&v).unwrap_or_default(),
                        };
                        let structured = if v.is_object() { v } else { json!({"result": v}) };
                        json!({"content": [{"type": "text", "text": text}], "structuredContent": structured, "isError": false})
                    }
                    Err(e) => json!({"content": [{"type": "text", "text": e}], "isError": true}),
                })
            }
            "prompts/list" => Ok(json!({"prompts": prompts()})),
            "prompts/get" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
                let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
                match prompt_text(name, &args) {
                    Some((description, t)) => {
                        Ok(json!({"description": description, "messages": [{"role": "user", "content": {"type": "text", "text": t}}]}))
                    }
                    None => Err((-32602, format!("unknown prompt or missing arguments: {}", name.chars().take(40).collect::<String>()))),
                }
            }
            "resources/list" => Ok(json!({"resources": [
                {"uri": REPORT_URI, "name": "report", "title": "Last report", "description": "The last coverage report as text", "mimeType": "text/plain"},
                {"uri": BRIDGES_URI, "name": "bridges", "title": "Radio bridges", "description": "The radio bridges last listed (tab separated)", "mimeType": "text/plain"},
            ]})),
            "resources/templates/list" => Ok(json!({"resourceTemplates": []})),
            "resources/read" => match params.get("uri").and_then(Value::as_str) {
                Some(REPORT_URI) => Ok(json!({"contents": [{"uri": REPORT_URI, "mimeType": "text/plain", "text": self.report_text()}]})),
                Some(BRIDGES_URI) => Ok(json!({"contents": [{"uri": BRIDGES_URI, "mimeType": "text/plain", "text": self.bridges_text()}]})),
                _ => Err((-32002, "resource not found".to_string())),
            },
            _ => Err((-32601, format!("method not found: {}", method.chars().take(64).collect::<String>()))),
        };
        Some(match result {
            Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
            Err((code, message)) => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}),
        })
    }

    /// Serves until `input` ends: one JSON-RPC message per line in, one response per line out.
    pub fn serve(&mut self, input: impl BufRead, mut output: impl Write) -> std::io::Result<()> {
        for line in input.lines() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            let reply = if line.len() > MAX_LINE {
                Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32600, "message": "request too large"}}))
            } else {
                match serde_json::from_str::<Value>(&line) {
                    Ok(Value::Array(batch)) => {
                        let replies: Vec<Value> = batch.iter().filter_map(|m| self.handle(m)).collect();
                        (!replies.is_empty()).then_some(Value::Array(replies))
                    }
                    Ok(msg) => self.handle(&msg),
                    Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": format!("parse error: {e}")}})),
                }
            };
            if let Some(r) = reply {
                writeln!(output, "{r}")?;
                output.flush()?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(s: &mut Server, name: &str, args: Value) -> Value {
        let r = s.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}})).unwrap();
        assert_eq!(r["result"]["isError"], false, "{r}");
        r["result"].clone()
    }

    #[test]
    fn an_agent_verifies_an_address() {
        let mut s = Server::default();
        let init = s.handle(&json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {"protocolVersion": "2024-11-05"}})).unwrap();
        assert_eq!(init["result"]["protocolVersion"], "2024-11-05");
        assert!(s.handle(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none());
        let tools = s.handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})).unwrap();
        assert_eq!(tools["result"]["tools"].as_array().unwrap().len(), 12);
        let r = call(&mut s, "search_comune", json!({"query": "cagl"}));
        assert_eq!(r["structuredContent"]["comuni"][0]["name"], "CAGLIARI");
        let r = call(&mut s, "search_street", json!({"comune": "cagliari", "query": "roma"}));
        assert_eq!(r["structuredContent"]["streets"][0]["egon"], "10001");
        let r = call(&mut s, "verify_coverage", json!({"comune": "Cagliari", "via": "via roma", "language": "it"}));
        assert!(r["content"][0]["text"].as_str().unwrap().contains("Più civici"));
        let r = call(&mut s, "verify_coverage", json!({"egon": "100010", "user": "anna", "language": "it"}));
        let t = r["content"][0]["text"].as_str().unwrap();
        assert!(t.starts_with("VIA ROMA 10, CAGLIARI: Coperto"), "{t}");
        assert!(t.contains("Ponti radio"));
        let r = call(&mut s, "history", json!({"all": true}));
        assert_eq!(r["structuredContent"]["history"][0]["user"], "anna");
        let id = r["structuredContent"]["history"][0]["id"].as_i64().unwrap();
        let r = call(&mut s, "open_history", json!({"id": id, "language": "en"}));
        assert!(r["content"][0]["text"].as_str().unwrap().contains("Covered by"));
        let res = s.handle(&json!({"jsonrpc": "2.0", "id": 5, "method": "resources/read", "params": {"uri": REPORT_URI}})).unwrap();
        assert!(res["result"]["contents"][0]["text"].as_str().unwrap().contains("EGON 100010"));
        let db = call(&mut s, "database_settings", json!({}));
        assert!(db["structuredContent"].get("password").is_none());
        let bad = s
            .handle(&json!({"jsonrpc": "2.0", "id": 6, "method": "tools/call", "params": {"name": "verify_coverage", "arguments": {"comune": "x"}}}))
            .unwrap();
        assert_eq!(bad["result"]["isError"], true);
        let p = s.handle(&json!({"jsonrpc": "2.0", "id": 7, "method": "prompts/get", "params": {"name": "check_address", "arguments": {"address": "via Roma 10 Cagliari"}}})).unwrap();
        assert!(p["result"]["messages"][0]["content"]["text"].as_str().unwrap().contains("verify_coverage"));
        let mut out = Vec::new();
        s.serve(std::io::Cursor::new("{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"ping\"}\nnot json\n"), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("\"id\":9") && text.contains("parse error"));
    }
}
