//! A [`CoverSession`] that lives in a browser (or any client) and mirrors a `covercraft-cli
//! serve` server: commands go to `POST /api/cmd`, state comes back from
//! `GET /api/snapshot?since=N` (only when it changed). Works natively and on wasm32 through `ehttp`.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use covercraft_engine::core::config::Config;
use covercraft_engine::{Completed, CoverSession, Error, Result, Snapshot, State};
use serde_json::{Value, json};

enum Incoming {
    Snapshot(Box<Snapshot>),
    Unchanged,
    Answer { command: String, result: std::result::Result<Value, String> },
    Failed(String),
}

type Inbox = Arc<Mutex<Vec<Incoming>>>;

pub struct RemoteSession {
    base: String,
    token: String,
    user: String,
    config: Config,
    has_password: bool,
    state: State,
    revision: u64,
    jobs_pending: usize,
    inbox: Inbox,
    polling: bool,
    problem: Option<String>,
    in_flight: usize,
    poll_now: bool,
    last_poll: f64,
    commands_by_job: BTreeMap<u64, String>,
}

fn now_seconds() -> f64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

const IDLE_POLL: f64 = 3.0;
const BUSY_POLL: f64 = 0.3;

impl RemoteSession {
    /// Connects to `base` (`http://127.0.0.1:7979`) with an optional bearer token and user name.
    pub fn new(base: &str, token: &str, user: &str) -> Self {
        let mut s = Self {
            base: base.trim_end_matches('/').to_string(),
            token: token.to_string(),
            user: user.to_string(),
            config: Config::default(),
            has_password: false,
            state: State::default(),
            revision: 0,
            jobs_pending: 0,
            inbox: Arc::default(),
            polling: false,
            problem: Some("connecting".into()),
            in_flight: 0,
            poll_now: true,
            last_poll: 0.0,
            commands_by_job: BTreeMap::new(),
        };
        s.poll();
        s
    }

    fn headers(&self, json: bool) -> ehttp::Headers {
        let mut pairs: Vec<(&str, String)> = vec![("Accept", "application/json".into())];
        if json {
            pairs.push(("Content-Type", "application/json".into()));
        }
        if !self.token.is_empty() {
            pairs.push(("Authorization", format!("Bearer {}", self.token)));
        }
        if !self.user.is_empty() {
            pairs.push(("X-Covercraft-User", self.user.clone()));
        }
        let borrowed: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
        ehttp::Headers::new(&borrowed)
    }

