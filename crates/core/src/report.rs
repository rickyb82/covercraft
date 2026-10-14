//! From the tables' rows to the report: TIM's hundred columns become a list of services,
//! Open Fiber's state codes get their meaning, Fastweb's tables their technology, the radio
//! bridges and mobile cells their distance from the civico.

use crate::config::Preferences;
use crate::geo::{self, Point};
use crate::model::*;
use crate::source::CoverageSource;
use crate::status::{self, Status};
use crate::{Result, limits};

/// A technology in TIM's row: which columns say what.
struct TimTech {
    label: &'static str,
    coverage: &'static str,
    planning: &'static str,
    down: &'static str,
    up: &'static str,
    distance: &'static str,
    details: &'static [(&'static str, &'static str)],
}

const TIM_TECHS: &[TimTech] = &[
    TimTech {
        label: "FTTH",
        coverage: "COPERTURA_BITSTREAM_NGA_VULA_FTTH",
        planning: "PIANIFICAZIONE_BITSTREAM_NGA_VULA_FTTH",
        down: "",
        up: "",
        distance: "",
        details: &[
            ("TIPO_COPERTURA_FTTH", "Tipo copertura"),
            ("EDR_BITSTREAM_NGA_VULA_FTTH", "EDR"),
            ("TIPO_EDR_BITSTREAM_NGA_VULA_FTTH", "Tipo EDR"),
            ("DIRETTO_INFRATEL", "Diretto Infratel"),
        ],
    },
    TimTech {
        label: "FTTH XGS-PON",
        coverage: "COPERTURA_BITSTREAM_NGA_VULA_XGSPON",
        planning: "PIANIFICAZIONE_BITSTREAM_NGA_VULA_XGSPON",
        down: "",
        up: "",
        distance: "",
        details: &[],
    },
    TimTech {
        label: "FTTC (VULA)",
        coverage: "COPERTURA_BITSTREAM_NGA_VULA_FTTCab",
        planning: "PIANIFICAZIONE_BITSTREAM_NGA_VULA_FTTCab",
        down: "VELOCITA_DOWN_BITSTREAM_NGA_VULA_FTTCab",
        up: "VELOCITA_UP_BITSTREAM_NGA_VULA_FTTCab",
        distance: "DISTANZA_BITSTREAM_NGA_VULA_FTTCab",
        details: &[
            ("EDR_BITSTREAM_NGA_VULA_FTTCab", "Armadio"),
            ("TIPO_EDR_BITSTREAM_NGA_VULA_FTTCab", "Tipo armadio"),
            ("CLLI_CENTRALE_OLT_BITSTREAM_NGA_VULA_FTTCab", "Centrale OLT"),
            ("PREVISIONE_AMPL_BITSTREAM_NGA_VULA_FTTCab", "Previsione ampliamento"),
        ],
    },
    TimTech {
        label: "FTTC EVDSL",
        coverage: "COPERTURA_EVDSL_BITSTREAM_NGA_VULA_FTTCab",
        planning: "",
        down: "VELOCITA_DOWN_EVDSL_BITSTREAM_NGA_VULA_FTTCab",
        up: "VELOCITA_UP_EVDSL_BITSTREAM_NGA_VULA_FTTCab",
        distance: "DISTANZA_BITSTREAM_NGA_VULA_FTTCab",
        details: &[],
    },
    TimTech {
        label: "SLU (sub-loop)",
        coverage: "COPERTURA_SLU",
        planning: "",
        down: "VELOCITA_DOWN_SLU",
        up: "VELOCITA_UP_SLU",
        distance: "DISTANZA_SLU",
        details: &[("EDR_SLU", "Armadio"), ("TIPO_EDR_SLU", "Tipo armadio"), ("ARMADIO_PADRE_SLU", "Armadio padre")],
    },
    TimTech {
        label: "ADSL (ATM)",
        coverage: "COPERTURA_BITSTREAM_ASIM_ATM",
        planning: "PIANIFICAZIONE_BITSTREAM_ASIM_ATM",
        down: "VELOCITA_DOWN_6dB_BITSTREAM_ASIM_ATM",
        up: "VELOCITA_UP_6dB_BITSTREAM_ASIM_ATM",
        distance: "DISTANZA_BITSTREAM_ASIM_ATM",
        details: &[
            ("VELOCITA_DOWN_11dB_BITSTREAM_ASIM_ATM", "Down 11 dB"),
            ("VELOCITA_DOWN_12dB_BITSTREAM_ASIM_ATM", "Down 12 dB"),
            ("EDR_BITSTREAM_ASIM_ATM", "Centrale"),
            ("TIPO_EDR_BITSTREAM_ASIM_ATM", "Tipo"),
        ],
    },
    TimTech {
        label: "ADSL (Ethernet)",
        coverage: "COPERTURA_BITSTREAM_ASIM_ETHERNET",
        planning: "PIANIFICAZIONE_BITSTREAM_ASIM_ETH",
        down: "VELOCITA_DOWN_6dB_BITSTREAM_ASIM_ETHERNET",
        up: "VELOCITA_UP_6dB_BITSTREAM_ASIM_ETHERNET",
        distance: "DISTANZA_BITSTREAM_ASIM_ETHERNET",
        details: &[
            ("VELOCITA_DOWN_11dB_BITSTREAM_ASIM_ETHERNET", "Down 11 dB"),
            ("VELOCITA_DOWN_12dB_BITSTREAM_ASIM_ETHERNET", "Down 12 dB"),
            ("EDR_BITSTREAM_ASIM_ETHERNET", "Centrale"),
            ("TIPO_EDR_BITSTREAM_ASIM_ETHERNET", "Tipo"),
            ("TIPO_DSLAM_BITSTREAM_ASIM_ETHERNET", "DSLAM"),
            ("STATO_SERVIZIO_BITSTREAM_ASIM_ETHERNET", "Stato servizio"),
            ("PREVISIONE_AMPL_BITSTREAM_ASIM_ETHERNET", "Previsione ampliamento"),
        ],
    },
    TimTech {
        label: "SHDSL ATM 2-8M",
        coverage: "COPERTURA_BITSTREAM_SIMM_ATM_2M_4M_6M_8M",
        planning: "PIANIFICAZIONE_BITSTREAM_SIMM_ATM_2M_4M_6M_8M",
        down: "",
        up: "",
        distance: "DISTANZA_BITSTREAM_SIMM_ATM_2M_4M_6M_8M",
        details: &[("EDR_BITSTREAM_SIMM_ATM_2M_4M_6M_8M", "Centrale"), ("TIPO_EDR_BITSTREAM_SIMM_ATM_2M_4M_6M_8M", "Tipo")],
    },
    TimTech {
        label: "SHDSL ATM bonding",
        coverage: "COPERTURA_BITSTREAM_SIMM_ATM_1M_1.6M_4M_Bonding",
        planning: "PIANIFICAZIONE_BITSTREAM_SIMM_ATM_1M_1.6M_4M_Bonding",
        down: "",
        up: "",
        distance: "DISTANZA_BITSTREAM_SIMM_ATM_1M_1.6M_4M_Bonding",
        details: &[("EDR_BITSTREAM_SIMM_ATM_1M_1.6M_4M_Bonding", "Centrale")],
    },
    TimTech {
        label: "SHDSL Ethernet 2-10M",
        coverage: "COPERTURA_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M",
        planning: "PIANIFICAZIONE_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M",
        down: "",
        up: "",
        distance: "DISTANZA_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M",
        details: &[("EDR_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M", "Centrale"), ("TIPO_EDR_BITSTREAM_SIMM_ETHERNET_2M_4M_6M_8M_10M", "Tipo")],
    },
    TimTech {
        label: "ULL (rame)",
        coverage: "COPERTURA_ULL",
        planning: "",
        down: "VELOCITA_DOWN_ULL_VDSL",
        up: "VELOCITA_UP_ULL_VDSL",
        distance: "DISTANZA_ULL",
        details: &[
            ("VELOCITA_DOWN_ULL_ADSL2+", "Down ADSL2+"),
            ("VELOCITA_UP_ULL_ADSL2+", "Up ADSL2+"),
            ("EDR_ULL", "Centrale"),
            ("TIPO_EDR_ULL", "Tipo"),
        ],
    },
    TimTech {
        label: "WLR (voce)",
        coverage: "COPERTURA_WLR",
        planning: "",
        down: "",
        up: "",
        distance: "DISTANZA_WLR",
        details: &[("EDR_WLR", "Centrale"), ("TIPO_EDR_WLR", "Tipo")],
    },
    TimTech {
        label: "Fibra dedicata (circuiti)",
        coverage: "COPERTURA_CIRCUITI_TERM_ETH_FIBRA_OTTICA",
        planning: "PIANIFICAZIONE_CIRCUITI_TERM_ETH_FIBRA_OTTICA",
        down: "",
        up: "",
        distance: "",
        details: &[
            ("FASCIA_CIRCUITI_TERM_ETH_FIBRA_OTTICA", "Fascia"),
            ("EDR_CIRCUITI_TERM_ETH_FIBRA_OTTICA", "Centrale"),
            ("TIPO_EDR_CIRCUITI_TERM_ETH_FIBRA_OTTICA", "Tipo"),
        ],
    },
];

