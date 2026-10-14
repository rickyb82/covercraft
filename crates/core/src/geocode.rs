//! Geocoding through a Nominatim server (OpenStreetMap): the request and the answer's
//! reading are here; the HTTP call is injected (the `db` crate makes it natively).

use crate::address;
use crate::geo::Point;
use crate::model::Civico;

/// Something that turns an address into a point (a Nominatim client, or a test double).
pub trait Geocoder: Send {
    /// The best point for the address, if any.
    fn geocode(&mut self, query: &GeocodeQuery) -> crate::Result<Option<Point>>;
}

/// Makes a geocoder for a server URL (`None` when the URL is empty); apps inject the real one.
pub type GeocoderFactory = std::sync::Arc<dyn Fn(&str) -> Option<Box<dyn Geocoder>> + Send + Sync>;

/// What is asked of the geocoder: a structured query first, a free one as the fallback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeocodeQuery {
    pub street: String,
    pub comune: String,
    pub provincia: String,
    /// The one-line form, for servers that only take `q=`.
    pub free_text: String,
}

impl GeocodeQuery {
    pub fn from_civico(c: &Civico) -> Self {
        let via = if c.particle.is_empty() { c.street.clone() } else { format!("{} {}", c.particle, c.street) };
        let street = if c.civico.is_empty() { via.clone() } else { format!("{via} {}", c.civico) };
        Self {
            street: title(&street),
            comune: title(&c.comune),
            provincia: c.provincia.clone(),
            free_text: format!("{}, {}, {}, Italia", title(&street), title(&c.comune), c.provincia).replace(", ,", ","),
        }
    }

    /// The query string of Nominatim's `/search` (structured), JSON output, one result.
    pub fn structured_query(&self) -> String {
        format!(
            "format=jsonv2&limit=1&countrycodes=it&street={}&city={}{}",
            url_encode(&self.street),
            url_encode(&self.comune),
            if self.provincia.is_empty() { String::new() } else { format!("&county={}", url_encode(&self.provincia)) }
        )
    }

    /// The free-text form (`q=`), used when the structured one finds nothing.
    pub fn free_query(&self) -> String {
        format!("format=jsonv2&limit=1&countrycodes=it&q={}", url_encode(&self.free_text))
    }
}

/// `VIA ROMA` → `Via Roma` (Nominatim matches better on normal case).
pub fn title(s: &str) -> String {
    address::normalize(s)
        .split(' ')
        .map(|w| {
            let mut c = w.chars();
            match c.next() {
                Some(f) => f.to_string() + &c.as_str().to_lowercase(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn url_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// The first result of a Nominatim JSON answer.
pub fn parse_answer(json: &str) -> Option<Point> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    let first = v.as_array()?.first()?;
    let num = |k: &str| first.get(k).and_then(|x| x.as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| x.as_f64()));
    Point::new(num("lat")?, num("lon")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_queries_and_reads_answers() {
        let c = Civico {
            particle: "VIA".into(),
            street: "ROMA".into(),
            civico: "10/A".into(),
            comune: "CAGLIARI".into(),
            provincia: "CA".into(),
            ..Default::default()
        };
        let q = GeocodeQuery::from_civico(&c);
        assert_eq!(q.street, "Via Roma 10/a");
        assert_eq!(q.structured_query(), "format=jsonv2&limit=1&countrycodes=it&street=Via+Roma+10%2Fa&city=Cagliari&county=CA");
        assert!(q.free_query().contains("q=Via+Roma+10%2Fa%2C+Cagliari%2C+CA%2C+Italia"));
        let p = parse_answer(r#"[{"lat":"39.2155","lon":"9.1115","display_name":"x"}]"#).unwrap();
        assert!((p.lat - 39.2155).abs() < 1e-9);
        assert!(parse_answer("[]").is_none());
        assert!(parse_answer("not json").is_none());
        assert_eq!(title("sant'elia"), "Sant'elia");
    }
}