    fn poll(&mut self) {
        if self.polling {
            return;
        }
        self.polling = true;
        self.poll_now = false;
        self.last_poll = now_seconds();
        let inbox = self.inbox.clone();
        let mut req = ehttp::Request::get(format!("{}/api/snapshot?since={}", self.base, self.revision));
        req.headers = self.headers(false);
        ehttp::fetch(req, move |result| {
            let incoming = match result {
                Ok(r) if r.ok => match serde_json::from_slice::<Value>(&r.bytes) {
                    Ok(v) if v.get("unchanged").is_some() => Incoming::Unchanged,
                    Ok(v) => match serde_json::from_value::<Snapshot>(v) {
                        Ok(s) => Incoming::Snapshot(Box::new(s)),
                        Err(e) => Incoming::Failed(format!("bad snapshot: {e}")),
                    },
                    Err(e) => Incoming::Failed(format!("bad snapshot: {e}")),
                },
                Ok(r) => Incoming::Failed(format!("server answered {} {}", r.status, r.status_text)),
                Err(e) => Incoming::Failed(e),
            };
            if let Ok(mut q) = inbox.lock() {
                q.push(incoming);
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn refresh_blocking(&mut self) -> Result<Option<Snapshot>> {
        let mut req = ehttp::Request::get(format!("{}/api/snapshot?since={}", self.base, self.revision));
        req.headers = self.headers(false);
        let r = ehttp::fetch_blocking(&req).map_err(|e| Error::Core(covercraft_engine::core::Error::Database(e)))?;
        if !r.ok {
            return Err(Error::Param(format!("server answered {} {}", r.status, r.status_text)));
        }
        let v: Value = serde_json::from_slice(&r.bytes).map_err(|e| Error::Param(format!("bad snapshot: {e}")))?;
        if v.get("unchanged").is_some() {
            return Ok(None);
        }
        let s: Snapshot = serde_json::from_value(v).map_err(|e| Error::Param(format!("bad snapshot: {e}")))?;
        self.apply(s.clone());
        Ok(Some(s))
    }

    fn apply(&mut self, s: Snapshot) {
        self.revision = s.revision;
        self.config = s.config;
        self.has_password = s.has_password;
        self.state = s.state;
        self.jobs_pending = s.jobs_pending;
        if !s.user.is_empty() {
            self.user = s.user;
        }
        self.problem = None;
    }
}

impl CoverSession for RemoteSession {
    fn config(&self) -> &Config {
        &self.config
    }
    fn state(&self) -> &State {
        &self.state
    }
    fn has_password(&self) -> bool {
        self.has_password
    }
    fn user(&self) -> String {
        self.user.clone()
    }
    fn jobs_pending(&self) -> usize {
        self.jobs_pending + self.in_flight
    }
    fn problem(&self) -> Option<String> {
        self.problem.clone()
    }
    fn server(&self) -> String {
        self.base.clone()
    }
    fn set_server(&mut self, url: &str) {
        self.base = url.trim_end_matches('/').to_string();
        self.revision = 0;
        self.problem = Some("connecting".into());
        self.poll_now = true;
    }

    fn execute(&mut self, id: &str, params: &Value) -> Result<Value> {
        let body = json!({"command": id, "params": params}).to_string().into_bytes();
        let mut req = ehttp::Request::post(format!("{}/api/cmd", self.base), body);
        req.headers = self.headers(true);
        let inbox = self.inbox.clone();
        let command = id.to_string();
        self.in_flight += 1;
        ehttp::fetch(req, move |result| {
            let incoming = match result {
                Ok(r) => match serde_json::from_slice::<Value>(&r.bytes) {
                    Ok(v) if v.get("error").is_some() => {
                        Incoming::Answer { command, result: Err(v["error"].as_str().unwrap_or("error").to_string()) }
                    }
                    Ok(v) => Incoming::Answer { command, result: Ok(v.get("result").cloned().unwrap_or(Value::Null)) },
                    Err(e) => Incoming::Answer { command, result: Err(format!("bad answer: {e}")) },
                },
                Err(e) => Incoming::Failed(e),
            };
            if let Ok(mut q) = inbox.lock() {
                q.push(incoming);
            }
        });
        Ok(json!({"queued": true}))
    }

    fn pump(&mut self) -> Vec<Completed> {
        let incoming: Vec<Incoming> = self.inbox.lock().map(|mut q| std::mem::take(&mut *q)).unwrap_or_default();
        let mut out = Vec::new();
        for i in incoming {
            match i {
                Incoming::Snapshot(s) => {
                    self.polling = false;
                    for c in &s.completed {
                        let mut done = Completed::from(c);
                        if done.command.is_empty() {
                            done.command = self.commands_by_job.remove(&c.job).unwrap_or_default();
                        } else {
                            self.commands_by_job.remove(&c.job);
                        }
                        out.push(done);
                    }
                    self.apply(*s);
                }
                Incoming::Unchanged => {
                    self.polling = false;
                    self.problem = None;
                }
                Incoming::Answer { command, result } => {
                    self.in_flight = self.in_flight.saturating_sub(1);
                    self.poll_now = true;
                    match result {
                        Ok(v) => {
                            if let Some(j) = v.get("job").and_then(Value::as_u64) {
                                self.commands_by_job.insert(j, command.clone());
                            } else {
                                out.push(Completed { job: 0, command, result: Ok(v) });
                            }
                        }
                        Err(e) => out.push(Completed { job: 0, command, result: Err(Error::Param(e)) }),
                    }
                }
                Incoming::Failed(e) => {
                    self.polling = false;
                    self.in_flight = self.in_flight.saturating_sub(1).min(self.in_flight);
                    self.problem = Some(e);
                }
            }
        }
        let now = now_seconds();
        let interval = if self.jobs_pending > 0 || self.in_flight > 0 { BUSY_POLL } else { IDLE_POLL };
        if self.poll_now || now - self.last_poll >= interval || (self.problem.is_some() && now - self.last_poll >= IDLE_POLL) {
            self.poll();
        }
        out
    }

    #[cfg(target_arch = "wasm32")]
    fn run_blocking(&mut self, id: &str, params: Value) -> Result<Value> {
        let _ = self.execute(id, &params)?;
        Err(Error::Param("remote sessions answer through pump(), not run_blocking".into()))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn run_blocking(&mut self, id: &str, params: Value) -> Result<Value> {
        let body = json!({"command": id, "params": params}).to_string().into_bytes();
        let mut req = ehttp::Request::post(format!("{}/api/cmd", self.base), body);
        req.headers = self.headers(true);
        let r = ehttp::fetch_blocking(&req).map_err(|e| Error::Core(covercraft_engine::core::Error::Database(e)))?;
        let v: Value = serde_json::from_slice(&r.bytes).map_err(|e| Error::Param(format!("bad answer: {e}")))?;
        if let Some(e) = v.get("error").and_then(Value::as_str) {
            return Err(Error::Param(e.to_string()));
        }
        let v = v.get("result").cloned().unwrap_or(Value::Null);
        let Some(job) = v.get("job").and_then(Value::as_u64) else {
            self.refresh_blocking()?;
            return Ok(v);
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(180);
        loop {
            if std::time::Instant::now() > deadline {
                return Err(Error::Timeout(job));
            }
            std::thread::sleep(std::time::Duration::from_millis(120));
            let Some(s) = self.refresh_blocking()? else { continue };
            if let Some(c) = s.completed.iter().find(|c| c.job == job) {
                return if c.error.is_empty() { Ok(c.result.clone()) } else { Err(Error::Param(c.error.clone())) };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreachable_server_is_a_problem_not_a_panic() {
        let mut s = RemoteSession::new("http://127.0.0.1:1", "", "anna");
        assert_eq!(s.problem(), Some("connecting".into()));
        for _ in 0..50 {
            let _ = s.pump();
            std::thread::sleep(std::time::Duration::from_millis(20));
            if s.problem().is_some_and(|p| p != "connecting") {
                break;
            }
        }
        assert!(s.problem().is_some_and(|p| p != "connecting"), "{:?}", s.problem());
        assert!(s.execute("state.get", &json!({})).is_ok());
        assert!(s.run_blocking("state.get", json!({})).is_err());
        assert_eq!(s.user(), "anna");
    }
}
