//! Live cells from OpenCelliD's area service (`/cell/getInArea`): the request and the
//! answer's reading are here; the HTTP call is injected (the `db` crate makes it natively).
//! The service takes a box of at most 4 km² and answers at most 50 cells per call, so the
//! lookup asks for a 2 km × 2 km box around the point, page after page.

use serde_json::Value;

use crate::geo::Point;
use crate::model::Cell;

/// Something that lists the cells around a point (OpenCelliD, or a test double).
pub trait CellLookup: Send {
    fn cells_near(&mut self, point: Point, limit: usize) -> crate::Result<Vec<Cell>>;
    /// One cell by its identifiers (`/cell/get`); `None` when the service does not know it.
    fn cell(&mut self, _query: &CellQuery) -> crate::Result<Option<Cell>> {
        Ok(None)
    }
}

/// A cell named by its identifiers: country (MCC), network (MNC), area (LAC or TAC) and
/// cell id (CID; for LTE the ECI, eNodeB × 256 + sector).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CellQuery {
    pub mcc: String,
    pub net: String,
    pub area: String,
    pub cell: String,
    /// `GSM`, `UMTS`, `LTE`, `NR`, or empty for any.
    pub radio: String,
}

fn digits(v: &Value, keys: &[&str]) -> String {
    keys.iter()
        .find_map(|k| v.get(k))
        .map(|x| match x {
            Value::String(s) => s.trim().to_string(),
            Value::Null => String::new(),
            other => other.to_string(),
        })
        .unwrap_or_default()
}

impl CellQuery {
    /// From command parameters: `mcc` (default 222), `net`/`mnc`, `area`/`lac`/`tac` (optional: the
    /// table is searched without it, the service needs it), `cell`/`cid` or `enb` + `sector`
    /// (LTE: ECI = eNodeB × 256 + sector), `radio`.
    pub fn from_params(v: &Value) -> crate::Result<Self> {
        let number = |s: &str, what: &str| -> crate::Result<u64> {
            let t = s.trim().trim_start_matches('0');
            let t = if t.is_empty() && !s.trim().is_empty() { "0" } else { t };
            if t.is_empty() {
                return Err(crate::Error::Invalid(format!("{what} is required")));
            }
            t.parse::<u64>().map_err(|_| crate::Error::Invalid(format!("{what} must be a number, got {s:?}")))
        };
        let mcc = digits(v, &["mcc"]);
        let mcc = if mcc.is_empty() { 222 } else { number(&mcc, "mcc")? };
        let net = number(&digits(v, &["net", "mnc"]), "net (MNC)")?;
        let area_text = digits(v, &["area", "lac", "tac"]);
        let area = if area_text.is_empty() || area_text.trim() == "0" { None } else { Some(number(&area_text, "area (LAC/TAC)")?) };
        let cell_text = digits(v, &["cell", "cellid", "cid", "eci"]);
        let cell = if cell_text.is_empty() {
            let enb = number(&digits(v, &["enb", "enodeb"]), "cell (CID/ECI) or enb + sector")?;
            let sector = number(&digits(v, &["sector"]), "sector")?;
            if sector > 255 {
                return Err(crate::Error::Invalid("sector: 0-255".into()));
            }
            enb.checked_mul(256).and_then(|x| x.checked_add(sector)).ok_or_else(|| crate::Error::Invalid("enb too large".into()))?
        } else {
            number(&cell_text, "cell")?
        };
        let radio = digits(v, &["radio"]).to_ascii_uppercase();
        if !radio.is_empty() && !["GSM", "UMTS", "LTE", "NR", "CDMA"].contains(&radio.as_str()) {
            return Err(crate::Error::Invalid("radio: GSM, UMTS, LTE or NR".into()));
        }
        Ok(Self { mcc: mcc.to_string(), net: net.to_string(), area: area.map(|a| a.to_string()).unwrap_or_default(), cell: cell.to_string(), radio })
    }

