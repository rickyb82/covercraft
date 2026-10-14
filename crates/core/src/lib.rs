//! CoverCraft's model (layer L0): addresses and EGON codes, the interpretation of the
//! providers' coverage tables (TIM/FiberCop, Open Fiber, Fastweb, the operator's own network),
//! radio bridges and mobile cells by distance, the report, the configuration.
//!
//! No database here: the [`source::CoverageSource`] trait is what the `db` crate implements
//! and [`mock::MockSource`] is the in-memory one for tests, the demo and the web snapshot.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod address;
pub mod config;
pub mod geo;
pub mod geocode;
pub mod mock;
pub mod model;
pub mod radio;
pub mod report;
pub mod source;
pub mod status;

/// Everything that can go wrong below the engine.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid {0}")]
    Invalid(String),
    #[error("database: {0}")]
    Database(String),
    #[error("not configured: {0}")]
    NotConfigured(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("i/o: {0}")]
    Io(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Bounds on hostile input (rows, names, lists).
pub mod limits {
    /// Longest text taken from a query parameter or a database cell.
    pub const TEXT: usize = 4_000;
    /// Most rows read for one answer (suggestions, scale, cells, history).
    pub const ROWS: usize = 500;
    /// Most rows a single statement may return (registries: `comuni` and `istat` hold ~8000).
    pub const QUERY_ROWS: usize = 50_000;
    /// Most radio bridges listed in a report.
    pub const BRIDGES: usize = 200;
    /// Most points of a street polyline kept.
    pub const POINTS: usize = 20_000;

    /// Clips `s` to [`TEXT`] characters on a char boundary.
    pub fn clip(s: &str) -> String {
        s.chars().take(TEXT).collect()
    }
}

/// Seconds since the Unix epoch, now.
pub fn now() -> u64 {
    web_time::SystemTime::now().duration_since(web_time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// `YYYY-MM-DD HH:MM` (UTC) of a Unix time.
pub fn format_time(secs: u64) -> String {
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", rem / 3600, (rem % 3600) / 60)
}

/// Howard Hinnant's days → civil date.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_formats() {
        assert_eq!(format_time(0), "1970-01-01 00:00");
        assert_eq!(format_time(1_760_400_000), "2025-10-14 00:00");
        assert!(now() > 1_700_000_000);
        assert_eq!(limits::clip(&"a".repeat(5000)).len(), limits::TEXT);
    }
}
