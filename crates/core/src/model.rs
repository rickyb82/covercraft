//! Rows as the tables give them and the report the person reads.

use std::collections::BTreeMap;

use crate::geo::Point;
use crate::status::Status;

/// A municipality and the TIM table that holds its civici (`comuni.table_name`).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Comune {
    pub name: String,
    pub provincia: String,
    /// `netmap_<hash>`; empty when the comune has no TIM table (only other providers).
    pub table: String,
    pub istat: String,
}

/// A street of a comune.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Street {
    pub egon: String,
    pub particle: String,
    pub name: String,
}

impl Street {
    pub fn display(&self) -> String {
        if self.particle.is_empty() { self.name.clone() } else { format!("{} {}", self.particle, self.name) }
    }
}

/// A civico with its EGON code: what every coverage table is keyed by.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Civico {
    pub egon: String,
    pub egon_strada: String,
    pub particle: String,
    pub street: String,
    pub civico: String,
    pub comune: String,
    pub provincia: String,
    /// Which table knew it: `tim`, `openfiber`, `fastweb`.
    pub source: String,
}

impl Civico {
    pub fn display(&self) -> String {
        let street = if self.particle.is_empty() { self.street.clone() } else { format!("{} {}", self.particle, self.street) };
        format!("{street} {}, {}", self.civico, self.comune).trim().to_string()
    }
}

/// One row of the comune's TIM table: every column by name (they are all text).
pub type TimRow = BTreeMap<String, String>;

/// One scala/palazzina of Open Fiber's table.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenFiberRow {
    pub id_scala: String,
    pub scala: String,
    pub building: String,
    pub coordinates: String,
    pub pop: String,
    pub units: String,
    pub building_state: String,
    pub scala_state: String,
    pub rfc_planned: String,
    pub rfc_actual: String,
    pub rfa_planned: String,
    pub rfa_actual: String,
    pub cluster: String,
    pub updated: String,
}

/// `openfiber_status`: a state code's meaning in a cluster.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenFiberStatus {
    pub value: i64,
    pub cluster: String,
    pub description: String,
    pub friendly: String,
}

/// Fastweb's tables for one civico (each row is columns by name, like TIM's).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FastwebRows {
    pub ftth: Vec<TimRow>,
    pub ftts: Vec<TimRow>,
    pub vula: Vec<TimRow>,
    pub gpon: Vec<TimRow>,
}

/// The operator's own network at a civico (`sirius_netmap` with the names joined in).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OwnService {
    pub service: String,
    pub state: String,
    pub since: String,
    pub note: String,
}

/// A radio bridge (`radio_table`).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Bridge {
    pub id: i64,
    pub name: String,
    pub lat: String,
    pub lon: String,
    pub alt: String,
    pub state: String,
    pub description: String,
    /// `fascia`: A, B, C, D (A the widest reach).
    pub band: String,
}

impl Bridge {
    pub fn point(&self) -> Option<Point> {
        Point::new(crate::geo::parse_number(&self.lat)?, crate::geo::parse_number(&self.lon)?)
    }
}

/// A mobile cell (`cell_db`, OpenCelliD's columns).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Cell {
    pub radio: String,
    pub mcc: String,
    pub net: String,
    pub area: String,
    pub cell: String,
    pub lat: String,
    pub lon: String,
    pub range: String,
    pub samples: String,
}

impl Cell {
    pub fn point(&self) -> Option<Point> {
        Point::new(crate::geo::parse_number(&self.lat)?, crate::geo::parse_number(&self.lon)?)
    }
}

/// A past verification (`requests_log`).
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HistoryEntry {
    pub id: i64,
    pub date: String,
    pub user: String,
    pub request: serde_json::Value,
    pub answer: serde_json::Value,
}

/// Who sells the service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Tim,
    OpenFiber,
    Fastweb,
    Own,
}

impl Provider {
    pub const ALL: [Self; 4] = [Self::Tim, Self::OpenFiber, Self::Fastweb, Self::Own];

    pub fn label(self) -> &'static str {
        match self {
            Self::Tim => "TIM / FiberCop",
            Self::OpenFiber => "Open Fiber",
            Self::Fastweb => "Fastweb",
            Self::Own => "Rete propria",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Self::Tim => "tim",
            Self::OpenFiber => "openfiber",
            Self::Fastweb => "fastweb",
            Self::Own => "own",
        }
    }
}