    /// `222-10-100-12345` (`?` for an unknown area).
    pub fn label(&self) -> String {
        format!("{}-{}-{}-{}", self.mcc, self.net, if self.area.is_empty() { "?" } else { self.area.as_str() }, self.cell)
    }

    /// The area (LAC/TAC) is known: OpenCelliD's single-cell service needs it; the table does not.
    pub fn has_area(&self) -> bool {
        !self.area.is_empty()
    }

    /// Whether a cell (from the table or the service) is the one asked.
    pub fn matches(&self, c: &Cell) -> bool {
        let same = |a: &str, b: &str| a.trim().trim_start_matches('0') == b.trim().trim_start_matches('0');
        same(&c.mcc, &self.mcc)
            && same(&c.net, &self.net)
            && (self.area.is_empty() || same(&c.area, &self.area))
            && same(&c.cell, &self.cell)
            && (self.radio.is_empty() || c.radio.eq_ignore_ascii_case(&self.radio))
    }

    /// For LTE: the eNodeB and the sector of the ECI. Only when the cell is LTE/NR, or its id
    /// does not fit a 2G/3G CI (16 bits): a plain CI is not split, or it reads as LTE.
    pub fn enb_sector(&self) -> Option<(u64, u64)> {
        let eci: u64 = self.cell.parse().ok()?;
        let lte = matches!(self.radio.as_str(), "LTE" | "NR") || eci > 65_535;
        lte.then_some((eci / 256, eci % 256))
    }
}

/// The single-cell service's base URL.
pub const GET_URL: &str = "https://opencellid.org/cell/get";

/// The query string of `/cell/get`.
pub fn cell_query(token: &str, q: &CellQuery) -> String {
    let mut s = format!(
        "key={}&mcc={}&mnc={}&lac={}&cellid={}&format=json",
        crate::geocode::url_encode(token.trim()),
        crate::geocode::url_encode(&q.mcc),
        crate::geocode::url_encode(&q.net),
        crate::geocode::url_encode(&q.area),
        crate::geocode::url_encode(&q.cell)
    );
    if !q.radio.is_empty() {
        s.push_str(&format!("&radio={}", crate::geocode::url_encode(&q.radio)));
    }
    s
}

/// The cell in a `/cell/get` answer; `None` when the service says it does not know it.
pub fn parse_cell_answer(text: &str) -> crate::Result<Option<Cell>> {
    let v: Value = serde_json::from_str(text.get(..text.len().min(100_000)).unwrap_or_default())
        .map_err(|e| crate::Error::Invalid(format!("opencellid: not JSON: {e}")))?;
    if let Some(err) = v.get("error").and_then(Value::as_str) {
        let code = v.get("code").and_then(Value::as_i64).unwrap_or(0);
        if code == 1 || err.to_ascii_lowercase().contains("not found") {
            return Ok(None);
        }
        return Err(crate::Error::Invalid(format!("opencellid: {err} (code {code})")));
    }
    let cells = parse_area_answer(&json_array_of(&v))?;
    Ok(cells.into_iter().next())
}

fn json_array_of(v: &Value) -> String {
    Value::Array(vec![v.clone()]).to_string()
}

/// Makes a lookup for an API token (`None` when the token is empty); apps inject the real one.
pub type CellLookupFactory = std::sync::Arc<dyn Fn(&str) -> Option<Box<dyn CellLookup>> + Send + Sync>;

/// The service's base URL.
pub const AREA_URL: &str = "https://opencellid.org/cell/getInArea";
/// Half the box side in degrees of latitude (about 1 km: the box stays under 4 km²).
pub const HALF_DEG: f64 = 0.009;
/// Cells per page (the service's maximum).
pub const PAGE: usize = 50;
/// Pages asked at most for one point.
pub const MAX_PAGES: usize = 4;

