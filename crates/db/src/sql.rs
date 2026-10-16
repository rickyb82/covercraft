//! The statements, written once for MySQL and SQLite.

use serde_json::Value;

use covercraft_core::model::*;
use covercraft_core::source::CoverageSource;
use covercraft_core::{Error, Result, address, limits};

use crate::{Executor, Table, invalid};

/// A [`CoverageSource`] over an [`Executor`].
pub struct SqlSource {
    exec: Box<dyn Executor>,
}

/// A table name from `comuni.table_name`, safe to put in a statement.
pub fn table_ident(name: &str) -> Result<String> {
    let t = name.trim();
    let ok = !t.is_empty()
        && t.len() <= 64
        && t.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '_');
    if ok { Ok(format!("`{t}`")) } else { Err(invalid(&format!("table name `{t}`"))) }
}

fn limit(n: usize) -> usize {
    n.clamp(1, limits::QUERY_ROWS)
}

fn text(row: &[Option<String>], i: usize) -> String {
    row.get(i).and_then(|c| c.as_deref()).map(|s| limits::clip(s.trim())).unwrap_or_default()
}

const TIM_CIVICO_COLS: &str = "`CODICE_EGON_CLIENTE`, `CODICE_EGON_STRADA_CLIENTE`, `PART_TOP_CLIENTE`, `STRADA_CLIENTE`, `CIVICO_CLIENTE`, `COMUNE_CLIENTE`, `PROVINCIA_CLIENTE`";

fn tim_civico(row: &[Option<String>]) -> Civico {
    Civico {
        egon: text(row, 0),
        egon_strada: text(row, 1),
        particle: text(row, 2),
        street: text(row, 3),
        civico: text(row, 4),
        comune: text(row, 5),
        provincia: text(row, 6),
        source: "tim".into(),
    }
}

const OF_CIVICO_COLS: &str = "`ID_EGON_CIVICO`, `ID_EGON_STRADA`, `PARTICELLA_TOP`, `INDIRIZZO`, `CIVICO`, `COMUNE`, `PROVINCIA`";

fn of_civico(row: &[Option<String>]) -> Civico {
    let mut c = tim_civico(row);
    c.source = "openfiber".into();
    c
}

const FW_CIVICO_COLS: &str =
    "`id_egon_civico`, `id_egon_via`, `particella_top_egon`, `nome_via_egon`, `civico_top_egon`, `comune_egon`, `sigla_provincia_egon`";

fn fw_civico(row: &[Option<String>]) -> Civico {
    let mut c = tim_civico(row);
    c.source = "fastweb".into();
    c
}

impl SqlSource {
    pub fn new(exec: Box<dyn Executor>) -> Self {
        Self { exec }
    }

    /// Runs a statement directly (schema creation, demo data).
    pub fn execute(&mut self, sql: &str, params: &[&str]) -> Result<u64> {
        self.exec.execute(sql, params)
    }

    pub fn query(&mut self, sql: &str, params: &[&str]) -> Result<Table> {
        self.exec.query(sql, params)
    }

    pub fn dialect(&self) -> &'static str {
        self.exec.dialect()
    }

    fn table_exists(&mut self, table: &str) -> bool {
        let ident = match table_ident(table) {
            Ok(i) => i,
            Err(_) => return false,
        };
        self.exec.query(&format!("SELECT 1 FROM {ident} LIMIT 1"), &[]).is_ok()
    }

    fn push_unique(out: &mut Vec<Civico>, c: Civico) {
        if !c.egon.is_empty() && !out.iter().any(|o| o.egon == c.egon) {
            out.push(c);
        }
    }
}

impl CoverageSource for SqlSource {
    fn ping(&mut self) -> Result<String> {
        let server = self.exec.describe()?;
        let t = self.exec.query("SELECT COUNT(*) FROM `comuni`", &[])?;
        let n = t.rows.first().map(|r| text(r, 0)).unwrap_or_default();
        Ok(format!("{server}: {n} comuni"))
    }

    fn comuni(&mut self) -> Result<Vec<Comune>> {
        let t = self.exec.query("SELECT `comune`, `provincia`, `table_name` FROM `comuni` ORDER BY `comune`", &[])?;
        let istat = self
            .exec
            .query("SELECT `nome_comune`, `sigla_provincia`, `codice_comune`, `nome_regione`, `nome_provincia` FROM `istat`", &[])
            .map(|t| t.rows)
            .unwrap_or_default();
        let mut out: Vec<Comune> = t
            .rows
            .iter()
            .map(|r| Comune {
                name: address::normalize(&text(r, 0)),
                provincia: address::normalize(&text(r, 1)),
                table: text(r, 2),
                ..Default::default()
            })
            .filter(|c| !c.name.is_empty())
            .collect();
        for c in &mut out {
            if let Some(r) = istat.iter().find(|r| address::normalize(&text(r, 0)) == c.name && address::normalize(&text(r, 1)) == c.provincia) {
                c.istat = text(r, 2);
                c.regione = address::normalize(&text(r, 3));
                c.provincia_name = address::normalize(&text(r, 4));
            }
        }
        Ok(out)
    }

