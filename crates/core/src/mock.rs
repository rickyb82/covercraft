//! An in-memory source with a small made-up city: tests, the demo, the web snapshot.

use serde_json::Value;

use crate::model::*;
use crate::source::CoverageSource;
use crate::{Result, address, limits};

/// The demo dataset: a few streets of CAGLIARI and the first rows of other comuni.
pub struct MockSource {
    pub comuni: Vec<Comune>,
    pub civici: Vec<Civico>,
    pub tim: Vec<(String, TimRow)>,
    pub openfiber: Vec<(String, OpenFiberRow)>,
    pub statuses: Vec<OpenFiberStatus>,
    pub xgspon: Vec<String>,
    pub fastweb: Vec<(String, &'static str, TimRow)>,
    pub own: Vec<(String, OwnService)>,
    pub polylines: Vec<(String, String)>,
    pub bridges: Vec<Bridge>,
    pub cells: Vec<Cell>,
    pub log: Vec<HistoryEntry>,
    /// When set, every call fails with this message (tests of error paths).
    pub failing: Option<String>,
}

fn row(pairs: &[(&str, &str)]) -> TimRow {
    pairs.iter().map(|(k, v)| ((*k).to_string(), (*v).to_string())).collect()
}

fn civico(egon: &str, strada: &str, particle: &str, street: &str, n: &str) -> Civico {
    Civico {
        egon: egon.into(),
        egon_strada: strada.into(),
        particle: particle.into(),
        street: street.into(),
        civico: n.into(),
        comune: "CAGLIARI".into(),
        provincia: "CA".into(),
        source: "tim".into(),
    }
}

impl Default for MockSource {
    fn default() -> Self {
        Self::demo()
    }
}

impl MockSource {
    pub fn demo() -> Self {
        let comuni = vec![
            Comune {
                name: "CAGLIARI".into(),
                provincia: "CA".into(),
                table: "netmap_demo_cagliari".into(),
                istat: "092009".into(),
                regione: "SARDEGNA".into(),
                provincia_name: "CAGLIARI".into(),
            },
            Comune {
                name: "QUARTU SANT'ELENA".into(),
                provincia: "CA".into(),
                table: "netmap_demo_quartu".into(),
                istat: "092051".into(),
                regione: "SARDEGNA".into(),
                provincia_name: "CAGLIARI".into(),
            },
            Comune {
                name: "SASSARI".into(),
                provincia: "SS".into(),
                table: "netmap_demo_sassari".into(),
                istat: "090064".into(),
                regione: "SARDEGNA".into(),
                provincia_name: "SASSARI".into(),
            },
            Comune {
                name: "NUORO".into(),
                provincia: "NU".into(),
                table: String::new(),
                istat: "091051".into(),
                regione: "SARDEGNA".into(),
                provincia_name: "NUORO".into(),
            },
            Comune {
                name: "ROMA".into(),
                provincia: "RM".into(),
                table: String::new(),
                istat: "058091".into(),
                regione: "LAZIO".into(),
                provincia_name: "ROMA".into(),
            },
        ];
        let civici = vec![
            civico("100010", "10001", "VIA", "ROMA", "10"),
            civico("100012", "10001", "VIA", "ROMA", "12"),
            civico("100014", "10001", "VIA", "ROMA", "14"),
            civico("100020", "10001", "VIA", "ROMA", "20/A"),
            civico("100105", "10002", "VIA", "DANTE", "5"),
            civico("100107", "10002", "VIA", "DANTE", "7"),
            civico("100201", "10003", "PIAZZA", "YENNE", "1"),
            civico("100300", "10004", "VIALE", "MARCONI", "100"),
        ];
        let base = [
            ("DR", "SARDEGNA"),
            ("CATEGORIA", "R"),
            ("COPERTURA_PER_PROSSIMITA", "NO"),
            ("TOTALE_UI", "8"),
            ("AREA", "NERA"),
            ("COPERTURA_BITSTREAM_ASIM_ETHERNET", "SI"),
            ("VELOCITA_DOWN_6dB_BITSTREAM_ASIM_ETHERNET", "20M"),
            ("VELOCITA_UP_6dB_BITSTREAM_ASIM_ETHERNET", "1M"),
            ("DISTANZA_BITSTREAM_ASIM_ETHERNET", "1800"),
            ("EDR_BITSTREAM_ASIM_ETHERNET", "CAGLIARI CENTRO"),
            ("COPERTURA_ULL", "SI"),
            ("VELOCITA_DOWN_ULL_ADSL2+", "20M"),
            ("COPERTURA_WLR", "SI"),
        ];
        let mut tim = Vec::new();
        let mut with = |egon: &str, extra: &[(&str, &str)]| {
            let mut r = row(&base);
            r.extend(row(extra));
            tim.push((egon.to_string(), r));
        };
        with(
            "100010",
            &[
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTH", "SI"),
                ("TIPO_COPERTURA_FTTH", "FIBERCOP"),
                ("EDR_BITSTREAM_NGA_VULA_FTTH", "CA-ROMA-01"),
                ("COPERTURA_BITSTREAM_NGA_VULA_XGSPON", "SI"),
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTCab", "SI"),
                ("VELOCITA_DOWN_BITSTREAM_NGA_VULA_FTTCab", "200M"),
                ("VELOCITA_UP_BITSTREAM_NGA_VULA_FTTCab", "20M"),
                ("DISTANZA_BITSTREAM_NGA_VULA_FTTCab", "320"),
                ("EDR_BITSTREAM_NGA_VULA_FTTCab", "ARL 12"),
                ("SWITCH_OFF", "SI"),
            ],
        );
        with(
            "100012",
            &[
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTH", "NO"),
                ("PIANIFICAZIONE_BITSTREAM_NGA_VULA_FTTH", "30/06/2027"),
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTCab", "SI"),
                ("VELOCITA_DOWN_BITSTREAM_NGA_VULA_FTTCab", "100M"),
                ("VELOCITA_UP_BITSTREAM_NGA_VULA_FTTCab", "20M"),
                ("DISTANZA_BITSTREAM_NGA_VULA_FTTCab", "610"),
                ("COPERTURA_EVDSL_BITSTREAM_NGA_VULA_FTTCab", "SI"),
                ("VELOCITA_DOWN_EVDSL_BITSTREAM_NGA_VULA_FTTCab", "200M"),
                ("VELOCITA_UP_EVDSL_BITSTREAM_NGA_VULA_FTTCab", "30M"),
            ],
        );
        with(
            "100014",
            &[
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTH", "NO"),
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTCab", "NO"),
                ("PIANIFICAZIONE_BITSTREAM_NGA_VULA_FTTCab", "PIANIFICATO"),
            ],
        );
        with(
            "100020",
            &[
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTH", "NO"),
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTCab", "NO"),
                ("COPERTURA_CIRCUITI_TERM_ETH_FIBRA_OTTICA", "SI"),
                ("FASCIA_CIRCUITI_TERM_ETH_FIBRA_OTTICA", "B"),
            ],
        );
        with(
            "100105",
            &[
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTH", "SI"),
                ("TIPO_COPERTURA_FTTH", "FIBERCOP"),
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTCab", "SI"),
                ("VELOCITA_DOWN_BITSTREAM_NGA_VULA_FTTCab", "100M"),
                ("VELOCITA_UP_BITSTREAM_NGA_VULA_FTTCab", "20M"),
                ("DISTANZA_BITSTREAM_NGA_VULA_FTTCab", "450"),
            ],
        );
        with(
            "100107",
            &[
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTH", "SI"),
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTCab", "SI"),
                ("VELOCITA_DOWN_BITSTREAM_NGA_VULA_FTTCab", "50M"),
                ("DISTANZA_BITSTREAM_NGA_VULA_FTTCab", "900"),
            ],
        );
        with(
            "100201",
            &[
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTH", "SI"),
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTCab", "SI"),
                ("VELOCITA_DOWN_BITSTREAM_NGA_VULA_FTTCab", "200M"),
                ("DISTANZA_BITSTREAM_NGA_VULA_FTTCab", "150"),
                ("BUSINESS", "SI"),
            ],
        );
        with(
            "100300",
            &[
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTH", "NO"),
                ("COPERTURA_BITSTREAM_NGA_VULA_FTTCab", "NO"),
                ("COPERTURA_BITSTREAM_ASIM_ETHERNET", "NO"),
                ("COPERTURA_ULL", "SI"),
                ("DISTANZA_ULL", "4200"),
            ],
        );

        let of = |id: &str, scala: &str, coords: &str, pop: &str, b: &str, s: &str, cluster: &str, rfc_p: &str, rfa_a: &str| OpenFiberRow {
            id_scala: id.into(),
            scala: scala.into(),
            building: format!("B{id}"),
            coordinates: coords.into(),
            pop: pop.into(),
            units: "6".into(),
            building_state: b.into(),
            scala_state: s.into(),
            rfc_planned: rfc_p.into(),
            rfa_actual: rfa_a.into(),
            cluster: cluster.into(),
            updated: "01/09/2026".into(),
            ..Default::default()
        };
        let openfiber = vec![
            ("100010".to_string(), of("CA0001-A", "A", "39.2155,9.1115", "CAG01", "1", "1", "C", "", "15/03/2025")),
            ("100010".to_string(), of("CA0001-B", "B", "39.2155,9.1115", "CAG01", "1", "0", "C", "31/12/2026", "")),
            ("100105".to_string(), of("CA0002-A", "A", "39.2172,9.1138", "CAG02", "1", "1", "A", "", "02/02/2024")),
            ("100014".to_string(), of("CA0003-A", "A", "39.2158,9.1121", "CAG01", "0", "0", "C", "30/09/2027", "")),
        ];
        let statuses = vec![
            OpenFiberStatus { value: 0, cluster: "C".into(), description: "In pianificazione".into(), friendly: "Pianificato".into() },
            OpenFiberStatus { value: 1, cluster: "C".into(), description: "RFA".into(), friendly: "Vendibile".into() },
            OpenFiberStatus { value: 0, cluster: "A".into(), description: "In pianificazione".into(), friendly: "Pianificato".into() },
            OpenFiberStatus { value: 1, cluster: "A".into(), description: "RFA".into(), friendly: "Vendibile".into() },
        ];
        let fastweb = vec![
            ("100010".to_string(), "ftth", row(&[("stato_vendibilita_gpon", "VENDIBILE"), ("stato_tecnico_gpon", "ATTIVO"), ("flag_opa", "N")])),
            (
                "100012".to_string(),
                "ftts",
                row(&[
                    ("stato_vendibilita_ftts", "VENDIBILE"),
                    ("velocita_down_ftts", "200"),
                    ("velocita_up_ftts", "20"),
                    ("distanza_civico_cabinet", "580"),
                    ("id_cabinet", "ARL 12"),
                    ("nome_sgu_ftts", "CAGLIARI CENTRO"),
                ]),
            ),
            (
                "100105".to_string(),
                "vula",
                row(&[
                    ("stato_vendibilita_vula", "VENDIBILE"),
                    ("velocita_down_vula", "100"),
                    ("velocita_up_vula", "20"),
                    ("distanza_cabinet_vula", "430"),
                    ("nome_centrale", "CAGLIARI CENTRO"),
                ]),
            ),
            (
                "100300".to_string(),
                "gpon",
                row(&[("stato_copertura_indirizzo", "PIANIFICATO"), ("data_pianificazione_prevista", "31/03/2027"), ("2_macroaree", "SUD")]),
            ),
        ];
        let own = vec![
            (
                "100014".to_string(),
                OwnService { service: "Radio 5 GHz".into(), state: "Attivo".into(), since: "2025-04-10".into(), note: "cliente dal 2025".into() },
            ),
            (
                "100300".to_string(),
                OwnService {
                    service: "Fibra dedicata".into(),
                    state: "Fattibile".into(),
                    since: "2026-01-20".into(),
                    note: "studio di fattibilità".into(),
                },
            ),
        ];
        let polylines = vec![
            ("10001".to_string(), "39.2150,9.1100 39.2160,9.1125 39.2170,9.1150".to_string()),
            ("10002".to_string(), "39.2165,9.1130 39.2180,9.1145".to_string()),
            ("10003".to_string(), "39.2178,9.1110".to_string()),
            ("10004".to_string(), "39.2200,9.1250 39.2240,9.1320 39.2280,9.1400".to_string()),
        ];
        let bridge = |id: i64, name: &str, lat: &str, lon: &str, alt: &str, band: &str, state: &str| Bridge {
            id,
            name: name.into(),
            lat: lat.into(),
            lon: lon.into(),
            alt: alt.into(),
            state: state.into(),
            description: format!("{name} ({band})"),
            band: band.into(),
        };
        let bridges = vec![
            bridge(1, "Monte Urpinu", "39.2305", "9.1290", "98", "A", "attivo"),
            bridge(2, "Sant'Elia", "39.2010", "9.1540", "40", "C", "attivo"),
            bridge(3, "Monte Arci", "39.7800", "8.7400", "812", "A", "attivo"),
            bridge(4, "Poetto", "39.2070", "9.1700", "12", "D", "manutenzione"),
            bridge(5, "Senza coordinate", "", "", "", "D", "spento"),
        ];
        let cell = |radio: &str, net: &str, lat: &str, lon: &str, range: &str| Cell {
            radio: radio.into(),
            mcc: "222".into(),
            net: net.into(),
            area: "1".into(),
            cell: format!("{net}{lat}").replace('.', ""),
            lat: lat.into(),
            lon: lon.into(),
            range: range.into(),
            samples: "40".into(),
        };
        let cells = vec![
            cell("LTE", "1", "39.2160", "9.1120", "900"),
            cell("LTE", "10", "39.2140", "9.1090", "1200"),
            cell("UMTS", "88", "39.2200", "9.1200", "700"),
            cell("LTE", "50", "39.2100", "9.1300", "1500"),
            cell("GSM", "1", "40.7259", "8.5557", "3000"),
        ];
        Self {
            comuni,
            civici,
            tim,
            openfiber,
            statuses,
            xgspon: vec!["CAG01".into()],
            fastweb,
            own,
            polylines,
            bridges,
            cells,
            log: Vec::new(),
            failing: None,
        }
    }

