//! What the engine asks a database: implemented by the `db` crate over MySQL (the real
//! `netmap` database) and SQLite (tests, demo), and by [`crate::mock::MockSource`] in memory.

use serde_json::Value;

use crate::Result;
use crate::model::{Bridge, Cell, Civico, Comune, FastwebRows, HistoryEntry, OpenFiberRow, OpenFiberStatus, OwnService, Street, TimRow};

pub trait CoverageSource: Send {
    /// A short description of the connection (host, database, server version).
    fn ping(&mut self) -> Result<String>;
    /// Every comune with a TIM table (`comuni`).
    fn comuni(&mut self) -> Result<Vec<Comune>>;
    /// Streets of a comune whose name contains `query` (empty = the first ones).
    fn streets(&mut self, comune: &Comune, query: &str, limit: usize) -> Result<Vec<Street>>;
    /// Civici of a street.
    fn civici(&mut self, comune: &Comune, street: &Street, limit: usize) -> Result<Vec<Civico>>;
    /// Civici matching a typed street and number, across the providers' tables.
    fn resolve(&mut self, comune: &Comune, via: &str, civico: &str, limit: usize) -> Result<Vec<Civico>>;
    /// The civico by its EGON code, from any table.
    fn civico(&mut self, comune: Option<&Comune>, egon: &str) -> Result<Option<Civico>>;
    fn tim(&mut self, comune: &Comune, egon: &str) -> Result<Option<TimRow>>;
    fn openfiber(&mut self, egon: &str) -> Result<Vec<OpenFiberRow>>;
    fn openfiber_statuses(&mut self) -> Result<Vec<OpenFiberStatus>>;
    fn xgspon_pops(&mut self) -> Result<Vec<String>>;
    fn fastweb(&mut self, egon: &str) -> Result<FastwebRows>;
    fn own(&mut self, egon: &str) -> Result<Vec<OwnService>>;
    /// The street's polyline text (`sirius_egon_polylines`).
    fn polyline(&mut self, egon_strada: &str) -> Result<Option<String>>;
    /// Coordinates of other Open Fiber buildings on the same street (`COORDINATE_BUILDING`).
    fn street_coordinates(&mut self, egon_strada: &str, limit: usize) -> Result<Vec<String>>;
    fn bridges(&mut self) -> Result<Vec<Bridge>>;
    /// Cells within `half_deg` degrees of the point (a bounding box; the caller measures).
    fn cells(&mut self, lat: f64, lon: f64, half_deg: f64, limit: usize) -> Result<Vec<Cell>>;
    /// Records a verification (`requests_log`).
    fn log(&mut self, user: &str, request: &Value, answer: &Value) -> Result<()>;
    /// Past verifications, newest first (`user` empty = everyone's).
    fn history(&mut self, user: &str, limit: usize) -> Result<Vec<HistoryEntry>>;
}

/// Makes a source from the configuration; apps inject the real one.
pub type SourceFactory = std::sync::Arc<dyn Fn(&crate::config::DbConfig, &str) -> Result<Box<dyn CoverageSource>> + Send + Sync>;
