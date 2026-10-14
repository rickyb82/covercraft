//! The real schema (`schema/netmap.sql`, a MySQL dump) turned into SQLite statements, and the
//! demo city written into it: the SQL of this crate runs on the true column names in tests
//! and in the apps' demo mode.

use covercraft_core::mock::MockSource;
use covercraft_core::model::TimRow;
use covercraft_core::source::CoverageSource;
use covercraft_core::{Error, Result};

use crate::sql::{SqlSource, table_ident};

/// The MySQL dump of the `netmap` schema.
pub const MYSQL_SCHEMA: &str = include_str!("../../../schema/netmap.sql");

/// The TIM table of the dump (one per comune in production).
pub const TIM_SAMPLE_TABLE: &str = "netmap_0000953650178cf38c94f42160f0bb42";

/// The dump's `CREATE TABLE` statements rewritten for SQLite.
pub fn sqlite_schema() -> Vec<String> {
    let mut out = Vec::new();
    let mut body: Option<(String, Vec<String>)> = None;
    for line in MYSQL_SCHEMA.lines() {
        let l = line.trim();
        if let Some((name, entries)) = &mut body {
            if l.starts_with(')') {
                out.push(format!("CREATE TABLE IF NOT EXISTS `{name}` (\n  {}\n)", finish(entries)));
                body = None;
            } else if !l.is_empty() {
                entries.push(l.trim_end_matches(',').to_string());
            }
            continue;
        }
        if let Some(rest) = l.strip_prefix("CREATE TABLE `")
            && let Some((name, _)) = rest.split_once('`')
        {
            body = Some((name.to_string(), Vec::new()));
        }
    }
    out
}

/// Column entries → SQLite: no `KEY` indexes, `AUTO_INCREMENT` as the rowid, no `ON UPDATE`.
fn finish(entries: &mut [String]) -> String {
    let mut auto: Option<String> = None;
    let mut cols: Vec<String> = Vec::new();
    for e in entries.iter() {
        if e.starts_with("KEY ") || e.starts_with("UNIQUE KEY ") {
            continue;
        }
        if e.starts_with("PRIMARY KEY") {
            cols.push(e.split(" USING").next().unwrap_or(e).to_string());
            continue;
        }
        let mut e = e.replace(" ON UPDATE CURRENT_TIMESTAMP", "").replace(" CHARACTER SET utf8", "");
        if e.contains("AUTO_INCREMENT")
            && let Some((name, _)) = e.trim_start_matches('`').split_once('`')
        {
            auto = Some(name.to_string());
            e = format!("`{name}` INTEGER PRIMARY KEY AUTOINCREMENT");
        }
        cols.push(e);
    }
    if let Some(a) = auto {
        cols.retain(|c| !(c.starts_with("PRIMARY KEY") && c.contains(&format!("`{a}`"))));
    }
    cols.join(",\n  ")
}

/// Creates the schema (plus TIM tables for the demo comuni) in an SQLite source.
pub fn create_schema(src: &mut SqlSource, tim_tables: &[&str]) -> Result<()> {
    if src.dialect() != "sqlite" {
        return Err(Error::Invalid("the demo schema is for SQLite".into()));
    }
    let statements = sqlite_schema();
    if statements.is_empty() {
        return Err(Error::Invalid("schema/netmap.sql has no tables".into()));
    }
    for s in &statements {
        src.execute(s, &[])?;
    }
    let tim = statements.iter().find(|s| s.contains(TIM_SAMPLE_TABLE)).ok_or_else(|| Error::Invalid("no TIM table in the schema".into()))?;
    for t in tim_tables {
        let ident = table_ident(t)?;
        src.execute(&tim.replace(&format!("`{TIM_SAMPLE_TABLE}`"), &ident), &[])?;
    }
    Ok(())
}

fn insert(src: &mut SqlSource, table: &str, row: &[(&str, &str)]) -> Result<()> {
    let cols: Vec<String> = row.iter().map(|(k, _)| format!("`{k}`")).collect();
    let marks: Vec<&str> = row.iter().map(|_| "?").collect();
    let params: Vec<&str> = row.iter().map(|(_, v)| *v).collect();
    src.execute(&format!("INSERT INTO {} ({}) VALUES ({})", table_ident(table)?, cols.join(", "), marks.join(", ")), &params)?;
    Ok(())
}