    fn check(&self) -> Result<()> {
        match &self.failing {
            Some(m) => Err(crate::Error::Database(m.clone())),
            None => Ok(()),
        }
    }
}

impl CoverageSource for MockSource {
    fn ping(&mut self) -> Result<String> {
        self.check()?;
        Ok(format!("demo: {} comuni, {} civici", self.comuni.len(), self.civici.len()))
    }

    fn comuni(&mut self) -> Result<Vec<Comune>> {
        self.check()?;
        Ok(self.comuni.clone())
    }

    fn streets(&mut self, comune: &Comune, query: &str, limit: usize) -> Result<Vec<Street>> {
        self.check()?;
        let words: Vec<String> = address::split_particle(query).1.split(' ').filter(|w| !w.is_empty()).map(str::to_string).collect();
        let mut out: Vec<Street> = Vec::new();
        for c in self.civici.iter().filter(|c| c.comune == comune.name) {
            if words.iter().all(|w| c.street.contains(w.as_str())) && !out.iter().any(|s| s.egon == c.egon_strada) {
                out.push(Street { egon: c.egon_strada.clone(), particle: c.particle.clone(), name: c.street.clone() });
            }
        }
        out.truncate(limit.min(limits::ROWS));
        Ok(out)
    }

    fn civici(&mut self, comune: &Comune, street: &Street, limit: usize) -> Result<Vec<Civico>> {
        self.check()?;
        Ok(self.civici.iter().filter(|c| c.comune == comune.name && c.egon_strada == street.egon).take(limit.min(limits::ROWS)).cloned().collect())
    }