/// General columns of TIM's row shown as facts of the civico.
const TIM_INFO: &[(&str, &str)] = &[
    ("CATEGORIA", "Categoria"),
    ("COPERTURA_PER_PROSSIMITA", "Copertura per prossimità"),
    ("TOTALE_UI", "Unità immobiliari"),
    ("BUSINESS", "Business"),
    ("BANDO_PUBBLICO", "Bando pubblico"),
    ("AREA", "Area"),
    ("SWITCH_OFF", "Switch-off rame"),
    ("SPECIAL_PRICE", "Special price"),
    ("FLAG_OPA", "OPA"),
    ("DR", "DR"),
    ("CODICE_TOP_SITE", "Codice sito"),
];

fn cell(row: &TimRow, key: &str) -> String {
    row.get(key).map(|v| v.trim().to_string()).unwrap_or_default()
}

/// TIM's row as services.
pub fn tim_services(row: &TimRow, prefs: &Preferences) -> Vec<Service> {
    let mut out = Vec::new();
    for t in TIM_TECHS {
        let raw = cell(row, t.coverage);
        let planned = if t.planning.is_empty() { String::new() } else { cell(row, t.planning) };
        if raw.is_empty() && planned.is_empty() {
            continue;
        }
        let mut st = status::interpret(&raw, &prefs.rules);
        if st != Status::Covered && !planned.is_empty() && status::interpret(&planned, &prefs.rules) != Status::NotCovered {
            st = Status::Planned;
        }
        let details: Vec<(String, String)> =
            t.details.iter().map(|(k, l)| ((*l).to_string(), cell(row, k))).filter(|(_, v)| !v.is_empty() && v != "-").collect();
        out.push(Service {
            provider: Some(Provider::Tim),
            technology: t.label.to_string(),
            status: st,
            raw,
            planned,
            speed_down: if t.down.is_empty() { String::new() } else { cell(row, t.down) },
            speed_up: if t.up.is_empty() { String::new() } else { cell(row, t.up) },
            distance_m: if t.distance.is_empty() { None } else { status::distance_m(&cell(row, t.distance)) },
            details,
        });
    }
    out
}

