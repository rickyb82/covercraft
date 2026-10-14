//! CoverCraft's engine (layer L5): the session that holds the configuration and the state,
//! the command table every front end calls, the query worker, and the snapshot a browser
//! mirrors.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod cmd;
pub mod text;
mod worker;

use std::collections::{BTreeMap, VecDeque};
use std::time::Duration;

pub use covercraft_core as core;
use covercraft_core::config::{Config, Paths, Secrets};
use covercraft_core::model::*;
use covercraft_core::source::SourceFactory;
use serde_json::{Value, json};

pub use cmd::{COMMANDS, Command};
pub use worker::{JobKind, Output};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Core(#[from] covercraft_core::Error),
    #[error("{0}")]
    Param(String),
    #[error("unknown command `{0}`")]
    UnknownCommand(String),
    #[error("job {0} timed out")]
    Timeout(u64),
    #[error("the query worker is gone")]
    WorkerGone,
}

pub type Result<T> = std::result::Result<T, Error>;

/// What the interfaces show, mirrored to the browser as JSON.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct State {
    pub comuni: Vec<Comune>,
    /// Street suggestions for the last `street.search`.
    pub streets: Vec<Street>,
    /// Civici of the last `civico.list` or the candidates of an ambiguous verification.
    pub civici: Vec<Civico>,
    pub report: Option<Report>,
    pub history: Vec<HistoryEntry>,
    pub bridges: Vec<Bridge>,
    /// `connected`, `error`, `unknown`.
    pub db_state: String,
    pub db_message: String,
    /// The last error of a job (translated by the interface).
    pub last_error: String,
}

/// A finished job, as the interfaces see it.
#[derive(Debug)]
pub struct Completed {
    pub job: u64,
    pub command: String,
    pub result: Result<Value>,
}

/// The same, in a snapshot.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CompletedJson {
    pub job: u64,
    pub command: String,
    pub result: Value,
    pub error: String,
}

impl From<&CompletedJson> for Completed {
    fn from(c: &CompletedJson) -> Self {
        let result = if c.error.is_empty() { Ok(c.result.clone()) } else { Err(Error::Param(c.error.clone())) };
        Self { job: c.job, command: c.command.clone(), result }
    }
}

/// The state at a revision, for the web app.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Snapshot {
    pub revision: u64,
    pub config: Config,
    pub has_password: bool,
    pub user: String,
    pub state: State,
    pub jobs_pending: usize,
    pub completed: Vec<CompletedJson>,
}

/// A session, local or remote: what the interfaces are written against.
pub trait CoverSession {
    fn config(&self) -> &Config;
    fn state(&self) -> &State;
    fn has_password(&self) -> bool;
    fn user(&self) -> String;
    fn jobs_pending(&self) -> usize;
    fn execute(&mut self, id: &str, params: &Value) -> Result<Value>;
    fn pump(&mut self) -> Vec<Completed>;
    fn run_blocking(&mut self, id: &str, params: Value) -> Result<Value>;
    fn problem(&self) -> Option<String> {
        None
    }
    fn server(&self) -> String {
        String::new()
    }
    fn set_server(&mut self, _url: &str) {}
    fn italian(&self) -> bool {
        self.config().preferences.language.starts_with("it")
    }
}

/// The local session.
pub struct Session {
    config: Config,
    secrets: Secrets,
    paths: Option<Paths>,
    worker: worker::Worker,
    state: State,
    user: String,
    revision: u64,
    next_job: u64,
    pending: BTreeMap<u64, String>,
    completed: VecDeque<(u64, CompletedJson)>,
}

impl Session {
    /// A session on the configuration under `paths` (or defaults), with the source factory.
    pub fn new(factory: SourceFactory, paths: Option<Paths>) -> Self {
        let (config, secrets) = match &paths {
            Some(p) => (Config::load(&p.config).unwrap_or_default(), Secrets::load(&p.secrets).unwrap_or_default()),
            None => (Config::default(), Secrets::default()),
        };
        let worker = worker::Worker::start(factory, config.db.clone(), secrets.db_password.clone());
        Self {
            config,
            secrets,
            paths,
            worker,
            state: State { db_state: "unknown".into(), ..Default::default() },
            user: String::new(),
            revision: 1,
            next_job: 1,
            pending: BTreeMap::new(),
            completed: VecDeque::new(),
        }
    }