/// The query string of one page: `BBOX=latmin,lonmin,latmax,lonmax`, JSON.
pub fn area_query(token: &str, p: Point, offset: usize) -> String {
    let lon_half = (HALF_DEG / p.lat.to_radians().cos().abs().max(0.2)).min(0.05);
    format!(
        "key={}&BBOX={:.5},{:.5},{:.5},{:.5}&format=json&limit={PAGE}&offset={offset}",
        crate::geocode::url_encode(token.trim()),
        (p.lat - HALF_DEG).clamp(-90.0, 90.0),
        (p.lon - lon_half).clamp(-180.0, 180.0),
        (p.lat + HALF_DEG).clamp(-90.0, 90.0),
        (p.lon + lon_half).clamp(-180.0, 180.0),
    )
}

/// The cells in one answer (`{"cells": [...]}` or a bare array); an `error` field is an error.
pub fn parse_area_answer(text: &str) -> crate::Result<Vec<Cell>> {
    let v: Value = serde_json::from_str(text.get(..text.len().min(4_000_000)).unwrap_or_default())
        .map_err(|e| crate::Error::Invalid(format!("opencellid: not JSON: {e}")))?;
    if let Some(err) = v.get("error").and_then(Value::as_str) {
        return Err(crate::Error::Invalid(format!("opencellid: {err} (code {})", v.get("code").map(Value::to_string).unwrap_or_default())));
    }
    let list = v.get("cells").and_then(Value::as_array).or_else(|| v.as_array());
    let text_of = |c: &Value, keys: &[&str]| -> String {
        keys.iter()
            .find_map(|k| c.get(k))
            .map(|x| match x {
                Value::String(s) => s.trim().to_string(),
                Value::Null => String::new(),
                other => other.to_string(),
            })
            .unwrap_or_default()
    };
    Ok(list
        .into_iter()
        .flatten()
        .take(crate::limits::ROWS)
        .map(|c| Cell {
            radio: text_of(c, &["radio"]),
            mcc: text_of(c, &["mcc"]),
            net: text_of(c, &["mnc", "net"]),
            area: text_of(c, &["lac", "area", "tac"]),
            cell: text_of(c, &["cellid", "cell", "cid"]),
            lat: text_of(c, &["lat", "latitude"]),
            lon: text_of(c, &["lon", "longitude"]),
            range: text_of(c, &["range"]),
            samples: text_of(c, &["samples"]),
            source: "opencellid".into(),
        })
        .filter(|c| c.point().is_some())
        .collect())
}

