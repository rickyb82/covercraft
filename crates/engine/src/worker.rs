//! The query thread: it owns the database source and answers jobs one at a time, so the
//! interface never blocks on the network.

use std::sync::mpsc::{Receiver, Sender, channel};

use covercraft_core::config::{DbConfig, Preferences};
use covercraft_core::model::*;
use covercraft_core::source::{CoverageSource, SourceFactory};
use covercraft_core::{Error, Result, address, report};
use serde_json::Value;

/// What a job asks.
#[derive(Clone, Debug)]
pub enum JobKind {
    Connect { db: DbConfig, password: String },
    Ping,
    Comuni,
    Streets { comune: String, query: String, limit: usize },
    Civici { comune: String, street_egon: String, limit: usize },
    Resolve { comune: String, via: String, civico: String, limit: usize },
    Verify { comune: String, via: String, civico: String, egon: String, user: String, prefs: Preferences, log: bool, request: Value },
    Bridges,
}

/// What a job answers.
#[derive(Clone, Debug)]
pub enum Output {
    Connected(String),
    Pong(String),
    Comuni(Vec<Comune>),
    Streets(Vec<Street>),
    Civici(Vec<Civico>),
    /// More than one civico matched: the person picks.
    Candidates(Vec<Civico>),
    /// The report, with the request it answers (`user` inside) for the local history.
    Report(Box<Report>, Value),
    Bridges(Vec<Bridge>),
}

pub struct Job {
    pub id: u64,
    pub kind: JobKind,
}

pub struct Event {
    pub id: u64,
    pub result: Result<Output>,
}

pub struct Worker {
    tx: Sender<Job>,
    rx: Receiver<Event>,
}

struct Inner {
    factory: SourceFactory,
    source: Option<Box<dyn CoverageSource>>,
    db: DbConfig,
    password: String,
    comuni: Vec<Comune>,
}

impl Inner {
    fn source(&mut self) -> Result<&mut (dyn CoverageSource + 'static)> {
        if self.source.is_none() {
            let s = (self.factory)(&self.db, &self.password)?;
            self.source = Some(s);
        }
        match self.source.as_mut() {
            Some(s) => Ok(s.as_mut()),
            None => Err(Error::NotConfigured("database".into())),
        }
    }

    fn comuni(&mut self) -> Result<Vec<Comune>> {
        if self.comuni.is_empty() {
            self.comuni = self.source()?.comuni()?;
        }
        Ok(self.comuni.clone())
    }

    /// The comune named, by exact name, `NAME (PR)`, or a unique prefix.
    fn comune(&mut self, name: &str) -> Result<Comune> {
        let wanted = address::normalize(name);
        if wanted.is_empty() {
            return Err(Error::Invalid("comune (empty)".into()));
        }
        let (plain, prov) = match wanted.split_once(['(', ',']) {
            Some((n, p)) => (n.trim().to_string(), p.trim_matches(|c: char| c == ')' || c == ' ').to_string()),
            None => (wanted.clone(), String::new()),
        };
        let all = self.comuni()?;
        let exact: Vec<&Comune> = all.iter().filter(|c| c.name == plain && (prov.is_empty() || c.provincia == prov)).collect();
        if let Some(c) = exact.first() {
            return Ok((*c).clone());
        }
        let prefix: Vec<&Comune> = all.iter().filter(|c| c.name.starts_with(&plain) && (prov.is_empty() || c.provincia == prov)).collect();
        match prefix.as_slice() {
            [one] => Ok((*one).clone()),
            [] => Err(Error::NotFound(format!("comune `{name}`"))),
            many => Err(Error::Invalid(format!(
                "comune `{name}` matches {}: {}",
                many.len(),
                many.iter().take(8).map(|c| format!("{} ({})", c.name, c.provincia)).collect::<Vec<_>>().join(", ")
            ))),
        }
    }

