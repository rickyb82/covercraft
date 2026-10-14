//! What a coverage cell means. The providers' files write availability in many ways
//! (`SI`, `COPERTO`, `VENDIBILE`, `NON VENDIBILE`, `PIANIFICATO`, dates…): the interpretation
//! is a set of rules the configuration can extend, and the raw value is always kept next to it.

/// A traffic light.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Nothing known (empty cell, unknown wording).
    #[default]
    Unknown,
    NotCovered,
    /// Planned, in progress, a date in the future.
    Planned,
    Covered,
}

impl Status {
    pub fn label(self, italian: bool) -> &'static str {
        match (self, italian) {
            (Self::Covered, true) => "Coperto",
            (Self::Covered, false) => "Covered",
            (Self::Planned, true) => "Pianificato",
            (Self::Planned, false) => "Planned",
            (Self::NotCovered, true) => "Non coperto",
            (Self::NotCovered, false) => "Not covered",
            (Self::Unknown, true) => "Non noto",
            (Self::Unknown, false) => "Unknown",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Covered => "covered",
            Self::Planned => "planned",
            Self::NotCovered => "not_covered",
            Self::Unknown => "unknown",
        }
    }
}

/// A rule from the configuration: a value (upper case, exact) or a prefix (`PREFIX*`) → status.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Rule {
    pub value: String,
    pub status: Status,
}

/// Reads a raw cell with the configured rules first, then the built-in wording.
pub fn interpret(raw: &str, rules: &[Rule]) -> Status {
    let v = crate::address::normalize(raw);
    for r in rules {
        let pat = crate::address::normalize(&r.value);
        let hit = match pat.strip_suffix('*') {
            Some(prefix) => v.starts_with(prefix),
            None => v == pat,
        };
        if hit {
            return r.status;
        }
    }
    builtin(&v)
}

/// The built-in wording, on a normalized (upper case) value.
fn builtin(v: &str) -> Status {
    if v.is_empty() || matches!(v, "NULL" | "-" | "N/A" | "NA" | "ND" | "N.D." | "NON DISPONIBILE" | "NON NOTO") {
        return Status::Unknown;
    }
    let negative = [
        "NO",
        "N",
        "0",
        "FALSE",
        "NON COPERT",
        "NON VENDIB",
        "NON ATTIV",
        "NON PRESENTE",
        "NON RAGGIUNG",
        "ASSENTE",
        "NESSUN",
        "KO",
        "CHIUS",
        "DISMESS",
        "SATUR",
        "ESAURIT",
    ];
    if negative.iter().any(|n| v == *n || v.starts_with(&format!("{n} ")) || (n.len() > 2 && v.starts_with(n))) {
        return Status::NotCovered;
    }
    let planned = ["PIANIF", "PREVIST", "PROGRAMM", "IN CORSO", "IN LAVOR", "LAVORI", "IN REALIZ", "IN COSTRUZ", "FUTUR", "PROSSIM", "IN ATTIVAZ"];
    if planned.iter().any(|p| v.contains(p)) || looks_like_date(v) {
        return Status::Planned;
    }
    let positive = [
        "SI",
        "S",
        "Y",
        "YES",
        "1",
        "TRUE",
        "OK",
        "COPERT",
        "VENDIB",
        "ATTIV",
        "DISPON",
        "PRESENTE",
        "COMPLET",
        "REALIZZ",
        "ESEGUIT",
        "APERT",
        "RFC",
        "RFA",
        "ON AIR",
        "ONLINE",
        "IN ESERCIZIO",
        "FTTH",
        "FTTC",
        "GPON",
        "XGS",
    ];
    if positive.iter().any(|p| v == *p || v.starts_with(p) || v.contains(&format!(" {p}"))) {
        return Status::Covered;
    }
    Status::Unknown
}

/// `31/12/2026`, `2026-12-31`, `12/2026`, `Q3 2026`.
pub fn looks_like_date(v: &str) -> bool {
    let digits = v.chars().filter(char::is_ascii_digit).count();
    let seps = v.chars().filter(|c| matches!(c, '/' | '-' | '.')).count();
    (digits >= 6 && seps >= 1 && digits + seps + 2 >= v.len()) || (v.starts_with('Q') && digits == 5)
}

/// `100M`, `100 Mbps`, `1000`, `1G` → Mbps.
pub fn speed_mbps(raw: &str) -> Option<f64> {
    let t = raw.trim().to_ascii_uppercase();
    let num: String = t.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
    let n: f64 = num.replace(',', ".").parse().ok()?;
    let unit = t.get(num.len()..).unwrap_or_default().trim();
    Some(match unit.chars().next() {
        Some('G') => n * 1000.0,
        Some('K') => n / 1000.0,
        _ => n,
    })
}

/// `350`, `350 m`, `0,35 km` → metres.
pub fn distance_m(raw: &str) -> Option<u32> {
    let t = raw.trim().to_ascii_lowercase();
    let num: String = t.chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == ',').collect();
    let n: f64 = num.replace(',', ".").parse().ok()?;
    let unit = t.get(num.len()..).unwrap_or_default().trim();
    let m = if unit.starts_with("km") { n * 1000.0 } else { n };
    (m.is_finite() && (0.0..1e7).contains(&m)).then_some(m.round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_providers_wording() {
        for (v, s) in [
            ("SI", Status::Covered),
            ("si", Status::Covered),
            ("COPERTO", Status::Covered),
            ("Vendibile", Status::Covered),
            ("ATTIVA", Status::Covered),
            ("NO", Status::NotCovered),
            ("NON VENDIBILE", Status::NotCovered),
            ("Non coperto", Status::NotCovered),
            ("ASSENTE", Status::NotCovered),
            ("PIANIFICATO", Status::Planned),
            ("Prevista", Status::Planned),
            ("31/12/2026", Status::Planned),
            ("Q3 2026", Status::Planned),
            ("", Status::Unknown),
            ("N/A", Status::Unknown),
            ("boh", Status::Unknown),
        ] {
            assert_eq!(interpret(v, &[]), s, "{v}");
        }
        let rules = [Rule { value: "boh".into(), status: Status::Covered }, Rule { value: "X*".into(), status: Status::Planned }];
        assert_eq!(interpret("BOH", &rules), Status::Covered);
        assert_eq!(interpret("xyz", &rules), Status::Planned);
        assert_eq!(interpret("SI", &rules), Status::Covered);
    }

    #[test]
    fn speeds_and_distances() {
        assert_eq!(speed_mbps("100M"), Some(100.0));
        assert_eq!(speed_mbps("1G"), Some(1000.0));
        assert_eq!(speed_mbps("2,5 Gbps"), Some(2500.0));
        assert_eq!(speed_mbps("640k"), Some(0.64));
        assert_eq!(speed_mbps("n/a"), None);
        assert_eq!(distance_m("350"), Some(350));
        assert_eq!(distance_m("0,35 km"), Some(350));
        assert_eq!(distance_m("boh"), None);
        assert_eq!(Status::Covered.label(true), "Coperto");
        assert!(Status::Covered > Status::Planned && Status::Planned > Status::NotCovered);
    }
}