    fn streets(&mut self, comune: &Comune, query: &str, limit_n: usize) -> Result<Vec<Street>> {
        let pattern = address::like_pattern(query);
        let n = limit(limit_n);
        let mut out: Vec<Street> = Vec::new();
        if !comune.table.is_empty() {
            let table = table_ident(&comune.table)?;
            let sql = format!(
                "SELECT DISTINCT `CODICE_EGON_STRADA_CLIENTE`, `PART_TOP_CLIENTE`, `STRADA_CLIENTE` FROM {table} WHERE UPPER(`STRADA_CLIENTE`) LIKE ? ORDER BY `STRADA_CLIENTE` LIMIT {n}"
            );
            for r in self.exec.query(&sql, &[&pattern])?.rows {
                out.push(Street { egon: text(&r, 0), particle: text(&r, 1), name: text(&r, 2) });
            }
        }
        if out.is_empty() {
            let sql = format!(
                "SELECT DISTINCT `ID_EGON_STRADA`, `PARTICELLA_TOP`, `INDIRIZZO` FROM `openfiber` WHERE UPPER(`COMUNE`) = ? AND UPPER(`INDIRIZZO`) LIKE ? ORDER BY `INDIRIZZO` LIMIT {n}"
            );
            for r in self.exec.query(&sql, &[&comune.name, &pattern])?.rows {
                out.push(Street { egon: text(&r, 0), particle: text(&r, 1), name: text(&r, 2) });
            }
        }
        Ok(out)
    }

    fn civici(&mut self, comune: &Comune, street: &Street, limit_n: usize) -> Result<Vec<Civico>> {
        let n = limit(limit_n);
        let mut out = Vec::new();
        if !comune.table.is_empty() {
            let table = table_ident(&comune.table)?;
            let sql = format!(
                "SELECT {TIM_CIVICO_COLS} FROM {table} WHERE `CODICE_EGON_STRADA_CLIENTE` = ? ORDER BY LENGTH(`CIVICO_CLIENTE`), `CIVICO_CLIENTE` LIMIT {n}"
            );
            for r in self.exec.query(&sql, &[&street.egon])?.rows {
                Self::push_unique(&mut out, tim_civico(&r));
            }
        }
        if out.is_empty() {
            let sql =
                format!("SELECT DISTINCT {OF_CIVICO_COLS} FROM `openfiber` WHERE `ID_EGON_STRADA` = ? ORDER BY LENGTH(`CIVICO`), `CIVICO` LIMIT {n}");
            for r in self.exec.query(&sql, &[&street.egon])?.rows {
                Self::push_unique(&mut out, of_civico(&r));
            }
        }
        Ok(out)
    }

    fn resolve(&mut self, comune: &Comune, via: &str, civico: &str, limit_n: usize) -> Result<Vec<Civico>> {
        let pattern = address::like_pattern(via);
        let number = address::normalize_civico(civico);
        let n = limit(limit_n);
        let mut out = Vec::new();
        if !comune.table.is_empty() {
            let table = table_ident(&comune.table)?;
            let sql = format!(
                "SELECT {TIM_CIVICO_COLS} FROM {table} WHERE UPPER(`STRADA_CLIENTE`) LIKE ? AND (? = '' OR REPLACE(UPPER(`CIVICO_CLIENTE`), ' ', '') = ?) ORDER BY `STRADA_CLIENTE`, LENGTH(`CIVICO_CLIENTE`), `CIVICO_CLIENTE` LIMIT {n}"
            );
            for r in self.exec.query(&sql, &[&pattern, &number, &number])?.rows {
                Self::push_unique(&mut out, tim_civico(&r));
            }
        }
        if out.len() < n {
            let sql = format!(
                "SELECT DISTINCT {OF_CIVICO_COLS} FROM `openfiber` WHERE UPPER(`COMUNE`) = ? AND UPPER(`INDIRIZZO`) LIKE ? AND (? = '' OR REPLACE(UPPER(`CIVICO`), ' ', '') = ?) LIMIT {n}"
            );
            for r in self.exec.query(&sql, &[&comune.name, &pattern, &number, &number])?.rows {
                Self::push_unique(&mut out, of_civico(&r));
            }
        }
        if out.len() < n {
            for table in ["ftth_fw", "ftts_fw", "vula_fw"] {
                let sql = format!(
                    "SELECT DISTINCT {FW_CIVICO_COLS} FROM `{table}` WHERE UPPER(`comune_egon`) = ? AND UPPER(`nome_via_egon`) LIKE ? AND (? = '' OR REPLACE(UPPER(`civico_top_egon`), ' ', '') = ?) LIMIT {n}"
                );
                for r in self.exec.query(&sql, &[&comune.name, &pattern, &number, &number])?.rows {
                    Self::push_unique(&mut out, fw_civico(&r));
                }
            }
        }
        out.truncate(n);
        Ok(out)
    }