    fn run(&mut self, kind: JobKind) -> Result<Output> {
        match kind {
            JobKind::Connect { db, password } => {
                self.db = db;
                self.password = password;
                self.source = None;
                self.comuni.clear();
                let s = self.source()?;
                Ok(Output::Connected(s.ping()?))
            }
            JobKind::Ping => Ok(Output::Pong(self.source()?.ping()?)),
            JobKind::Comuni => Ok(Output::Comuni(self.comuni()?)),
            JobKind::Streets { comune, query, limit } => {
                let c = self.comune(&comune)?;
                Ok(Output::Streets(self.source()?.streets(&c, &query, limit)?))
            }
            JobKind::Civici { comune, street_egon, limit } => {
                let c = self.comune(&comune)?;
                let street = Street { egon: street_egon, ..Default::default() };
                Ok(Output::Civici(self.source()?.civici(&c, &street, limit)?))
            }
            JobKind::Resolve { comune, via, civico, limit } => {
                let c = self.comune(&comune)?;
                Ok(Output::Candidates(self.source()?.resolve(&c, &via, &civico, limit)?))
            }
            JobKind::Verify { comune, via, civico, egon, user, prefs, log, request } => {
                let c = if comune.trim().is_empty() { None } else { Some(self.comune(&comune)?) };
                let target = if !egon.trim().is_empty() {
                    self.source()?.civico(c.as_ref(), &egon)?.ok_or_else(|| Error::NotFound(format!("EGON {}", egon.trim())))?
                } else {
                    let c = c.clone().ok_or_else(|| Error::Invalid("comune (needed with via and civico)".into()))?;
                    if via.trim().is_empty() {
                        return Err(Error::Invalid("via (empty)".into()));
                    }
                    let mut found = self.source()?.resolve(&c, &via, &civico, covercraft_core::limits::QUERY_ROWS)?;
                    match found.len() {
                        0 => {
                            return Err(Error::NotFound(format!(
                                "{} {} in {}",
                                address::normalize(&via),
                                address::normalize_civico(&civico),
                                c.name
                            )));
                        }
                        1 => found.remove(0),
                        _ => return Ok(Output::Candidates(found)),
                    }
                };
                let comune = match c {
                    Some(c) => c,
                    None => {
                        let all = self.comuni()?;
                        all.into_iter().find(|k| k.name == target.comune).unwrap_or_else(|| Comune {
                            name: target.comune.clone(),
                            provincia: target.provincia.clone(),
                            ..Default::default()
                        })
                    }
                };
                let r = report::gather(self.source()?, &comune, &target, &prefs)?;
                if log {
                    let answer = crate::report_json(&r);
                    if let Err(e) = self.source()?.log(&user, &request, &answer) {
                        // A failed database log never hides the answer.
                        let mut r2 = r.clone();
                        r2.notes.push(format!("log_failed: {e}"));
                        return Ok(Output::Report(Box::new(r2), request));
                    }
                }
                Ok(Output::Report(Box::new(r), request))
            }
            JobKind::Bridges => Ok(Output::Bridges(self.source()?.bridges()?)),
        }
    }
}

impl Worker {
    pub fn start(factory: SourceFactory, db: DbConfig, password: String) -> Self {
        let (tx, jobs) = channel::<Job>();
        let (events, rx) = channel::<Event>();
        std::thread::Builder::new()
            .name("covercraft-db".into())
            .spawn(move || {
                let mut inner = Inner { factory, source: None, db, password, comuni: Vec::new() };
                while let Ok(job) = jobs.recv() {
                    let kind = job.kind.clone();
                    let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| inner.run(kind))) {
                        Ok(r) => r,
                        Err(_) => {
                            inner.source = None;
                            Err(Error::Database("the query crashed; the connection was reset".into()))
                        }
                    };
                    if let Err(Error::Database(_)) = &result {
                        // Drop a connection that failed: the next job reconnects.
                        inner.source = None;
                    }
                    if events.send(Event { id: job.id, result }).is_err() {
                        break;
                    }
                }
            })
            .ok();
        Self { tx, rx }
    }

    pub fn submit(&self, job: Job) -> bool {
        self.tx.send(job).is_ok()
    }

    pub fn poll(&self) -> Vec<Event> {
        let mut out = Vec::new();
        while let Ok(e) = self.rx.try_recv() {
            out.push(e);
        }
        out
    }

    /// Waits up to `timeout` for one event.
    pub fn wait(&self, timeout: std::time::Duration) -> Option<Event> {
        self.rx.recv_timeout(timeout).ok()
    }
}
