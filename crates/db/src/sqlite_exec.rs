//! The SQLite executor (`rusqlite`, bundled).

use rusqlite::Connection;
use rusqlite::types::ValueRef;

use covercraft_core::{Error, Result};

use crate::{Executor, Table};

pub struct SqliteExecutor {
    conn: Connection,
}

impl SqliteExecutor {
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path).map_err(|e| Error::Database(format!("sqlite {path}: {e}")))?;
        Ok(Self { conn })
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| Error::Database(format!("sqlite: {e}")))?;
        Ok(Self { conn })
    }
}

fn cell(v: ValueRef<'_>) -> Option<String> {
    match v {
        ValueRef::Null => None,
        ValueRef::Integer(i) => Some(i.to_string()),
        ValueRef::Real(f) => Some(f.to_string()),
        ValueRef::Text(t) => Some(String::from_utf8_lossy(t).into_owned()),
        ValueRef::Blob(b) => Some(String::from_utf8_lossy(b).into_owned()),
    }
}

impl Executor for SqliteExecutor {
    fn query(&mut self, sql: &str, params: &[&str]) -> Result<Table> {
        let mut stmt = self.conn.prepare(sql).map_err(|e| Error::Database(format!("sqlite: {e} in `{sql}`")))?;
        let n = stmt.column_count();
        let columns: Vec<String> = stmt.column_names().iter().map(|c| (*c).to_string()).collect();
        let mut rows = stmt.query(rusqlite::params_from_iter(params.iter())).map_err(|e| Error::Database(format!("sqlite: {e}")))?;
        let mut out = Vec::new();
        while let Some(r) = rows.next().map_err(|e| Error::Database(format!("sqlite: {e}")))? {
            let mut row = Vec::with_capacity(n);
            for i in 0..n {
                row.push(r.get_ref(i).ok().and_then(cell));
            }
            out.push(row);
            if out.len() >= covercraft_core::limits::QUERY_ROWS {
                break;
            }
        }
        Ok(Table { columns, rows: out })
    }

    fn execute(&mut self, sql: &str, params: &[&str]) -> Result<u64> {
        let n = self.conn.execute(sql, rusqlite::params_from_iter(params.iter())).map_err(|e| Error::Database(format!("sqlite: {e}")))?;
        Ok(n as u64)
    }

    fn dialect(&self) -> &'static str {
        "sqlite"
    }

    fn describe(&mut self) -> Result<String> {
        Ok(format!("SQLite {}", rusqlite::version()))
    }
}
