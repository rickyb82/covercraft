//! An OpenCelliD client: `GET /cell/getInArea?key=…&BBOX=…`, page after page.

use covercraft_core::cells::{AREA_URL, CellLookup, MAX_PAGES, PAGE, area_query, parse_area_answer};
use covercraft_core::geo::Point;
use covercraft_core::model::Cell;
use covercraft_core::{Error, Result};

pub struct OpenCellId {
    token: String,
    base: String,
}

impl OpenCellId {
    pub fn new(token: &str) -> Self {
        Self { token: token.trim().to_string(), base: AREA_URL.to_string() }
    }

    /// The same against another server (tests).
    pub fn at(token: &str, base: &str) -> Self {
        Self { token: token.trim().to_string(), base: base.trim_end_matches('/').to_string() }
    }

    fn page(&self, p: Point, offset: usize) -> Result<Vec<Cell>> {
        let url = format!("{}?{}", self.base, area_query(&self.token, p, offset));
        let mut req = ehttp::Request::get(url);
        req.headers = ehttp::Headers::new(&[("Accept", "application/json"), ("User-Agent", concat!("covercraft/", env!("CARGO_PKG_VERSION")))]);
        let r = ehttp::fetch_blocking(&req).map_err(|e| Error::Database(format!("opencellid: {}", e.trim_start_matches("io: "))))?;
        let text = String::from_utf8_lossy(&r.bytes);
        if !r.ok && !text.trim_start().starts_with('{') {
            return Err(Error::Database(format!("opencellid: HTTP {} {}", r.status, r.status_text)));
        }
        parse_area_answer(&text)
    }
}

impl CellLookup for OpenCellId {
    fn cells_near(&mut self, p: Point, limit: usize) -> Result<Vec<Cell>> {
        let mut out = Vec::new();
        for page in 0..MAX_PAGES {
            let cells = self.page(p, page * PAGE)?;
            let n = cells.len();
            out.extend(cells);
            if n < PAGE || out.len() >= limit {
                break;
            }
        }
        out.truncate(limit);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Answers the first page full (50 cells) and the second with one.
    fn serve(listener: TcpListener) {
        for stream in listener.incoming().take(2) {
            let Ok(mut s) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let cell =
                |id: usize| format!(r#"{{"lat":39.22,"lon":9.12,"mcc":222,"mnc":10,"lac":1,"cellid":{id},"range":500,"samples":3,"radio":"LTE"}}"#);
            let body = if req.contains("offset=0") {
                format!(r#"{{"cells":[{}],"count":51}}"#, (0..50).map(cell).collect::<Vec<_>>().join(","))
            } else {
                format!(r#"{{"cells":[{}],"count":51}}"#, cell(50))
            };
            let _ = s.write_all(
                format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                    .as_bytes(),
            );
        }
    }

    #[test]
    fn pages_until_a_short_page() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || serve(listener));
        let mut o = OpenCellId::at("tok", &format!("http://{addr}/cell/getInArea"));
        let cells = o.cells_near(Point::new(39.22, 9.12).unwrap(), 500).unwrap();
        assert_eq!(cells.len(), 51);
        assert_eq!(cells[50].cell, "50");
        let mut dead = OpenCellId::at("tok", "http://127.0.0.1:1");
        assert!(dead.cells_near(Point::new(39.22, 9.12).unwrap(), 10).is_err());
        assert!(crate::cell_lookup_factory()("").is_none());
        assert!(crate::cell_lookup_factory()("tok").is_some());
    }
}