    fn resolve(&mut self, comune: &Comune, via: &str, civico: &str, limit: usize) -> Result<Vec<Civico>> {
        self.check()?;
        let words: Vec<String> = address::split_particle(via).1.split(' ').filter(|w| !w.is_empty()).map(str::to_string).collect();
        let n = address::normalize_civico(civico);
        Ok(self
            .civici
            .iter()
            .filter(|c| c.comune == comune.name && words.iter().all(|w| c.street.contains(w.as_str())) && (n.is_empty() || c.civico == n))
            .take(limit.min(limits::ROWS))
            .cloned()
            .collect())
    }

    fn civico(&mut self, _comune: Option<&Comune>, egon: &str) -> Result<Option<Civico>> {
        self.check()?;
        Ok(self.civici.iter().find(|c| c.egon == egon.trim()).cloned())
    }

    fn tim(&mut self, _comune: &Comune, egon: &str) -> Result<Option<TimRow>> {
        self.check()?;
        Ok(self.tim.iter().find(|(e, _)| e == egon).map(|(_, r)| r.clone()))
    }

    fn openfiber(&mut self, egon: &str) -> Result<Vec<OpenFiberRow>> {
        self.check()?;
        Ok(self.openfiber.iter().filter(|(e, _)| e == egon).map(|(_, r)| r.clone()).collect())
    }