    /// The demo session on the in-memory city.
    pub fn demo() -> Self {
        let factory: SourceFactory = std::sync::Arc::new(|_, _| Ok(Box::new(covercraft_core::mock::MockSource::demo())));
        let mut s = Self::new(factory, None);
        s.config.db.kind = "demo".into();
        s
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn has_password(&self) -> bool {
        !self.secrets.db_password.is_empty()
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn jobs_pending(&self) -> usize {
        self.pending.len()
    }

    pub fn italian(&self) -> bool {
        self.config.preferences.language.starts_with("it")
    }

    /// Who verifies (for the log); the server sets it per request.
    pub fn set_user(&mut self, user: &str) {
        self.user = covercraft_core::limits::clip(user.trim());
    }

    pub fn user(&self) -> String {
        self.user.clone()
    }

    /// The state since `since`; `None` when nothing changed.
    pub fn snapshot(&self, since: u64) -> Option<Snapshot> {
        if since != 0 && since == self.revision {
            return None;
        }
        Some(Snapshot {
            revision: self.revision,
            config: self.config.clone(),
            has_password: self.has_password(),
            user: self.user.clone(),
            state: self.state.clone(),
            jobs_pending: self.pending.len(),
            completed: self.completed.iter().filter(|(rev, _)| *rev > since).map(|(_, c)| c.clone()).collect(),
        })
    }

    fn save(&mut self) -> Result<()> {
        if let Some(p) = &self.paths {
            self.config.save(&p.config)?;
            self.secrets.save(&p.secrets)?;
        }
        Ok(())
    }

    fn touch(&mut self) {
        self.revision += 1;
    }

    /// Runs a command: local ones answer at once, the others queue a job (`{"job": id}`).
    pub fn execute(&mut self, id: &str, params: &Value) -> Result<Value> {
        let cmd = COMMANDS.iter().find(|c| c.id == id).ok_or_else(|| Error::UnknownCommand(id.to_string()))?;
        let params = if params.is_object() { params.clone() } else { json!({}) };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (cmd.run)(self, &params)));
        match result {
            Ok(r) => r,
            Err(_) => Err(Error::Param(format!("command `{id}` crashed"))),
        }
    }

    fn submit(&mut self, command: &str, kind: JobKind) -> Result<u64> {
        let id = self.next_job;
        self.next_job += 1;
        if !self.worker.submit(worker::Job { id, kind }) {
            return Err(Error::WorkerGone);
        }
        self.pending.insert(id, command.to_string());
        self.touch();
        Ok(id)
    }

    /// Reconnects the worker to the configured database.
    fn reconnect(&mut self) -> Result<u64> {
        self.state.db_state = "unknown".into();
        self.submit("db.configure", JobKind::Connect { db: self.config.db.clone(), password: self.secrets.db_password.clone() })
    }

    /// Applies finished jobs; the interfaces call it every frame.
    pub fn pump(&mut self) -> Vec<Completed> {
        let events = self.worker.poll();
        events.into_iter().map(|e| self.apply(e)).collect()
    }

    /// Runs a command and waits for its job (CLI, MCP).
    pub fn run_blocking(&mut self, id: &str, params: Value) -> Result<Value> {
        let v = self.execute(id, &params)?;
        match v.get("job").and_then(Value::as_u64) {
            Some(j) => self.wait(j, Duration::from_secs(180)),
            None => Ok(v),
        }
    }