/// The facts of the civico from TIM's row.
pub fn tim_info(row: &TimRow) -> Vec<(String, String)> {
    TIM_INFO.iter().map(|(k, l)| ((*l).to_string(), cell(row, k))).filter(|(_, v)| !v.is_empty() && v != "-").collect()
}

fn of_state(code: &str, cluster: &str, statuses: &[OpenFiberStatus], prefs: &Preferences) -> (Status, String) {
    let c = code.trim();
    if c.is_empty() {
        return (Status::Unknown, String::new());
    }
    let n = c.parse::<i64>().ok();
    let found = statuses
        .iter()
        .find(|s| Some(s.value) == n && s.cluster.eq_ignore_ascii_case(cluster.trim()))
        .or_else(|| statuses.iter().find(|s| Some(s.value) == n));
    match found {
        Some(s) => {
            let text = if s.friendly.trim().is_empty() { s.description.clone() } else { s.friendly.clone() };
            let st = match status::interpret(&text, &prefs.rules) {
                Status::Unknown => status::interpret(&s.description, &prefs.rules),
                st => st,
            };
            (st, text)
        }
        None => (status::interpret(c, &prefs.rules), c.to_string()),
    }
}

/// Open Fiber's rows spelled out, plus one service summarizing them.
pub fn openfiber_services(
    rows: &[OpenFiberRow],
    statuses: &[OpenFiberStatus],
    xgspon: &[String],
    prefs: &Preferences,
) -> (Vec<OpenFiberScala>, Option<Service>) {
    let mut scale = Vec::new();
    for r in rows.iter().take(limits::ROWS) {
        let (bs, bt) = of_state(&r.building_state, &r.cluster, statuses, prefs);
        let (ss, st) = of_state(&r.scala_state, &r.cluster, statuses, prefs);
        let is_xgs = !r.pop.trim().is_empty() && xgspon.iter().any(|p| p.trim().eq_ignore_ascii_case(r.pop.trim()));
        scale.push(OpenFiberScala { row: r.clone(), building_status: bs, building_text: bt, scala_status: ss, scala_text: st, xgspon: is_xgs });
    }
    if scale.is_empty() {
        return (scale, None);
    }
    let xgs = scale.iter().any(|s| s.xgspon);
    let best = scale.iter().max_by_key(|s| (s.scala_status.max(s.building_status), s.scala_status)).cloned();
    let Some(b) = best else { return (scale, None) };
    let st = b.scala_status.max(b.building_status);
    let st = if st == Status::Unknown && !b.row.rfa_actual.trim().is_empty() { Status::Covered } else { st };
    let planned = [&b.row.rfa_planned, &b.row.rfc_planned].iter().map(|s| s.trim()).find(|s| !s.is_empty()).unwrap_or_default().to_string();
    let st = if st < Status::Planned && !planned.is_empty() { Status::Planned } else { st };
    let first = |f: fn(&OpenFiberRow) -> &str| scale.iter().map(|s| f(&s.row).trim()).find(|v| !v.is_empty()).unwrap_or_default().to_string();
    let mut details = vec![
        ("POP".to_string(), first(|r| &r.pop)),
        ("Cluster".to_string(), first(|r| &r.cluster)),
        ("Scale".to_string(), scale.len().to_string()),
        ("Unità immobiliari".to_string(), b.row.units.clone()),
        ("RFC effettiva".to_string(), b.row.rfc_actual.clone()),
        ("RFA effettiva".to_string(), b.row.rfa_actual.clone()),
        ("Stato building".to_string(), b.building_text.clone()),
        ("Stato scala".to_string(), b.scala_text.clone()),
    ];
    details.retain(|(_, v)| !v.trim().is_empty());
    let service = Service {
        provider: Some(Provider::OpenFiber),
        technology: if xgs { "FTTH XGS-PON".into() } else { "FTTH".into() },
        status: st,
        raw: if b.scala_text.is_empty() { b.building_text.clone() } else { b.scala_text.clone() },
        planned,
        speed_down: if xgs { "10G".into() } else { "1G".into() },
        speed_up: String::new(),
        distance_m: None,
        details,
    };
    (scale, Some(service))
}