    fn openfiber_statuses(&mut self) -> Result<Vec<OpenFiberStatus>> {
        self.check()?;
        Ok(self.statuses.clone())
    }

    fn xgspon_pops(&mut self) -> Result<Vec<String>> {
        self.check()?;
        Ok(self.xgspon.clone())
    }

    fn fastweb(&mut self, egon: &str) -> Result<FastwebRows> {
        self.check()?;
        let mut out = FastwebRows::default();
        for (e, table, r) in &self.fastweb {
            if e == egon {
                match *table {
                    "ftth" => out.ftth.push(r.clone()),
                    "ftts" => out.ftts.push(r.clone()),
                    "vula" => out.vula.push(r.clone()),
                    _ => out.gpon.push(r.clone()),
                }
            }
        }
        Ok(out)
    }

    fn own(&mut self, egon: &str) -> Result<Vec<OwnService>> {
        self.check()?;
        Ok(self.own.iter().filter(|(e, _)| e == egon).map(|(_, s)| s.clone()).collect())
    }

    fn polyline(&mut self, egon_strada: &str) -> Result<Option<String>> {
        self.check()?;
        Ok(self.polylines.iter().find(|(e, _)| e == egon_strada).map(|(_, p)| p.clone()))
    }

    fn bridges(&mut self) -> Result<Vec<Bridge>> {
        self.check()?;
        Ok(self.bridges.clone())
    }

