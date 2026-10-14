//! Database access (layer L1): the `netmap` schema queried through one set of SQL statements
//! that run on MySQL (the real server) and on SQLite (tests and demo files) alike.
//!
//! Every value is read as text, as the tables store it; `core` interprets it. Table names of
//! the per-comune TIM tables come from the `comuni` table and are checked against a strict
//! pattern before they are put in a statement.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod demo;
pub mod geocoder;
mod mysql_exec;
mod sql;
mod sqlite_exec;

use covercraft_core::config::DbConfig;
use covercraft_core::source::{CoverageSource, SourceFactory};
use covercraft_core::{Error, Result};

pub use sql::SqlSource;

/// A row: the cells as text, `None` for NULL.
pub type Row = Vec<Option<String>>;

/// A query's answer: column names and rows.
#[derive(Clone, Debug, Default)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
}

impl Table {
    /// The rows as maps (column → text, NULL as empty).
    pub fn maps(&self) -> Vec<covercraft_core::model::TimRow> {
        self.rows.iter().map(|r| self.columns.iter().cloned().zip(r.iter().map(|c| c.clone().unwrap_or_default())).collect()).collect()
    }
}

/// Runs statements; one per engine.
pub trait Executor: Send {
    /// A `SELECT` with `?` placeholders.
    fn query(&mut self, sql: &str, params: &[&str]) -> Result<Table>;
    /// An `INSERT`/`UPDATE`.
    fn execute(&mut self, sql: &str, params: &[&str]) -> Result<u64>;
    /// `mysql` or `sqlite`.
    fn dialect(&self) -> &'static str;
    /// A line about the server.
    fn describe(&mut self) -> Result<String>;
}

/// Opens the source the configuration names.
pub fn open(db: &DbConfig, password: &str) -> Result<Box<dyn CoverageSource>> {
    db.validate()?;
    match db.kind.as_str() {
        "mysql" => Ok(Box::new(SqlSource::new(Box::new(mysql_exec::MySqlExecutor::connect(db, password)?)))),
        "sqlite" => Ok(Box::new(SqlSource::new(Box::new(sqlite_exec::SqliteExecutor::open(&db.path)?)))),
        _ => Ok(Box::new(covercraft_core::mock::MockSource::demo())),
    }
}

/// The factory apps hand to the engine.
pub fn factory() -> SourceFactory {
    std::sync::Arc::new(open)
}

/// The geocoder factory apps hand to the engine (Nominatim over HTTP).
pub fn geocoder_factory() -> covercraft_core::geocode::GeocoderFactory {
    std::sync::Arc::new(|url| {
        let url = url.trim().trim_end_matches('/');
        (!url.is_empty()).then(|| Box::new(geocoder::Nominatim::new(url)) as Box<dyn covercraft_core::geocode::Geocoder>)
    })
}

/// A source on an in-memory SQLite copy of the schema, filled with the demo city: the demo
/// mode of the apps, where the real SQL runs.
pub fn demo_sqlite() -> Result<SqlSource> {
    let exec = sqlite_exec::SqliteExecutor::in_memory()?;
    let mut src = SqlSource::new(Box::new(exec));
    demo::fill(&mut src)?;
    Ok(src)
}

fn invalid(what: &str) -> Error {
    Error::Invalid(what.to_string())
}
