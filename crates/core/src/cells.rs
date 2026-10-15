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
    fn merging_skips_the_cells_the_table_has() {
        let mk = |net: &str, cell: &str| Cell { mcc: "222".into(), net: net.into(), area: "1".into(), cell: cell.into(), ..Default::default() };
        let mut table = vec![mk("10", "1")];
        let added = merge(&mut table, vec![mk("10", "1"), mk("88", "2")]);
        assert_eq!((added, table.len()), (1, 2));
    }
}
