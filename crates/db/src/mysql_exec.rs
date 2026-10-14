//! The MySQL executor (`mysql` crate, rustls).

use mysql::prelude::Queryable;
use mysql::{Conn, OptsBuilder, Value};

use covercraft_core::config::DbConfig;
use covercraft_core::{Error, Result};

use crate::{Executor, Table};

pub struct MySqlExecutor {
    conn: Conn,
    opts: OptsBuilder,
}

impl MySqlExecutor {
    pub fn connect(db: &DbConfig, password: &str) -> Result<Self> {
        let opts = OptsBuilder::new()
            .ip_or_hostname(Some(db.host.trim()))
            .tcp_port(db.port)
            .user(Some(db.user.trim()))
            .pass(Some(password))
            .db_name(Some(db.database.trim()))
            .tcp_connect_timeout(Some(std::time::Duration::from_secs(10)))
            .read_timeout(Some(std::time::Duration::from_secs(60)))
            .prefer_socket(false);
        let conn = Conn::new(opts.clone()).map_err(|e| Error::Database(format!("mysql {}:{}: {e}", db.host, db.port)))?;
        Ok(Self { conn, opts })
    }

    /// Reconnects once after a dropped connection.
    fn ensure(&mut self) -> Result<()> {
        if self.conn.ping().is_ok() {
            return Ok(());
        }
        self.conn = Conn::new(self.opts.clone()).map_err(|e| Error::Database(format!("mysql: {e}")))?;
        Ok(())
    }
}

fn cell(v: Value) -> Option<String> {
    match v {
        Value::NULL => None,
        Value::Bytes(b) => Some(String::from_utf8_lossy(&b).into_owned()),
        Value::Int(i) => Some(i.to_string()),
        Value::UInt(u) => Some(u.to_string()),
        Value::Float(f) => Some(f.to_string()),
        Value::Double(f) => Some(f.to_string()),
        Value::Date(y, m, d, h, i, s, _) => Some(format!("{y:04}-{m:02}-{d:02} {h:02}:{i:02}:{s:02}")),
        Value::Time(neg, d, h, m, s, _) => Some(format!("{}{}:{m:02}:{s:02}", if neg { "-" } else { "" }, d * 24 + u32::from(h))),
    }
}

impl Executor for MySqlExecutor {
    fn query(&mut self, sql: &str, params: &[&str]) -> Result<Table> {
        self.ensure()?;
        let params: Vec<Value> = params.iter().map(|p| Value::from(*p)).collect();
        let rows: Vec<mysql::Row> = self.conn.exec(sql, params).map_err(|e| Error::Database(format!("mysql: {e}")))?;
        let columns: Vec<String> = rows.first().map(|r| r.columns_ref().iter().map(|c| c.name_str().into_owned()).collect()).unwrap_or_default();
        let rows = rows
            .into_iter()
            .take(covercraft_core::limits::QUERY_ROWS)
            .map(|r| {
                let n = r.len();
                let mut r = r;
                (0..n).map(|i| r.take::<Value, usize>(i).and_then(cell)).collect()
            })
            .collect();
        Ok(Table { columns, rows })
    }

    fn execute(&mut self, sql: &str, params: &[&str]) -> Result<u64> {
        self.ensure()?;
        let params: Vec<Value> = params.iter().map(|p| Value::from(*p)).collect();
        self.conn.exec_drop(sql, params).map_err(|e| Error::Database(format!("mysql: {e}")))?;
        Ok(self.conn.affected_rows())
    }

    fn dialect(&self) -> &'static str {
        "mysql"
    }

    fn describe(&mut self) -> Result<String> {
        self.ensure()?;
        let v: Option<String> = self.conn.query_first("SELECT VERSION()").map_err(|e| Error::Database(format!("mysql: {e}")))?;
        Ok(format!("MySQL {}", v.unwrap_or_default()))
    }
}