/// One service at the address, interpreted.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Service {
    pub provider: Option<Provider>,
    pub technology: String,
    pub status: Status,
    /// The cell as the table has it.
    pub raw: String,
    /// Planned date or expansion note.
    pub planned: String,
    pub speed_down: String,
    pub speed_up: String,
    pub distance_m: Option<u32>,
    /// Other columns worth showing: (label, value).
    pub details: Vec<(String, String)>,
}

/// A radio bridge seen from the address.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RadioHit {
    pub bridge: Bridge,
    pub distance_m: f64,
    pub bearing_deg: f64,
    pub compass: String,
    /// Within the band's reach.
    pub in_reach: bool,
    pub reach_m: f64,
}

/// A mobile cell seen from the address.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CellHit {
    pub cell: Cell,
    pub operator: String,
    pub distance_m: f64,
    pub in_range: bool,
}

/// The whole answer for one civico.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Report {
    pub civico: Civico,
    pub point: Option<Point>,
    /// Where the point came from: `openfiber`, `polyline`, `none`.
    pub point_source: String,
    pub services: Vec<Service>,
    /// General facts of the civico from TIM's row (category, units, area, switch-off…).
    pub info: Vec<(String, String)>,
    pub openfiber: Vec<OpenFiberScala>,
    pub radio: Vec<RadioHit>,
    pub cells: Vec<CellHit>,
    pub notes: Vec<String>,
    pub time: u64,
}

/// An Open Fiber scala with its states spelled out.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OpenFiberScala {
    pub row: OpenFiberRow,
    pub building_status: Status,
    pub building_text: String,
    pub scala_status: Status,
    pub scala_text: String,
    pub xgspon: bool,
}

impl Report {
    /// The best service at the address, covered first, then by speed.
    pub fn best(&self) -> Option<&Service> {
        self.services.iter().max_by(|a, b| {
            let ka = (a.status, crate::status::speed_mbps(&a.speed_down).unwrap_or(0.0));
            let kb = (b.status, crate::status::speed_mbps(&b.speed_down).unwrap_or(0.0));
            ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1))
        })
    }

    /// Services of one provider.
    pub fn of(&self, p: Provider) -> impl Iterator<Item = &Service> {
        self.services.iter().filter(move |s| s.provider == Some(p))
    }

    /// Nearest bridge in reach, if any.
    pub fn best_radio(&self) -> Option<&RadioHit> {
        self.radio.iter().filter(|r| r.in_reach).min_by(|a, b| a.distance_m.total_cmp(&b.distance_m))
    }
}

/// Mobile operator names by MNC (Italy, MCC 222).
pub fn operator_name(mcc: &str, mnc: &str) -> String {
    if mcc.trim() != "222" {
        return format!("MCC {} MNC {}", mcc.trim(), mnc.trim());
    }
    match mnc.trim().trim_start_matches('0') {
        "1" | "48" => "TIM".into(),
        "10" => "Vodafone".into(),
        "88" => "WindTre".into(),
        "99" => "WindTre (3)".into(),
        "50" => "Iliad".into(),
        "33" | "34" | "35" | "36" | "37" | "38" | "39" => "Fastweb".into(),
        "7" => "Noverca".into(),
        "53" => "Spusu".into(),
        other => format!("MNC {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_the_best_service() {
        let mut r = Report::default();
        r.services.push(Service { technology: "ADSL".into(), status: Status::Covered, speed_down: "20M".into(), ..Default::default() });
        r.services.push(Service { technology: "FTTH".into(), status: Status::Planned, speed_down: "1000".into(), ..Default::default() });
        r.services.push(Service { technology: "FTTC".into(), status: Status::Covered, speed_down: "200".into(), ..Default::default() });
        assert_eq!(r.best().map(|s| s.technology.as_str()), Some("FTTC"));
        assert_eq!(operator_name("222", "010"), "Vodafone");
        assert_eq!(operator_name("222", "88"), "WindTre");
        assert_eq!(operator_name("262", "01"), "MCC 262 MNC 01");
        let c = Civico { particle: "VIA".into(), street: "ROMA".into(), civico: "10".into(), comune: "CAGLIARI".into(), ..Default::default() };
        assert_eq!(c.display(), "VIA ROMA 10, CAGLIARI");
        assert_eq!(Street { particle: "".into(), name: "X".into(), ..Default::default() }.display(), "X");
    }
}