    fn civico(&mut self, comune: Option<&Comune>, egon: &str) -> Result<Option<Civico>> {
        let egon = egon.trim();
        if egon.is_empty() || egon.len() > 32 {
            return Err(Error::Invalid("egon".into()));
        }
        if let Some(c) = comune.filter(|c| !c.table.is_empty()) {
            let table = table_ident(&c.table)?;
            let sql = format!("SELECT {TIM_CIVICO_COLS} FROM {table} WHERE `CODICE_EGON_CLIENTE` = ? LIMIT 1");
            if let Some(r) = self.exec.query(&sql, &[egon])?.rows.first() {
                return Ok(Some(tim_civico(r)));
            }
        }
        let sql = format!("SELECT {OF_CIVICO_COLS} FROM `openfiber` WHERE `ID_EGON_CIVICO` = ? LIMIT 1");
        if let Some(r) = self.exec.query(&sql, &[egon])?.rows.first() {
            return Ok(Some(of_civico(r)));
        }
        for table in ["ftth_fw", "ftts_fw", "vula_fw"] {
            let sql = format!("SELECT {FW_CIVICO_COLS} FROM `{table}` WHERE `id_egon_civico` = ? LIMIT 1");
            if let Some(r) = self.exec.query(&sql, &[egon])?.rows.first() {
                return Ok(Some(fw_civico(r)));
            }
        }
        if comune.is_none() {
            // Any comune's TIM table: expensive, so only when nothing else knows the code.
            let comuni = self.comuni()?;
            for c in comuni.iter().filter(|c| !c.table.is_empty()) {
                let Ok(table) = table_ident(&c.table) else { continue };
                if !self.table_exists(&c.table) {
                    continue;
                }
                let sql = format!("SELECT {TIM_CIVICO_COLS} FROM {table} WHERE `CODICE_EGON_CLIENTE` = ? LIMIT 1");
                if let Some(r) = self.exec.query(&sql, &[egon])?.rows.first() {
                    return Ok(Some(tim_civico(r)));
                }
            }
        }
        Ok(None)
    }

    fn tim(&mut self, comune: &Comune, egon: &str) -> Result<Option<TimRow>> {
        if comune.table.is_empty() {
            return Ok(None);
        }
        let table = table_ident(&comune.table)?;
        let t = self.exec.query(&format!("SELECT * FROM {table} WHERE `CODICE_EGON_CLIENTE` = ? LIMIT 1"), &[egon.trim()])?;
        Ok(t.maps().into_iter().next())
    }

    fn openfiber(&mut self, egon: &str) -> Result<Vec<OpenFiberRow>> {
        let sql = format!(
            "SELECT `ID_SCALA`, `SCALA_PALAZZINA`, `ID_BUILDING`, `COORDINATE_BUILDING`, `POP`, `TOTALE_UI`, `STATO_BUILDING`, `STATO_SCALA_PALAZZINA`, `DATA_RFC_INDICATIVA`, `DATA_RFC_EFFETTIVA`, `DATA_RFA_INDICATIVA`, `DATA_RFA_EFFETTIVA`, `CLUSTER`, `DATA_ULTIMA_MODIFICA_RECORD` FROM `openfiber` WHERE `ID_EGON_CIVICO` = ? ORDER BY `ID_SCALA` LIMIT {}",
            limits::ROWS
        );
        Ok(self
            .exec
            .query(&sql, &[egon.trim()])?
            .rows
            .iter()
            .map(|r| OpenFiberRow {
                id_scala: text(r, 0),
                scala: text(r, 1),
                building: text(r, 2),
                coordinates: text(r, 3),
                pop: text(r, 4),
                units: text(r, 5),
                building_state: text(r, 6),
                scala_state: text(r, 7),
                rfc_planned: text(r, 8),
                rfc_actual: text(r, 9),
                rfa_planned: text(r, 10),
                rfa_actual: text(r, 11),
                cluster: text(r, 12),
                updated: text(r, 13),
            })
            .collect())
    }