/// Adds the live cells to the table's, skipping the ones the table already has (same
/// network, area and cell id).
pub fn merge(table: &mut Vec<Cell>, live: Vec<Cell>) -> usize {
    let mut added = 0;
    for c in live {
        if table.len() >= crate::limits::ROWS {
            break;
        }
        if !table.iter().any(|t| t.mcc == c.mcc && t.net == c.net && t.area == c.area && t.cell == c.cell) {
            table.push(c);
            added += 1;
        }
    }
    added
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_query_stays_in_a_small_box() {
        let q = area_query("k y", Point::new(39.22, 9.12).unwrap(), 50);
        assert!(q.starts_with("key=k+y&BBOX=39.21100,9.10838,39.22900,9.13162&format=json&limit=50&offset=50"), "{q}");
        let polar = area_query("k", Point::new(89.9, 0.0).unwrap(), 0);
        assert!(polar.contains("BBOX=89.89100,-0.04500,89.90900,0.04500"), "{polar}");
    }

    #[test]
    fn reads_the_answer_and_errors() {
        let text = r#"{"cells":[{"lat":39.22,"lon":9.12,"mcc":222,"mnc":10,"lac":100,"cellid":12345,"averageSignalStrength":-80,"range":1200,"samples":25,"changeable":1,"radio":"LTE"},{"lat":"x","lon":1,"mcc":222,"mnc":1,"lac":1,"cellid":1}],"count":2}"#;
        let cells = parse_area_answer(text).unwrap();
        assert_eq!(cells.len(), 1, "the cell without a point is dropped");
        let c = &cells[0];
        assert_eq!(
            (c.mcc.as_str(), c.net.as_str(), c.area.as_str(), c.cell.as_str(), c.range.as_str(), c.radio.as_str(), c.source.as_str()),
            ("222", "10", "100", "12345", "1200", "LTE", "opencellid")
        );
        assert!(parse_area_answer(r#"{"error":"Invalid API key","code":2}"#).unwrap_err().to_string().contains("Invalid API key"));
        assert!(parse_area_answer("<html>").is_err());
        assert!(parse_area_answer("[]").unwrap().is_empty());
        assert!(parse_area_answer(r#"{"cells":"no"}"#).unwrap().is_empty());
    }

    #[test]
    fn a_cell_query_reads_identifiers_and_the_single_cell_answer() {
        let q = CellQuery::from_params(&serde_json::json!({"net": "10", "area": 100, "cell": "12345"})).unwrap();
        assert_eq!((q.mcc.as_str(), q.net.as_str(), q.area.as_str(), q.cell.as_str(), q.radio.as_str()), ("222", "10", "100", "12345", ""));
        let q = CellQuery::from_params(&serde_json::json!({"mcc": "222", "mnc": "01", "tac": "7", "enb": 100, "sector": 3, "radio": "lte"})).unwrap();
        assert_eq!((q.cell.as_str(), q.radio.as_str(), q.enb_sector()), ("25603", "LTE", Some((100, 3))));
        assert_eq!(q.label(), "222-1-7-25603");
        assert!(CellQuery::from_params(&serde_json::json!({"net": "x"})).is_err());
        let no_area = CellQuery::from_params(&serde_json::json!({"net": 10, "cell": 12633651})).unwrap();
        assert!(!no_area.has_area() && no_area.label() == "222-10-?-12633651" && no_area.enb_sector() == Some((49350, 51)));
        assert!(no_area.matches(&Cell { mcc: "222".into(), net: "10".into(), area: "7".into(), cell: "12633651".into(), ..Default::default() }));
        assert!(CellQuery::from_params(&serde_json::json!({"net": 1, "area": 1})).is_err(), "no cell");
        assert!(CellQuery::from_params(&serde_json::json!({"net": 1, "area": 1, "enb": 1, "sector": 300})).is_err());
        assert!(CellQuery::from_params(&serde_json::json!({"net": 1, "area": 1, "cell": 1, "radio": "WIFI"})).is_err());
        let c = Cell { mcc: "222".into(), net: "010".into(), area: "100".into(), cell: "12345".into(), radio: "LTE".into(), ..Default::default() };
        let q = CellQuery { mcc: "222".into(), net: "10".into(), area: "100".into(), cell: "12345".into(), radio: String::new() };
        assert!(q.matches(&c));
        assert_eq!(q.enb_sector(), None, "a 16-bit id without a radio is a 2G/3G CI");
        assert_eq!(CellQuery { radio: "LTE".into(), ..q.clone() }.enb_sector(), Some((48, 57)));
        assert!(!CellQuery { radio: "GSM".into(), ..q.clone() }.matches(&c));
        let url = cell_query("k", &q);
        assert_eq!(url, "key=k&mcc=222&mnc=10&lac=100&cellid=12345&format=json");
        let found =
            parse_cell_answer(r#"{"lat":39.22,"lon":9.12,"mcc":222,"mnc":10,"lac":100,"cellid":12345,"range":1200,"samples":25,"radio":"LTE"}"#)
                .unwrap()
                .unwrap();
        assert_eq!((found.lat.as_str(), found.source.as_str()), ("39.22", "opencellid"));
        assert!(parse_cell_answer(r#"{"error":"cell not found","code":1}"#).unwrap().is_none());
        assert!(parse_cell_answer(r#"{"error":"Invalid API key","code":2}"#).is_err());
        assert!(parse_cell_answer("<html>").is_err());
    }

    #[test]
    fn merging_skips_the_cells_the_table_has() {
        let mk = |net: &str, cell: &str| Cell { mcc: "222".into(), net: net.into(), area: "1".into(), cell: cell.into(), ..Default::default() };
        let mut table = vec![mk("10", "1")];
        let added = merge(&mut table, vec![mk("10", "1"), mk("88", "2")]);
        assert_eq!((added, table.len()), (1, 2));
    }
}