/// Fastweb's tables as services.
pub fn fastweb_services(rows: &FastwebRows, prefs: &Preferences) -> Vec<Service> {
    let mut out = Vec::new();
    let mut push = |rows: &[TimRow], label: &str, cov: &str, planned: &str, down: &str, up: &str, dist: &str, details: &[(&str, &str)]| {
        for r in rows.iter().take(limits::ROWS) {
            let raw = cell(r, cov);
            let planned_v = if planned.is_empty() { String::new() } else { cell(r, planned) };
            let mut st = status::interpret(&raw, &prefs.rules);
            if st != Status::Covered && !planned_v.is_empty() && status::interpret(&planned_v, &prefs.rules) != Status::NotCovered {
                st = Status::Planned;
            }
            let d: Vec<(String, String)> =
                details.iter().map(|(k, l)| ((*l).to_string(), cell(r, k))).filter(|(_, v)| !v.is_empty() && v != "-").collect();
            out.push(Service {
                provider: Some(Provider::Fastweb),
                technology: label.to_string(),
                status: st,
                raw,
                planned: planned_v,
                speed_down: if down.is_empty() { String::new() } else { cell(r, down) },
                speed_up: if up.is_empty() { String::new() } else { cell(r, up) },
                distance_m: if dist.is_empty() { None } else { status::distance_m(&cell(r, dist)) },
                details: d,
            });
        }
    };
    push(
        &rows.ftth,
        "FTTH (GPON)",
        "stato_vendibilita_gpon",
        "previsione_ampliamento_gpon",
        "",
        "",
        "",
        &[("stato_tecnico_gpon", "Stato tecnico"), ("flag_opa", "OPA")],
    );
    push(
        &rows.gpon,
        "GPON (indirizzo)",
        "stato_copertura_indirizzo",
        "data_pianificazione_prevista",
        "",
        "",
        "",
        &[("2_macroaree", "Macroarea"), ("8_macroaree", "Macroarea (8)"), ("data_ultima_variazione_copertura_indirizzo", "Ultima variazione")],
    );
    push(
        &rows.ftts,
        "FTTS (FTTC)",
        "stato_vendibilita_ftts",
        "previsione_ampliamento_ftts",
        "velocita_down_ftts",
        "velocita_up_ftts",
        "distanza_civico_cabinet",
        &[
            ("stato_tecnico_ftts", "Stato tecnico"),
            ("tipo_ftts", "Tipo"),
            ("id_cabinet", "Cabinet"),
            ("nome_sgu_ftts", "Centrale"),
            ("categoria", "Categoria"),
            ("flag_opa", "OPA"),
        ],
    );
    push(
        &rows.vula,
        "VULA (FTTC)",
        "stato_vendibilita_vula",
        "previsione_ampliamento_vula",
        "velocita_down_vula",
        "velocita_up_vula",
        "distanza_cabinet_vula",
        &[
            ("stato_tecnico_vula", "Stato tecnico"),
            ("tipo_vula", "Tipo"),
            ("id_cabinet", "Cabinet"),
            ("nome_centrale", "Centrale"),
            ("categoria", "Categoria"),
        ],
    );
    out
}

