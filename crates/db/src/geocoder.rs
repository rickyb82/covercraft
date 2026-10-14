//! A Nominatim client: `GET {url}/search?…` with a structured query, then a free one.

use covercraft_core::geo::Point;
use covercraft_core::geocode::{GeocodeQuery, Geocoder, parse_answer};
use covercraft_core::{Error, Result};

pub struct Nominatim {
    base: String,
}

impl Nominatim {
    pub fn new(base: &str) -> Self {
        Self { base: base.trim_end_matches('/').to_string() }
    }

    fn get(&self, query: &str) -> Result<Option<Point>> {
        let url = format!("{}/search?{query}", self.base);
        let mut req = ehttp::Request::get(url);
        req.headers = ehttp::Headers::new(&[("Accept", "application/json"), ("User-Agent", concat!("covercraft/", env!("CARGO_PKG_VERSION")))]);
        let r = ehttp::fetch_blocking(&req).map_err(|e| Error::Database(format!("geocoder: {e}")))?;
        if !r.ok {
            return Err(Error::Database(format!("geocoder: HTTP {} {}", r.status, r.status_text)));
        }
        let text = String::from_utf8_lossy(&r.bytes);
        Ok(parse_answer(&text))
    }
}

impl Geocoder for Nominatim {
    fn geocode(&mut self, q: &GeocodeQuery) -> Result<Option<Point>> {
        match self.get(&q.structured_query())? {
            Some(p) => Ok(Some(p)),
            None => self.get(&q.free_query()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A tiny server answering the structured query with nothing and the free one with a point.
    fn serve(listener: TcpListener) {
        for stream in listener.incoming().take(2) {
            let Ok(mut s) = stream else { continue };
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).unwrap_or(0);
            let req = String::from_utf8_lossy(&buf[..n]).to_string();
            let body = if req.contains("street=") { "[]" } else { r#"[{"lat":"39.2155","lon":"9.1115"}]"# };
            let _ = s.write_all(
                format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())
                    .as_bytes(),
            );
        }
    }

    #[test]
    fn asks_structured_then_free() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        std::thread::spawn(move || serve(listener));
        let mut g = Nominatim::new(&format!("http://{addr}/"));
        let c = covercraft_core::model::Civico {
            street: "ROMA".into(),
            particle: "VIA".into(),
            civico: "10".into(),
            comune: "CAGLIARI".into(),
            provincia: "CA".into(),
            ..Default::default()
        };
        let p = g.geocode(&GeocodeQuery::from_civico(&c)).unwrap().unwrap();
        assert!((p.lat - 39.2155).abs() < 1e-9);
        let mut dead = Nominatim::new("http://127.0.0.1:1");
        assert!(dead.geocode(&GeocodeQuery::from_civico(&c)).is_err());
        assert!(crate::geocoder_factory()("").is_none());
        assert!(crate::geocoder_factory()("http://x").is_some());
    }
}
