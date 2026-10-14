//! Coordinates: parsing the tables' strings, distances and bearings, street polylines.

/// A point on Earth (degrees).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Point {
    pub lat: f64,
    pub lon: f64,
}

impl Point {
    pub fn new(lat: f64, lon: f64) -> Option<Self> {
        (lat.is_finite() && lon.is_finite() && (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some(Self { lat, lon })
    }

    /// Great-circle distance in metres.
    pub fn distance_m(self, other: Self) -> f64 {
        let r = 6_371_000.0_f64;
        let (la1, la2) = (self.lat.to_radians(), other.lat.to_radians());
        let dlat = la2 - la1;
        let dlon = (other.lon - self.lon).to_radians();
        let a = (dlat / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlon / 2.0).sin().powi(2);
        2.0 * r * a.sqrt().min(1.0).asin()
    }

    /// Initial bearing from here to `other`, degrees clockwise from north.
    pub fn bearing_deg(self, other: Self) -> f64 {
        let (la1, la2) = (self.lat.to_radians(), other.lat.to_radians());
        let dlon = (other.lon - self.lon).to_radians();
        let y = dlon.sin() * la2.cos();
        let x = la1.cos() * la2.sin() - la1.sin() * la2.cos() * dlon.cos();
        (y.atan2(x).to_degrees() + 360.0) % 360.0
    }
}

/// A number written with a comma or a point.
pub fn parse_number(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    let t = t.trim_matches(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'));
    t.parse().ok().filter(|v: &f64| v.is_finite())
}

/// Two numbers in one cell: `39.2238,9.1217`, `39.2238;9.1217`, `39.2238 9.1217`, `POINT(9.12 39.22)`
/// (lon first in WKT), or `lat:39.2,lon:9.1`.
pub fn parse_point(s: &str) -> Option<Point> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    let upper = t.to_ascii_uppercase();
    if let Some(inner) = upper.strip_prefix("POINT").map(|r| r.trim().trim_matches(['(', ')'])) {
        let mut it = inner.split_whitespace();
        let lon = parse_number(it.next()?)?;
        let lat = parse_number(it.next()?)?;
        return Point::new(lat, lon);
    }
    // Comma is a decimal separator when it is the only one and there is another separator.
    let has_other = t.contains([';', ' ', '|', '/']);
    let normalized = if has_other { t.replace(',', ".") } else { t.to_string() };
    let parts: Vec<&str> = normalized.split([',', ';', ' ', '|', '/']).filter(|p| !p.trim().is_empty()).collect();
    if parts.len() < 2 {
        return None;
    }
    let mut nums = parts.iter().filter_map(|p| parse_number(p.split(':').next_back().unwrap_or(p)));
    let a = nums.next()?;
    let b = nums.next()?;
    // Italy: latitude 35..48, longitude 6..19. Swap when written the other way round.
    if !(35.0..=48.0).contains(&a) && (35.0..=48.0).contains(&b) { Point::new(b, a) } else { Point::new(a, b) }
}

/// A street polyline as the tables store it: Google's encoded format, `lat,lon lat,lon …`,
/// `lat,lon;lat,lon`, or a JSON array of pairs.
pub fn parse_polyline(s: &str) -> Vec<Point> {
    let t = s.trim();
    if t.is_empty() {
        return Vec::new();
    }
    if t.starts_with('[')
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(t)
    {
        {
            let mut out = Vec::new();
            for pair in v.as_array().into_iter().flatten().take(crate::limits::POINTS) {
                let nums: Vec<f64> = match pair {
                    serde_json::Value::Array(a) => a.iter().filter_map(serde_json::Value::as_f64).collect(),
                    serde_json::Value::Object(o) => {
                        ["lat", "lng", "lon"].iter().filter_map(|k| o.get(*k).and_then(serde_json::Value::as_f64)).collect()
                    }
                    _ => Vec::new(),
                };
                if let (Some(a), Some(b)) = (nums.first(), nums.get(1))
                    && let Some(p) = parse_point(&format!("{a} {b}"))
                {
                    out.push(p);
                }
            }
            return out;
        }
    }
    let separated: Vec<Point> = t.split([' ', ';', '|']).filter_map(parse_point).take(crate::limits::POINTS).collect();
    if separated.len() >= 2 || (separated.len() == 1 && !t.chars().any(|c| c.is_ascii_alphabetic())) {
        return separated;
    }
    decode_encoded(t)
}

/// Google's encoded polyline algorithm.
fn decode_encoded(s: &str) -> Vec<Point> {
    let mut out = Vec::new();
    let (mut lat, mut lon) = (0i64, 0i64);
    let bytes = s.as_bytes();
    let mut i = 0;
    let read = |i: &mut usize| -> Option<i64> {
        let (mut result, mut shift) = (0i64, 0u32);
        loop {
            let b = i64::from(*bytes.get(*i)?) - 63;
            *i += 1;
            if !(0..=63).contains(&b) {
                return None;
            }
            result |= (b & 0x1f) << shift;
            shift += 5;
            if b < 0x20 || shift > 60 {
                break;
            }
        }
        Some(if result & 1 == 1 { !(result >> 1) } else { result >> 1 })
    };
    while i < bytes.len() && out.len() < crate::limits::POINTS {
        let Some(dlat) = read(&mut i) else { break };
        let Some(dlon) = read(&mut i) else { break };
        lat += dlat;
        lon += dlon;
        match Point::new(lat as f64 / 1e5, lon as f64 / 1e5) {
            Some(p) => out.push(p),
            None => break,
        }
    }
    out
}

/// The middle point of a polyline (by vertex count: enough to place a street).
pub fn midpoint(points: &[Point]) -> Option<Point> {
    points.get(points.len() / 2).copied().or_else(|| points.first().copied())
}

/// The vertex nearest to `p`.
pub fn nearest(points: &[Point], p: Point) -> Option<Point> {
    points.iter().copied().min_by(|a, b| a.distance_m(p).total_cmp(&b.distance_m(p)))
}

/// Compass name of a bearing: N, NE, E…
pub fn compass(bearing: f64) -> &'static str {
    const NAMES: [&str; 8] = ["N", "NE", "E", "SE", "S", "SO", "O", "NO"];
    let i = (((bearing % 360.0 + 360.0) % 360.0 + 22.5) / 45.0) as usize % 8;
    NAMES[i]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_points_in_every_spelling() {
        let p = parse_point("39.2238,9.1217").unwrap();
        assert!((p.lat - 39.2238).abs() < 1e-9 && (p.lon - 9.1217).abs() < 1e-9);
        assert_eq!(parse_point("39,2238;9,1217"), Some(Point { lat: 39.2238, lon: 9.1217 }));
        assert_eq!(parse_point("9.1217 39.2238"), Some(Point { lat: 39.2238, lon: 9.1217 }));
        assert_eq!(parse_point("POINT(9.1217 39.2238)"), Some(Point { lat: 39.2238, lon: 9.1217 }));
        assert_eq!(parse_point("lat:39.2,lon:9.1"), Some(Point { lat: 39.2, lon: 9.1 }));
        assert_eq!(parse_point(""), None);
        assert_eq!(parse_point("abc"), None);
        assert_eq!(parse_point("999,999"), None);
    }

    #[test]
    fn distances_and_bearings() {
        let cagliari = Point { lat: 39.2238, lon: 9.1217 };
        let sassari = Point { lat: 40.7259, lon: 8.5557 };
        let d = cagliari.distance_m(sassari);
        assert!((d - 174_000.0).abs() < 3_000.0, "{d}");
        let b = cagliari.bearing_deg(sassari);
        assert!((330.0..350.0).contains(&b), "{b}");
        assert!(["N", "NO"].contains(&compass(b)), "{b}");
        assert_eq!(compass(0.0), "N");
        assert_eq!(compass(90.0), "E");
        assert_eq!(compass(-10.0), "N");
    }

    #[test]
    fn polylines_in_three_formats() {
        let plain = parse_polyline("39.22,9.12 39.23,9.13 39.24,9.14");
        assert_eq!(plain.len(), 3);
        assert_eq!(midpoint(&plain), Some(Point { lat: 39.23, lon: 9.13 }));
        let json = parse_polyline("[[39.22,9.12],[39.23,9.13]]");
        assert_eq!(json.len(), 2);
        // Google's example: (38.5, -120.2), (40.7, -120.95), (43.252, -126.453).
        let enc = parse_polyline("_p~iF~ps|U_ulLnnqC_mqNvxq`@");
        assert_eq!(enc.len(), 3);
        assert!((enc[2].lat - 43.252).abs() < 1e-6);
        assert!(nearest(&enc, Point { lat: 40.0, lon: -121.0 }).is_some());
        assert!(parse_polyline("").is_empty());
        assert!(parse_polyline("garbage!!").len() <= 1);
    }
}