/// The operator's own services.
pub fn own_services(rows: &[OwnService], prefs: &Preferences) -> Vec<Service> {
    rows.iter()
        .take(limits::ROWS)
        .map(|r| Service {
            provider: Some(Provider::Own),
            technology: r.service.clone(),
            status: status::interpret(&r.state, &prefs.rules),
            raw: r.state.clone(),
            planned: String::new(),
            speed_down: String::new(),
            speed_up: String::new(),
            distance_m: None,
            details: [("Dal".to_string(), r.since.clone()), ("Note".to_string(), r.note.clone())]
                .into_iter()
                .filter(|(_, v)| !v.trim().is_empty())
                .collect(),
        })
        .collect()
}

/// The bridges around a point, nearest first; those in reach always listed.
pub fn radio_hits(point: Point, bridges: &[Bridge], prefs: &Preferences) -> Vec<RadioHit> {
    let mut hits: Vec<RadioHit> = bridges
        .iter()
        .take(limits::BRIDGES)
        .filter_map(|b| {
            let p = b.point()?;
            let distance_m = point.distance_m(p);
            let bearing_deg = point.bearing_deg(p);
            let reach_m = prefs.reach_m(&b.band);
            Some(RadioHit {
                bridge: b.clone(),
                distance_m,
                bearing_deg,
                compass: geo::compass(bearing_deg).to_string(),
                in_reach: distance_m <= reach_m,
                reach_m,
            })
        })
        .collect();
    hits.sort_by(|a, b| a.distance_m.total_cmp(&b.distance_m));
    let keep = prefs.max_bridges.max(1);
    let mut out: Vec<RadioHit> = Vec::new();
    for (i, h) in hits.into_iter().enumerate() {
        if i < keep || h.in_reach {
            out.push(h);
        }
    }
    out
}