    fn cells(&mut self, lat: f64, lon: f64, half_deg: f64, limit: usize) -> Result<Vec<Cell>> {
        self.check()?;
        Ok(self
            .cells
            .iter()
            .filter(|c| c.point().is_some_and(|p| (p.lat - lat).abs() <= half_deg && (p.lon - lon).abs() <= half_deg))
            .take(limit.min(limits::ROWS))
            .cloned()
            .collect())
    }

    fn log(&mut self, user: &str, request: &Value, answer: &Value) -> Result<()> {
        self.check()?;
        let id = i64::try_from(self.log.len()).unwrap_or(0) + 1;
        self.log.push(HistoryEntry {
            id,
            date: crate::format_time(crate::now()),
            user: user.to_string(),
            request: request.clone(),
            answer: answer.clone(),
        });
        if self.log.len() > limits::ROWS {
            self.log.remove(0);
        }
        Ok(())
    }

    fn history(&mut self, user: &str, limit: usize) -> Result<Vec<HistoryEntry>> {
        self.check()?;
        Ok(self.log.iter().rev().filter(|e| user.is_empty() || e.user == user).take(limit.min(limits::ROWS)).cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Preferences;
    use crate::status::Status;

    #[test]
    fn the_demo_city_answers() {
        let mut m = MockSource::demo();
        assert!(m.ping().unwrap().contains("5 comuni"));
        let cagliari = m.comuni().unwrap()[0].clone();
        let streets = m.streets(&cagliari, "rom", 10).unwrap();
        assert_eq!(streets.len(), 1);
        assert_eq!(streets[0].display(), "VIA ROMA");
        assert_eq!(m.civici(&cagliari, &streets[0], 10).unwrap().len(), 4);
        let found = m.resolve(&cagliari, "via roma", "10", 10).unwrap();
        assert_eq!(found.len(), 1);
        let prefs = Preferences::default();
        let r = crate::report::gather(&mut m, &cagliari, &found[0], &prefs).unwrap();
        assert_eq!(r.best().map(|s| (s.technology.as_str(), s.status)), Some(("FTTH XGS-PON", Status::Covered)));
        assert_eq!(r.point_source, "openfiber");
        assert_eq!(r.openfiber.len(), 2);
        assert!(r.radio.iter().any(|h| h.in_reach && h.bridge.name == "Monte Urpinu"));
        assert!(!r.cells.is_empty() && r.cells.iter().all(|c| c.distance_m < 5_000.0));
        assert!(r.of(Provider::Fastweb).count() == 1 && r.of(Provider::Tim).count() >= 4);
        // A civico without Open Fiber coordinates is placed on its street.
        let c14 = m.civico(None, "100014").unwrap().unwrap();
        let r14 = crate::report::gather(&mut m, &cagliari, &c14, &prefs).unwrap();
        assert_eq!(r14.point_source, "openfiber");
        let marconi = m.civico(None, "100300").unwrap().unwrap();
        let rm = crate::report::gather(&mut m, &cagliari, &marconi, &prefs).unwrap();
        assert_eq!(rm.point_source, "polyline");
        assert!(rm.of(Provider::Own).next().is_some());
        m.log("anna", &serde_json::json!({"egon": "100010"}), &serde_json::json!({"ok": true})).unwrap();
        assert_eq!(m.history("", 10).unwrap().len(), 1);
        assert!(m.history("luca", 10).unwrap().is_empty());
        m.failing = Some("down".into());
        assert!(m.comuni().is_err());
    }
}