    /// Waits for a job to finish.
    pub fn wait(&mut self, id: u64, timeout: Duration) -> Result<Value> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            for c in self.pump() {
                if c.job == id {
                    return c.result;
                }
            }
            if !self.pending.contains_key(&id) {
                if let Some((_, c)) = self.completed.iter().find(|(_, c)| c.job == id) {
                    return Completed::from(c).result;
                }
                return Err(Error::Param(format!("job {id} unknown")));
            }
            let left = deadline.saturating_duration_since(std::time::Instant::now());
            if left.is_zero() {
                return Err(Error::Timeout(id));
            }
            if let Some(e) = self.worker.wait(left.min(Duration::from_millis(200))) {
                let c = self.apply(e);
                if c.job == id {
                    return c.result;
                }
            }
        }
    }

    fn apply(&mut self, e: worker::Event) -> Completed {
        let command = self.pending.remove(&e.id).unwrap_or_default();
        let result: Result<Value> = match e.result {
            Ok(out) => Ok(self.apply_output(&command, out)),
            Err(err) => {
                let msg = err.to_string();
                self.state.last_error = msg.clone();
                if matches!(command.as_str(), "db.configure" | "db.test")
                    || matches!(err, covercraft_core::Error::Database(_) | covercraft_core::Error::NotConfigured(_))
                {
                    self.state.db_state = "error".into();
                    self.state.db_message = msg;
                }
                Err(Error::Core(err))
            }
        };
        self.touch();
        let json = CompletedJson {
            job: e.id,
            command: command.clone(),
            result: result.as_ref().ok().cloned().unwrap_or(Value::Null),
            error: result.as_ref().err().map(ToString::to_string).unwrap_or_default(),
        };
        self.completed.push_back((self.revision, json));
        while self.completed.len() > 200 {
            self.completed.pop_front();
        }
        Completed { job: e.id, command, result }
    }

    fn apply_output(&mut self, command: &str, out: Output) -> Value {
        self.state.last_error.clear();
        match out {
            Output::Connected(info) | Output::Pong(info) => {
                self.state.db_state = "connected".into();
                self.state.db_message = info.clone();
                json!({"ok": true, "info": info, "database": self.config.db.display()})
            }
            Output::Comuni(list) => {
                self.state.comuni = list;
                json!({"comuni": self.state.comuni.iter().map(comune_json).collect::<Vec<_>>()})
            }
            Output::Streets(list) => {
                self.state.streets = list;
                json!({"streets": self.state.streets.iter().map(street_json).collect::<Vec<_>>()})
            }
            Output::Civici(list) => {
                self.state.civici = list;
                json!({"civici": self.state.civici.iter().map(civico_json).collect::<Vec<_>>()})
            }
            Output::Candidates(list) => {
                self.state.civici = list;
                let n = self.state.civici.len();
                json!({"candidates": self.state.civici.iter().map(civico_json).collect::<Vec<_>>(), "count": n, "ambiguous": n != 1})
            }
            Output::Report(r) => {
                let v = report_json(&r);
                if command == "coverage.verify" {
                    self.state.report = Some(*r);
                    self.state.civici.clear();
                }
                v
            }
            Output::Bridges(list) => {
                self.state.bridges = list;
                json!({"bridges": self.state.bridges.iter().map(bridge_json).collect::<Vec<_>>()})
            }
            Output::History(list) => {
                self.state.history = list;
                json!({"history": self.state.history.iter().map(history_json).collect::<Vec<_>>()})
            }
        }
    }
}

impl CoverSession for Session {
    fn config(&self) -> &Config {
        &self.config
    }
    fn state(&self) -> &State {
        &self.state
    }
    fn has_password(&self) -> bool {
        self.has_password()
    }
    fn user(&self) -> String {
        self.user.clone()
    }
    fn jobs_pending(&self) -> usize {
        self.jobs_pending()
    }
    fn execute(&mut self, id: &str, params: &Value) -> Result<Value> {
        Session::execute(self, id, params)
    }
    fn pump(&mut self) -> Vec<Completed> {
        Session::pump(self)
    }
    fn run_blocking(&mut self, id: &str, params: Value) -> Result<Value> {
        Session::run_blocking(self, id, params)
    }
}

pub fn comune_json(c: &Comune) -> Value {
    json!({"name": c.name, "provincia": c.provincia, "provincia_name": c.provincia_name, "regione": c.regione, "istat": c.istat, "has_tim": !c.table.is_empty(), "display": c.display()})
}

pub fn street_json(s: &Street) -> Value {
    json!({"egon": s.egon, "particle": s.particle, "name": s.name, "display": s.display()})
}

pub fn civico_json(c: &Civico) -> Value {
    json!({"egon": c.egon, "egon_strada": c.egon_strada, "particle": c.particle, "street": c.street, "civico": c.civico, "comune": c.comune, "provincia": c.provincia, "source": c.source, "display": c.display()})
}

pub fn bridge_json(b: &Bridge) -> Value {
    json!({"id": b.id, "name": b.name, "lat": b.lat, "lon": b.lon, "alt": b.alt, "state": b.state, "description": b.description, "band": b.band})
}

pub fn history_json(h: &HistoryEntry) -> Value {
    let summary = h.answer.get("summary").cloned().unwrap_or(Value::Null);
    json!({"id": h.id, "date": h.date, "user": h.user, "request": h.request, "summary": summary})
}