/// The cells around a point, nearest first.
pub fn cell_hits(point: Point, cells: &[Cell], prefs: &Preferences) -> Vec<CellHit> {
    let mut hits: Vec<CellHit> = cells
        .iter()
        .take(limits::ROWS)
        .filter_map(|c| {
            let p = c.point()?;
            let distance_m = point.distance_m(p);
            let range = geo::parse_number(&c.range).unwrap_or(0.0);
            Some(CellHit { operator: operator_name(&c.mcc, &c.net), cell: c.clone(), distance_m, in_range: range > 0.0 && distance_m <= range })
        })
        .collect();
    hits.sort_by(|a, b| a.distance_m.total_cmp(&b.distance_m));
    hits.truncate(prefs.max_cells.max(1));
    hits
}

/// Everything read for one civico.
#[derive(Clone, Debug, Default)]
pub struct Inputs {
    pub civico: Civico,
    pub tim: Option<TimRow>,
    pub openfiber: Vec<OpenFiberRow>,
    pub statuses: Vec<OpenFiberStatus>,
    pub xgspon_pops: Vec<String>,
    pub fastweb: FastwebRows,
    pub own: Vec<OwnService>,
    pub polyline: Option<String>,
    pub bridges: Vec<Bridge>,
    pub cells: Vec<Cell>,
}

/// The civico's point: Open Fiber's building coordinates, else the street's polyline.
pub fn locate(openfiber: &[OpenFiberRow], polyline: Option<&str>) -> (Option<Point>, &'static str) {
    if let Some(p) = openfiber.iter().find_map(|r| geo::parse_point(&r.coordinates)) {
        return (Some(p), "openfiber");
    }
    if let Some(text) = polyline
        && let Some(p) = geo::midpoint(&geo::parse_polyline(text))
    {
        return (Some(p), "polyline");
    }
    (None, "none")
}

/// Builds the report from what was read.
pub fn build(inputs: Inputs, prefs: &Preferences) -> Report {
    let mut r = Report { civico: inputs.civico, time: crate::now(), ..Default::default() };
    if let Some(row) = &inputs.tim {
        r.services.extend(tim_services(row, prefs));
        r.info = tim_info(row);
    } else {
        r.notes.push("tim_missing".into());
    }
    let (scale, of_service) = openfiber_services(&inputs.openfiber, &inputs.statuses, &inputs.xgspon_pops, prefs);
    r.openfiber = scale;
    if let Some(s) = of_service {
        r.services.push(s);
    } else {
        r.notes.push("openfiber_missing".into());
    }
    let fw = fastweb_services(&inputs.fastweb, prefs);
    if fw.is_empty() {
        r.notes.push("fastweb_missing".into());
    }
    r.services.extend(fw);
    r.services.extend(own_services(&inputs.own, prefs));
    let (point, source) = locate(&inputs.openfiber, inputs.polyline.as_deref());
    r.point = point;
    r.point_source = source.into();
    match point {
        Some(p) => {
            r.radio = radio_hits(p, &inputs.bridges, prefs);
            if prefs.cells {
                r.cells = cell_hits(p, &inputs.cells, prefs);
            }
        }
        None => r.notes.push("no_point".into()),
    }
    r
}

