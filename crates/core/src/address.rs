//! Addresses as people type them and as the tables spell them.

/// What the person asks about.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Address {
    pub comune: String,
    pub via: String,
    pub civico: String,
}

impl Address {
    pub fn new(comune: &str, via: &str, civico: &str) -> Self {
        Self { comune: normalize(comune), via: normalize(via), civico: normalize_civico(civico) }
    }

    pub fn is_empty(&self) -> bool {
        self.comune.is_empty() && self.via.is_empty() && self.civico.is_empty()
    }

    /// `VIA ROMA 10, CAGLIARI` for titles.
    pub fn display(&self) -> String {
        let mut s = self.via.clone();
        if !self.civico.is_empty() {
            if !s.is_empty() {
                s.push(' ');
            }
            s.push_str(&self.civico);
        }
        if !self.comune.is_empty() {
            if !s.is_empty() {
                s.push_str(", ");
            }
            s.push_str(&self.comune);
        }
        s
    }
}

/// Upper case, single spaces, accents folded (the tables are latin1 upper case without accents
/// most of the time: `CAGLIARI`, `VIA ROMA`).
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut space = true;
    for c in s.chars() {
        let c = fold(c);
        if c.is_whitespace() {
            if !space {
                out.push(' ');
                space = true;
            }
        } else {
            for u in c.to_uppercase() {
                out.push(u);
            }
            space = false;
        }
    }
    out.trim_end().to_string()
}

/// Accents off, apostrophes unified.
fn fold(c: char) -> char {
    match c {
        'à' | 'á' | 'â' | 'ä' | 'À' | 'Á' | 'Â' | 'Ä' => 'A',
        'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'E',
        'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'I',
        'ò' | 'ó' | 'ô' | 'ö' | 'Ò' | 'Ó' | 'Ô' | 'Ö' => 'O',
        'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'U',
        '’' | '`' | '´' => '\'',
        c => c,
    }
}

/// `10/a` → `10/A`, `10 a` → `10A`, ` 7 ` → `7`.
pub fn normalize_civico(s: &str) -> String {
    let t = normalize(s);
    let mut out = String::with_capacity(t.len());
    for c in t.chars() {
        if c != ' ' {
            out.push(c);
        }
    }
    out
}

/// The street name without its particle: `VIA ROMA` → (`VIA`, `ROMA`); `ROMA` → (``, `ROMA`).
/// Tables keep the particle (`PART_TOP_CLIENTE`, `particella_top_egon`) apart from the name.
pub fn split_particle(via: &str) -> (String, String) {
    const PARTICLES: [&str; 28] = [
        "VIA",
        "VIALE",
        "PIAZZA",
        "PIAZZALE",
        "PIAZZETTA",
        "CORSO",
        "LARGO",
        "VICOLO",
        "VICO",
        "STRADA",
        "LOCALITA'",
        "LOCALITA",
        "LOC.",
        "LOC",
        "FRAZIONE",
        "FRAZ.",
        "BORGO",
        "LUNGOMARE",
        "SALITA",
        "DISCESA",
        "TRAVERSA",
        "CIRCONVALLAZIONE",
        "RAMPA",
        "REGIONE",
        "CONTRADA",
        "SP",
        "SS",
        "STRADA PROVINCIALE",
    ];
    let v = normalize(via);
    // Two-word particles first.
    for p in PARTICLES.iter().filter(|p| p.contains(' ')) {
        if let Some(rest) = v.strip_prefix(p).filter(|r| r.starts_with(' ')) {
            return ((*p).to_string(), rest.trim().to_string());
        }
    }
    match v.split_once(' ') {
        Some((head, rest)) if PARTICLES.contains(&head) => (head.to_string(), rest.trim().to_string()),
        _ => (String::new(), v),
    }
}

/// `LIKE` pattern for a street search: every word must appear, in order.
pub fn like_pattern(query: &str) -> String {
    let (_, name) = split_particle(query);
    let mut out = String::from("%");
    for w in name.split(' ').filter(|w| !w.is_empty()) {
        out.push_str(&w.replace(['%', '_'], ""));
        out.push('%');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_addresses() {
        assert_eq!(normalize("  via   Roma "), "VIA ROMA");
        assert_eq!(normalize("Sant’Elia è là"), "SANT'ELIA E LA");
        assert_eq!(normalize_civico("10 / a"), "10/A");
        assert_eq!(split_particle("via roma"), ("VIA".into(), "ROMA".into()));
        assert_eq!(split_particle("Roma"), ("".into(), "ROMA".into()));
        assert_eq!(split_particle("strada provinciale 8"), ("STRADA PROVINCIALE".into(), "8".into()));
        assert_eq!(like_pattern("via dei mille%"), "%DEI%MILLE%");
        let a = Address::new("cagliari", "via roma", "10/a");
        assert_eq!(a.display(), "VIA ROMA 10/A, CAGLIARI");
        assert!(Address::default().is_empty());
    }
}