/// The report as JSON: the structure plus a one-line summary and the best option.
pub fn report_json(r: &Report) -> Value {
    let mut v = serde_json::to_value(r).unwrap_or(Value::Null);
    if let Some(o) = v.as_object_mut() {
        o.insert("summary".into(), json!(text::summary(r, false)));
        o.insert("summary_it".into(), json!(text::summary(r, true)));
        o.insert("best".into(), r.best().and_then(|s| serde_json::to_value(s).ok()).unwrap_or(Value::Null));
        o.insert("text".into(), json!(text::report_text(r, false)));
    }
    v
}

/// The report back from a stored answer (`history.open`).
pub fn report_from_json(v: &Value) -> Option<Report> {
    serde_json::from_value(v.clone()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use covercraft_core::status::Status;

    fn run(s: &mut Session, id: &str, p: Value) -> Result<Value> {
        s.run_blocking(id, p)
    }

    #[test]
    fn verifies_an_address_end_to_end() {
        let mut s = Session::demo();
        assert_eq!(s.state().db_state, "unknown");
        let v = run(&mut s, "db.test", json!({})).unwrap();
        assert!(v["ok"].as_bool().unwrap());
        assert_eq!(s.state().db_state, "connected");
        let v = run(&mut s, "comune.list", json!({})).unwrap();
        assert_eq!(v["comuni"].as_array().unwrap().len(), 5);
        let v = run(&mut s, "comune.list", json!({"query": "qua"})).unwrap();
        assert_eq!(v["comuni"].as_array().unwrap().len(), 1);
        let v = run(&mut s, "comune.list", json!({"regione": "sardegna", "provincia": "CA"})).unwrap();
        assert_eq!(v["comuni"].as_array().unwrap().len(), 2);
        let v = run(&mut s, "comune.list", json!({"regioni": true})).unwrap();
        assert_eq!(v["regioni"].as_array().unwrap().len(), 2);
        let v = run(&mut s, "comune.list", json!({"regione": "Lazio", "province": true})).unwrap();
        assert_eq!(v["province"][0]["sigla"], "RM");
        let v = run(&mut s, "street.search", json!({"comune": "cagliari", "query": "ro"})).unwrap();
        assert_eq!(v["streets"][0]["display"], "VIA ROMA");
        let v = run(&mut s, "civico.list", json!({"comune": "Cagliari (CA)", "street_egon": "10001"})).unwrap();
        assert_eq!(v["civici"].as_array().unwrap().len(), 4);
        // Ambiguous: every civico of the street comes back as candidates.
        let v = run(&mut s, "coverage.verify", json!({"comune": "cagliari", "via": "via roma"})).unwrap();
        assert_eq!(v["ambiguous"], true);
        assert_eq!(s.state().civici.len(), 4);
        assert!(s.state().report.is_none());
        s.set_user("anna");
        let v = run(&mut s, "coverage.verify", json!({"comune": "cagliari", "via": "via roma", "civico": "10"})).unwrap();
        assert_eq!(v["best"]["technology"], "FTTH XGS-PON");
        assert!(v["summary_it"].as_str().unwrap().contains("Coperto"), "{}", v["summary_it"]);
        let r = s.state().report.clone().unwrap();
        assert_eq!(r.best().map(|b| b.status), Some(Status::Covered));
        assert!(s.state().civici.is_empty());
        let v = run(&mut s, "coverage.verify", json!({"egon": "100300"})).unwrap();
        assert_eq!(v["civico"]["street"], "MARCONI");
        let v = run(&mut s, "history.list", json!({})).unwrap();
        assert_eq!(v["history"].as_array().unwrap().len(), 2);
        assert_eq!(v["history"][0]["user"], "anna");
        let id = v["history"][1]["id"].as_i64().unwrap();
        let v = run(&mut s, "history.open", json!({"id": id})).unwrap();
        assert_eq!(v["civico"]["civico"], "10");
        assert_eq!(s.state().report.as_ref().map(|r| r.civico.egon.as_str()), Some("100010"));
        let v = run(&mut s, "radio.list", json!({})).unwrap();
        assert_eq!(v["bridges"].as_array().unwrap().len(), 5);
        assert!(run(&mut s, "coverage.verify", json!({"comune": "atlantide", "via": "x", "civico": "1"})).is_err());
        assert!(run(&mut s, "coverage.verify", json!({"comune": "cagliari", "via": "via nessuna", "civico": "1"})).is_err());
        assert!(run(&mut s, "coverage.verify", json!({"comune": "sas", "via": "x", "civico": "1"})).is_err());
        let v = run(&mut s, "state.get", json!({})).unwrap();
        assert_eq!(v["db_state"], "connected");
        assert!(s.snapshot(0).is_some());
        let rev = s.revision();
        assert!(s.snapshot(rev).is_none());
    }

    #[test]
    fn configuration_commands_and_persistence() {
        let dir = std::env::temp_dir().join(format!("covercraft-engine-{}", std::process::id()));
        let factory: SourceFactory = std::sync::Arc::new(|db, pw| {
            if db.kind == "mysql" && pw == "secret" {
                Ok(Box::new(covercraft_core::mock::MockSource::demo()))
            } else if db.kind == "mysql" {
                Err(covercraft_core::Error::Database("access denied".into()))
            } else {
                Ok(Box::new(covercraft_core::mock::MockSource::demo()))
            }
        });
        let mut s = Session::new(factory.clone(), Some(Paths::under(&dir)));
        let v = run(&mut s, "db.get", json!({})).unwrap();
        assert_eq!(v["kind"], "demo");
        assert!(run(&mut s, "db.configure", json!({"kind": "mysql", "host": "db", "user": "u", "database": "netmap", "password": "wrong"})).is_err());
        assert_eq!(s.state().db_state, "error");
        assert!(s.state().db_message.contains("access denied"));
        let v = run(&mut s, "db.configure", json!({"password": "secret"})).unwrap();
        assert!(v["ok"].as_bool().unwrap());
        assert_eq!(s.state().db_state, "connected");
        let v = run(&mut s, "db.get", json!({})).unwrap();
        assert_eq!((v["kind"].as_str(), v["has_password"].as_bool(), v.get("password")), (Some("mysql"), Some(true), None));
        run(&mut s, "prefs.set", json!({"language": "it", "max_bridges": 3, "radio_reach_km": {"D": 2}})).unwrap();
        assert!(s.italian());
        assert!(run(&mut s, "prefs.set", json!({"bogus": 1})).is_err());
        drop(s);
        let s2 = Session::new(factory, Some(Paths::under(&dir)));
        assert_eq!(s2.config().db.kind, "mysql");
        assert!(s2.has_password());
        assert_eq!(s2.config().preferences.max_bridges, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_command_survives_empty_and_hostile_params() {
        let mut s = Session::demo();
        let hostile = [
            json!({}),
            json!(null),
            json!([1, 2]),
            json!({"comune": "", "via": "", "civico": "", "egon": "", "query": "", "limit": -1}),
            json!({"comune": 7, "via": true, "civico": {"a": 1}, "limit": "x", "id": "no"}),
            json!({"comune": "x".repeat(10_000), "via": "%%%", "civico": "'; DROP TABLE comuni; --", "limit": 1_000_000_000_u64}),
            json!({"kind": "mysql", "port": "zzz", "radio_reach_km": {"ZZZZ": -1}, "rules": "no", "user": ["a"]}),
        ];
        for c in COMMANDS {
            for p in &hostile {
                let _ = s.run_blocking(c.id, p.clone());
            }
        }
        assert!(run(&mut s, "db.test", json!({})).is_ok());
        assert!(s.execute("nope", &json!({})).is_err());
    }

    #[test]
    fn a_broken_database_reports_and_recovers() {
        let flaky = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let f2 = flaky.clone();
        let factory: SourceFactory = std::sync::Arc::new(move |_, _| {
            let mut m = covercraft_core::mock::MockSource::demo();
            if f2.load(std::sync::atomic::Ordering::SeqCst) {
                m.failing = Some("connection refused".into());
            }
            Ok(Box::new(m))
        });
        let mut s = Session::new(factory, None);
        let e = run(&mut s, "comune.list", json!({})).unwrap_err().to_string();
        assert!(e.contains("connection refused"), "{e}");
        assert_eq!(s.state().db_state, "error");
        flaky.store(false, std::sync::atomic::Ordering::SeqCst);
        assert!(run(&mut s, "comune.list", json!({})).is_ok());
        assert!(s.state().last_error.is_empty());
    }
}