/// Reads everything for a civico from a source and builds the report.
pub fn gather(source: &mut dyn CoverageSource, comune: &Comune, civico: &Civico, prefs: &Preferences) -> Result<Report> {
    let egon = civico.egon.trim();
    let tim = if comune.table.is_empty() { None } else { source.tim(comune, egon)? };
    let openfiber = source.openfiber(egon)?;
    let statuses = if openfiber.is_empty() { Vec::new() } else { source.openfiber_statuses()? };
    let xgspon_pops = if openfiber.is_empty() { Vec::new() } else { source.xgspon_pops()? };
    let fastweb = source.fastweb(egon)?;
    let own = source.own(egon)?;
    let polyline = if openfiber.iter().any(|r| !r.coordinates.trim().is_empty()) || civico.egon_strada.trim().is_empty() {
        None
    } else {
        source.polyline(civico.egon_strada.trim())?
    };
    let (point, _) = locate(&openfiber, polyline.as_deref());
    let bridges = if point.is_some() { source.bridges()? } else { Vec::new() };
    let cells = match point {
        Some(p) if prefs.cells => {
            let half = (prefs.cells_radius_km.clamp(0.1, 50.0) / 111.0).max(0.001);
            source.cells(p.lat, p.lon, half, limits::ROWS)?
        }
        _ => Vec::new(),
    };
    Ok(build(Inputs { civico: civico.clone(), tim, openfiber, statuses, xgspon_pops, fastweb, own, polyline, bridges, cells }, prefs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tim_row_becomes_services() {
        let mut row = TimRow::new();
        row.insert("COPERTURA_BITSTREAM_NGA_VULA_FTTH".into(), "SI".into());
        row.insert("TIPO_COPERTURA_FTTH".into(), "FIBERCOP".into());
        row.insert("COPERTURA_BITSTREAM_NGA_VULA_FTTCab".into(), "NO".into());
        row.insert("PIANIFICAZIONE_BITSTREAM_NGA_VULA_FTTCab".into(), "30/06/2027".into());
        row.insert("COPERTURA_BITSTREAM_ASIM_ETHERNET".into(), "SI".into());
        row.insert("VELOCITA_DOWN_6dB_BITSTREAM_ASIM_ETHERNET".into(), "20M".into());
        row.insert("DISTANZA_BITSTREAM_ASIM_ETHERNET".into(), "1250".into());
        row.insert("TOTALE_UI".into(), "12".into());
        let prefs = Preferences::default();
        let s = tim_services(&row, &prefs);
        assert_eq!(s.len(), 3);
        assert_eq!((s[0].technology.as_str(), s[0].status), ("FTTH", Status::Covered));
        assert_eq!(s[0].details, vec![("Tipo copertura".to_string(), "FIBERCOP".to_string())]);
        assert_eq!((s[1].technology.as_str(), s[1].status, s[1].planned.as_str()), ("FTTC (VULA)", Status::Planned, "30/06/2027"));
        assert_eq!((s[2].speed_down.as_str(), s[2].distance_m), ("20M", Some(1250)));
        assert_eq!(tim_info(&row), vec![("Unità immobiliari".to_string(), "12".to_string())]);
    }

    #[test]
    fn openfiber_fastweb_own_and_radio() {
        let prefs = Preferences::default();
        let statuses = vec![
            OpenFiberStatus { value: 1, cluster: "C".into(), description: "RFA".into(), friendly: "Vendibile".into() },
            OpenFiberStatus { value: 0, cluster: "C".into(), description: "In pianificazione".into(), friendly: String::new() },
        ];
        let rows = vec![
            OpenFiberRow {
                id_scala: "S1".into(),
                coordinates: "39.2238,9.1217".into(),
                pop: "CAG01".into(),
                building_state: "1".into(),
                scala_state: "0".into(),
                cluster: "C".into(),
                rfc_planned: "31/12/2026".into(),
                ..Default::default()
            },
            OpenFiberRow {
                id_scala: "S2".into(),
                building_state: "1".into(),
                scala_state: "1".into(),
                cluster: "C".into(),
                rfa_actual: "01/02/2025".into(),
                ..Default::default()
            },
        ];
        let (scale, service) = openfiber_services(&rows, &statuses, &["cag01".into()], &prefs);
        assert_eq!(scale.len(), 2);
        assert_eq!(scale[0].scala_status, Status::Planned);
        let s = service.unwrap();
        assert_eq!((s.technology.as_str(), s.status), ("FTTH XGS-PON", Status::Covered));
        assert!(s.details.iter().any(|(k, v)| k == "POP" && v == "CAG01"));
        let (none, no_service) = openfiber_services(&[], &statuses, &[], &prefs);
        assert!(none.is_empty() && no_service.is_none());

        let mut ftts = TimRow::new();
        ftts.insert("stato_vendibilita_ftts".into(), "VENDIBILE".into());
        ftts.insert("velocita_down_ftts".into(), "200".into());
        ftts.insert("distanza_civico_cabinet".into(), "0,3 km".into());
        let fw = fastweb_services(&FastwebRows { ftts: vec![ftts], ..Default::default() }, &prefs);
        assert_eq!(fw.len(), 1);
        assert_eq!((fw[0].status, fw[0].distance_m), (Status::Covered, Some(300)));

        let own = own_services(&[OwnService { service: "Radio".into(), state: "Attivo".into(), since: "2025".into(), note: String::new() }], &prefs);
        assert_eq!(own[0].status, Status::Covered);
        assert_eq!(own[0].details.len(), 1);

        let here = Point { lat: 39.2238, lon: 9.1217 };
        let bridges = vec![
            Bridge { id: 1, name: "Urpinu".into(), lat: "39.231".into(), lon: "9.128".into(), band: "D".into(), ..Default::default() },
            Bridge { id: 2, name: "Lontano".into(), lat: "40.7".into(), lon: "8.5".into(), band: "A".into(), ..Default::default() },
            Bridge { id: 3, name: "Senza coordinate".into(), ..Default::default() },
        ];
        let hits = radio_hits(here, &bridges, &prefs);
        assert_eq!(hits.len(), 2);
        assert!(hits[0].in_reach && hits[0].bridge.name == "Urpinu" && hits[0].distance_m < 1_200.0);
        assert!(!hits[1].in_reach);
        let cells =
            vec![Cell { mcc: "222".into(), net: "10".into(), lat: "39.224".into(), lon: "9.122".into(), range: "1000".into(), ..Default::default() }];
        let ch = cell_hits(here, &cells, &prefs);
        assert_eq!((ch[0].operator.as_str(), ch[0].in_range), ("Vodafone", true));
    }

    #[test]
    fn builds_a_report_with_notes_for_what_is_missing() {
        let prefs = Preferences::default();
        let r = build(Inputs { polyline: Some("39.22,9.12 39.23,9.13 39.24,9.14".into()), ..Default::default() }, &prefs);
        assert_eq!(r.point_source, "polyline");
        assert!(r.notes.contains(&"tim_missing".to_string()) && r.notes.contains(&"openfiber_missing".to_string()));
        assert!(r.best().is_none());
        let r2 = build(Inputs::default(), &prefs);
        assert!(r2.notes.contains(&"no_point".to_string()));
        assert_eq!(locate(&[], None).1, "none");
    }
}