    fn openfiber_statuses(&mut self) -> Result<Vec<OpenFiberStatus>> {
        let t = self.exec.query("SELECT `value`, `cluster`, `description`, `friendly_description` FROM `openfiber_status`", &[])?;
        Ok(t.rows
            .iter()
            .map(|r| OpenFiberStatus { value: text(r, 0).parse().unwrap_or(-1), cluster: text(r, 1), description: text(r, 2), friendly: text(r, 3) })
            .collect())
    }

    fn xgspon_pops(&mut self) -> Result<Vec<String>> {
        let t = self.exec.query("SELECT `pop` FROM `openfiber_xgpon_pop`", &[])?;
        Ok(t.rows.iter().map(|r| text(r, 0)).collect())
    }

    fn fastweb(&mut self, egon: &str) -> Result<FastwebRows> {
        let mut out = FastwebRows::default();
        let egon = egon.trim();
        let n = 50;
        out.ftth = self.exec.query(&format!("SELECT * FROM `ftth_fw` WHERE `id_egon_civico` = ? LIMIT {n}"), &[egon])?.maps();
        out.ftts = self.exec.query(&format!("SELECT * FROM `ftts_fw` WHERE `id_egon_civico` = ? LIMIT {n}"), &[egon])?.maps();
        out.vula = self.exec.query(&format!("SELECT * FROM `vula_fw` WHERE `id_egon_civico` = ? LIMIT {n}"), &[egon])?.maps();
        out.gpon = self.exec.query(&format!("SELECT * FROM `gpon_fw` WHERE `id_egon_civico` = ? LIMIT {n}"), &[egon])?.maps();
        Ok(out)
    }

    fn own(&mut self, egon: &str) -> Result<Vec<OwnService>> {
        let sql = format!(
            "SELECT COALESCE(t.`name`, n.`id_tipo_servizio`), COALESCE(s.`name`, n.`id_stato`), n.`data_stato_corrente`, n.`note` FROM `sirius_netmap` n LEFT JOIN `sirius_tipi_servizio` t ON t.`id_tipo_servizio` = n.`id_tipo_servizio` LEFT JOIN `sirius_stati` s ON s.`id_stato` = n.`id_stato` WHERE n.`egon_civico` = ? LIMIT {}",
            limits::ROWS
        );
        Ok(self
            .exec
            .query(&sql, &[egon.trim()])?
            .rows
            .iter()
            .map(|r| OwnService { service: text(r, 0), state: text(r, 1), since: text(r, 2), note: text(r, 3) })
            .collect())
    }

    fn polyline(&mut self, egon_strada: &str) -> Result<Option<String>> {
        let t = self.exec.query("SELECT `polylines` FROM `sirius_egon_polylines` WHERE `egon_strada` = ? LIMIT 1", &[egon_strada.trim()])?;
        Ok(t.rows.first().and_then(|r| r.first().cloned().flatten()).map(|p| p.chars().take(limits::POINTS * 24).collect()))
    }

    fn street_coordinates(&mut self, egon_strada: &str, limit_n: usize) -> Result<Vec<String>> {
        let sql = format!(
            "SELECT `COORDINATE_BUILDING` FROM `openfiber` WHERE `ID_EGON_STRADA` = ? AND `COORDINATE_BUILDING` IS NOT NULL AND `COORDINATE_BUILDING` <> '' LIMIT {}",
            limit(limit_n)
        );
        Ok(self.exec.query(&sql, &[egon_strada.trim()])?.rows.iter().map(|r| text(r, 0)).filter(|c| !c.is_empty()).collect())
    }

    fn bridges(&mut self) -> Result<Vec<Bridge>> {
        let sql = format!(
            "SELECT `id_ponte`, `nome`, `lat`, `lon`, `alt`, `stato`, `descrizione`, `fascia` FROM `radio_table` ORDER BY `id_ponte` LIMIT {}",
            limits::BRIDGES
        );
        Ok(self
            .exec
            .query(&sql, &[])?
            .rows
            .iter()
            .map(|r| Bridge {
                id: text(r, 0).parse().unwrap_or(0),
                name: text(r, 1),
                lat: text(r, 2),
                lon: text(r, 3),
                alt: text(r, 4),
                state: text(r, 5),
                description: text(r, 6),
                band: text(r, 7),
            })
            .collect())
    }