fn insert_map(src: &mut SqlSource, table: &str, row: &TimRow) -> Result<()> {
    let pairs: Vec<(&str, &str)> = row.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    insert(src, table, &pairs)
}

/// Writes the demo city into an empty SQLite source (schema included).
pub fn fill(src: &mut SqlSource) -> Result<()> {
    let m = MockSource::demo();
    let tables: Vec<&str> = m.comuni.iter().map(|c| c.table.as_str()).filter(|t| !t.is_empty()).collect();
    create_schema(src, &tables)?;
    for c in &m.comuni {
        insert(src, "comuni", &[("comune", &c.name), ("provincia", &c.provincia), ("table_name", &c.table)])?;
        insert(
            src,
            "istat",
            &[
                ("codice_comune", &c.istat),
                ("nome_comune", &c.name),
                ("sigla_provincia", &c.provincia),
                ("nome_regione", &c.regione),
                ("nome_provincia", &c.provincia_name),
            ],
        )?;
    }
    let cagliari = m.comuni.first().ok_or_else(|| Error::Invalid("demo".into()))?;
    for (i, (egon, row)) in m.tim.iter().enumerate() {
        let Some(c) = m.civici.iter().find(|c| &c.egon == egon) else { continue };
        let mut full = row.clone();
        full.insert("PROGRESSIVO_RIGA".into(), (i + 1).to_string());
        full.insert("CODICE_EGON_CLIENTE".into(), c.egon.clone());
        full.insert("CODICE_EGON_STRADA_CLIENTE".into(), c.egon_strada.clone());
        full.insert("PART_TOP_CLIENTE".into(), c.particle.clone());
        full.insert("STRADA_CLIENTE".into(), c.street.clone());
        full.insert("CIVICO_CLIENTE".into(), c.civico.clone());
        full.insert("COMUNE_CLIENTE".into(), c.comune.clone());
        full.insert("PROVINCIA_CLIENTE".into(), c.provincia.clone());
        full.insert("CODICE_ISTAT_COMUNE".into(), cagliari.istat.clone());
        insert_map(src, &cagliari.table, &full)?;
    }
    for (egon, r) in &m.openfiber {
        let Some(c) = m.civici.iter().find(|c| &c.egon == egon) else { continue };
        insert(
            src,
            "openfiber",
            &[
                ("ID_SCALA", &r.id_scala),
                ("REGIONE", "SARDEGNA"),
                ("PROVINCIA", &c.provincia),
                ("COMUNE", &c.comune),
                ("PARTICELLA_TOP", &c.particle),
                ("INDIRIZZO", &c.street),
                ("CIVICO", &c.civico),
                ("SCALA_PALAZZINA", &r.scala),
                ("ID_BUILDING", &r.building),
                ("COORDINATE_BUILDING", &r.coordinates),
                ("POP", &r.pop),
                ("TOTALE_UI", &r.units),
                ("STATO_BUILDING", &r.building_state),
                ("STATO_SCALA_PALAZZINA", &r.scala_state),
                ("DATA_RFC_INDICATIVA", &r.rfc_planned),
                ("DATA_RFC_EFFETTIVA", &r.rfc_actual),
                ("DATA_RFA_INDICATIVA", &r.rfa_planned),
                ("DATA_RFA_EFFETTIVA", &r.rfa_actual),
                ("DATA_ULTIMA_MODIFICA_RECORD", &r.updated),
                ("ID_EGON_CIVICO", &c.egon),
                ("ID_EGON_STRADA", &c.egon_strada),
                ("CLUSTER", &r.cluster),
            ],
        )?;
    }
    for s in &m.statuses {
        let v = s.value.to_string();
        insert(
            src,
            "openfiber_status",
            &[("value", &v), ("description", &s.description), ("cluster", &s.cluster), ("friendly_description", &s.friendly)],
        )?;
    }
    for p in &m.xgspon {
        insert(src, "openfiber_xgpon_pop", &[("pop", p)])?;
    }
    for (i, (egon, table, row)) in m.fastweb.iter().enumerate() {
        let Some(c) = m.civici.iter().find(|c| &c.egon == egon) else { continue };
        let mut full = row.clone();
        full.insert("id_egon_civico".into(), c.egon.clone());
        if *table == "gpon" {
            full.insert("id_egon_strada".into(), c.egon_strada.clone());
            full.insert("citta".into(), c.comune.clone());
            full.insert("particella_toponomastica".into(), c.particle.clone());
            full.insert("indirizzo".into(), c.street.clone());
            full.insert("civico".into(), c.civico.clone());
        } else {
            full.insert("id_record".into(), (i + 1).to_string());
            full.insert("id_egon_via".into(), c.egon_strada.clone());
            full.insert("particella_top_egon".into(), c.particle.clone());
            full.insert("nome_via_egon".into(), c.street.clone());
            full.insert("civico_top_egon".into(), c.civico.clone());
            full.insert("comune_egon".into(), c.comune.clone());
            full.insert("sigla_provincia_egon".into(), c.provincia.clone());
        }
        insert_map(src, &format!("{table}_fw"), &full)?;
    }
    let mut services: Vec<String> = Vec::new();
    let mut states: Vec<String> = Vec::new();
    for (egon, o) in &m.own {
        let Some(c) = m.civici.iter().find(|c| &c.egon == egon) else { continue };
        if !services.contains(&o.service) {
            services.push(o.service.clone());
            let id = services.len().to_string();
            insert(src, "sirius_tipi_servizio", &[("id_tipo_servizio", &id), ("name", &o.service)])?;
        }
        if !states.contains(&o.state) {
            states.push(o.state.clone());
            let id = states.len().to_string();
            insert(src, "sirius_stati", &[("id_stato", &id), ("name", &o.state)])?;
        }
        let ts = services.iter().position(|s| *s == o.service).map_or(0, |i| i + 1).to_string();
        let st = states.iter().position(|s| *s == o.state).map_or(0, |i| i + 1).to_string();
        insert(
            src,
            "sirius_netmap",
            &[
                ("egon_civico", &c.egon),
                ("egon_strada", &c.egon_strada),
                ("egon_comune", "092009"),
                ("egon_provincia", "092"),
                ("egon_regione", "20"),
                ("civico", &c.civico),
                ("strada", &c.street),
                ("comune", &c.comune),
                ("provincia", &c.provincia),
                ("regione", "SARDEGNA"),
                ("id_tipo_servizio", &ts),
                ("id_stato", &st),
                ("data_stato_corrente", &o.since),
                ("note", &o.note),
            ],
        )?;
    }
    for (egon_strada, poly) in &m.polylines {
        let street = m.civici.iter().find(|c| &c.egon_strada == egon_strada).map(|c| c.street.clone()).unwrap_or_default();
        insert(src, "sirius_egon_polylines", &[("egon_strada", egon_strada), ("egon_comune", "092009"), ("strada", &street), ("polylines", poly)])?;
    }
    for b in &m.bridges {
        let id = b.id.to_string();
        insert(
            src,
            "radio_table",
            &[
                ("id_ponte", &id),
                ("nome", &b.name),
                ("lat", &b.lat),
                ("lon", &b.lon),
                ("alt", &b.alt),
                ("stato", &b.state),
                ("descrizione", &b.description),
                ("fascia", &b.band),
            ],
        )?;
    }
    for c in &m.cells {
        insert(
            src,
            "cell_db",
            &[
                ("radio", &c.radio),
                ("mcc", &c.mcc),
                ("net", &c.net),
                ("area", &c.area),
                ("cell", &c.cell),
                ("lat", &c.lat),
                ("lon", &c.lon),
                ("range", &c.range),
                ("samples", &c.samples),
            ],
        )?;
    }
    for mnc in ["1", "10", "88", "50"] {
        insert(src, "cell_mnc", &[("value", mnc)])?;
    }
    src.ping()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use covercraft_core::config::Preferences;
    use covercraft_core::model::Provider;
    use covercraft_core::status::Status;

    #[test]
    fn the_schema_converts_and_the_queries_run_on_it() {
        let ddl = sqlite_schema();
        assert!(ddl.len() >= 20, "{}", ddl.len());
        assert!(ddl.iter().any(|s| s.contains("`requests_log`") && s.contains("INTEGER PRIMARY KEY AUTOINCREMENT")));
        assert!(ddl.iter().all(|s| !s.contains("ENGINE") && !s.contains(" KEY `")));
        let mut src = crate::demo_sqlite().unwrap();
        assert!(src.ping().unwrap().contains("5 comuni"), "{}", src.ping().unwrap());
        let comuni = src.comuni().unwrap();
        assert_eq!(comuni.len(), 5);
        let cagliari = comuni.iter().find(|c| c.name == "CAGLIARI").cloned().unwrap();
        assert_eq!(cagliari.istat, "092009");
        assert_eq!((cagliari.regione.as_str(), cagliari.provincia_name.as_str()), ("SARDEGNA", "CAGLIARI"));
        let streets = src.streets(&cagliari, "via rom", 20).unwrap();
        assert_eq!(streets.len(), 1);
        assert_eq!(streets[0].display(), "VIA ROMA");
        let civici = src.civici(&cagliari, &streets[0], 20).unwrap();
        assert_eq!(civici.iter().map(|c| c.civico.as_str()).collect::<Vec<_>>(), ["10", "12", "14", "20/A"]);
        let found = src.resolve(&cagliari, "Via Roma", "20/a", 20).unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].egon, "100020");
        let all = src.resolve(&cagliari, "roma", "", 20).unwrap();
        assert_eq!(all.len(), 4);
        assert_eq!(src.civico(Some(&cagliari), "100105").unwrap().unwrap().street, "DANTE");
        assert_eq!(src.civico(None, "100010").unwrap().unwrap().source, "openfiber");
        assert!(src.civico(None, "nope").unwrap().is_none());
        assert!(src.civico(None, "").is_err());
        // A comune without a TIM table falls back to the other tables.
        let nuoro = comuni.iter().find(|c| c.table.is_empty()).cloned().unwrap();
        assert!(src.streets(&nuoro, "x", 5).unwrap().is_empty());
        let prefs = Preferences::default();
        let roma10 = src.civico(Some(&cagliari), "100010").unwrap().unwrap();
        let r = covercraft_core::report::gather(&mut src, &cagliari, &roma10, &prefs).unwrap();
        assert_eq!(r.best().map(|s| (s.technology.as_str(), s.status)), Some(("FTTH XGS-PON", Status::Covered)));
        assert_eq!(r.openfiber.len(), 2);
        assert_eq!(r.point_source, "openfiber");
        assert!(r.of(Provider::Tim).any(|s| s.technology == "FTTC (VULA)" && s.distance_m == Some(320)));
        assert_eq!(r.of(Provider::Fastweb).count(), 1);
        assert!(r.radio.iter().any(|h| h.in_reach && h.bridge.name == "Monte Urpinu"));
        assert_eq!(r.cells.len(), 4, "{:?}", r.cells);
        assert!(r.info.iter().any(|(k, v)| k == "Unità immobiliari" && v == "8"));
        let marconi = src.civico(Some(&cagliari), "100300").unwrap().unwrap();
        let rm = covercraft_core::report::gather(&mut src, &cagliari, &marconi, &prefs).unwrap();
        assert_eq!(rm.point_source, "polyline");
        assert!(rm.of(Provider::Own).any(|s| s.technology == "Fibra dedicata"));
        assert!(rm.of(Provider::Fastweb).any(|s| s.technology == "GPON (indirizzo)" && s.status == Status::Planned));
        src.log("anna", &serde_json::json!({"egon": "100010"}), &serde_json::json!({"best": "FTTH"})).unwrap();
        src.log("luca", &serde_json::json!({"egon": "100012"}), &serde_json::json!({})).unwrap();
        let h = src.history("", 10).unwrap();
        assert_eq!(h.len(), 2);
        assert_eq!((h[0].user.as_str(), h[0].request["egon"].as_str()), ("luca", Some("100012")));
        assert_eq!(src.history("anna", 10).unwrap().len(), 1);
        assert!(!h[0].date.is_empty());
    }

    #[test]
    fn open_follows_the_configuration() {
        let demo = covercraft_core::config::DbConfig::default();
        assert!(crate::open(&demo, "").is_ok());
        let bad = covercraft_core::config::DbConfig { kind: "sqlite".into(), path: "/nonexistent/dir/x.db".into(), ..Default::default() };
        assert!(crate::open(&bad, "").is_err());
        let mysql = covercraft_core::config::DbConfig { kind: "mysql".into(), host: "127.0.0.1".into(), port: 1, ..Default::default() };
        assert!(crate::open(&mysql, "pw").is_err());
    }
}