    fn cells(&mut self, lat: f64, lon: f64, half_deg: f64, limit_n: usize) -> Result<Vec<Cell>> {
        let half = half_deg.clamp(0.0005, 1.0);
        let (la0, la1, lo0, lo1) = (lat - half, lat + half, lon - half, lon + half);
        let sql = format!(
            "SELECT `radio`, `mcc`, `net`, `area`, `cell`, `lat`, `lon`, `range`, `samples` FROM `cell_db` WHERE (`lat` + 0) BETWEEN (? + 0) AND (? + 0) AND (`lon` + 0) BETWEEN (? + 0) AND (? + 0) LIMIT {}",
            limit(limit_n)
        );
        let p = [la0.to_string(), la1.to_string(), lo0.to_string(), lo1.to_string()];
        let params: Vec<&str> = p.iter().map(String::as_str).collect();
        Ok(self
            .exec
            .query(&sql, &params)?
            .rows
            .iter()
            .map(|r| Cell {
                radio: text(r, 0),
                mcc: text(r, 1),
                net: text(r, 2),
                area: text(r, 3),
                cell: text(r, 4),
                lat: text(r, 5),
                lon: text(r, 6),
                range: text(r, 7),
                samples: text(r, 8),
                source: "db".into(),
            })
            .collect())
    }

    fn cell(&mut self, query: &covercraft_core::cells::CellQuery) -> Result<Vec<Cell>> {
        // The columns are text as the export writes them: compare as numbers so `010` finds `10`.
        let mut sql = "SELECT `radio`, `mcc`, `net`, `area`, `cell`, `lat`, `lon`, `range`, `samples` FROM `cell_db` WHERE (`mcc` + 0) = (? + 0) AND (`net` + 0) = (? + 0) AND (`cell` + 0) = (? + 0)".to_string();
        let mut p = vec![query.mcc.clone(), query.net.clone(), query.cell.clone()];
        if query.has_area() {
            sql.push_str(" AND (`area` + 0) = (? + 0)");
            p.push(query.area.clone());
        }
        if !query.radio.is_empty() {
            sql.push_str(" AND UPPER(`radio`) = ?");
            p.push(query.radio.to_ascii_uppercase());
        }
        sql.push_str(" LIMIT 20");
        let params: Vec<&str> = p.iter().map(String::as_str).collect();
        Ok(self
            .exec
            .query(&sql, &params)?
            .rows
            .iter()
            .map(|r| Cell {
                radio: text(r, 0),
                mcc: text(r, 1),
                net: text(r, 2),
                area: text(r, 3),
                cell: text(r, 4),
                lat: text(r, 5),
                lon: text(r, 6),
                range: text(r, 7),
                samples: text(r, 8),
                source: "db".into(),
            })
            .collect())
    }

    fn log(&mut self, user: &str, request: &Value, answer: &Value) -> Result<()> {
        let req = request.to_string();
        let ans = answer.to_string();
        self.exec
            .execute("INSERT INTO `requests_log` (`user`, `request_json`, `answer_json`) VALUES (?, ?, ?)", &[&limits::clip(user), &req, &ans])?;
        Ok(())
    }

    fn history(&mut self, user: &str, limit_n: usize) -> Result<Vec<HistoryEntry>> {
        let sql = format!(
            "SELECT `id`, `request_date`, `user`, `request_json`, `answer_json` FROM `requests_log` WHERE (? = '' OR `user` = ?) ORDER BY `id` DESC LIMIT {}",
            limit(limit_n)
        );
        Ok(self
            .exec
            .query(&sql, &[user.trim(), user.trim()])?
            .rows
            .iter()
            .map(|r| HistoryEntry {
                id: text(r, 0).parse().unwrap_or(0),
                date: text(r, 1),
                user: text(r, 2),
                request: serde_json::from_str(&r.get(3).cloned().flatten().unwrap_or_default()).unwrap_or(Value::Null),
                answer: serde_json::from_str(&r.get(4).cloned().flatten().unwrap_or_default()).unwrap_or(Value::Null),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_names_are_checked() {
        assert_eq!(table_ident("netmap_0000953650178cf38c94f42160f0bb42").unwrap(), "`netmap_0000953650178cf38c94f42160f0bb42`");
        assert!(table_ident("netmap; DROP TABLE comuni").is_err());
        assert!(table_ident("").is_err());
        assert!(table_ident("1abc").is_err());
        assert!(table_ident(&"a".repeat(65)).is_err());
    }
}
