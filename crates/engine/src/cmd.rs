//! The command table: every action of the interface, the CLI and the MCP server. Local
//! commands answer at once; the others queue a job on the worker and answer `{"job": id}`.

use covercraft_core::address;
use covercraft_core::config::DbConfig;
use serde_json::{Value, json};

use crate::{Error, JobKind, Result, Session};

pub struct Command {
    pub id: &'static str,
    pub label: &'static str,
    /// The parameters, for `--help` and the MCP tool list.
    pub params: &'static str,
    /// Answers at once (no job).
    pub local: bool,
    pub run: fn(&mut Session, &Value) -> Result<Value>,
}

fn s(v: &Value, key: &str) -> String {
    match v.get(key) {
        Some(Value::String(t)) => covercraft_core::limits::clip(t.trim()),
        Some(Value::Number(n)) => n.to_string(),
        Some(Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

fn limit(v: &Value, default: usize) -> usize {
    v.get("limit")
        .and_then(|l| l.as_u64().or_else(|| l.as_str().and_then(|t| t.parse().ok())))
        .map_or(default, |l| usize::try_from(l).unwrap_or(default))
        .clamp(1, covercraft_core::limits::ROWS)
}

fn db_get(s: &mut Session, _: &Value) -> Result<Value> {
    let d = &s.config.db;
    Ok(json!({
        "kind": d.kind, "host": d.host, "port": d.port, "user": d.user, "database": d.database, "path": d.path,
        "display": d.display(), "has_password": s.has_password(), "state": s.state.db_state, "message": s.state.db_message,
    }))
}

fn db_configure(sess: &mut Session, v: &Value) -> Result<Value> {
    let mut db: DbConfig = sess.config.db.clone();
    db.apply(v)?;
    if let Some(p) = v.get("password").and_then(Value::as_str) {
        sess.secrets.db_password = covercraft_core::limits::clip(p);
    }
    sess.config.db = db;
    sess.save()?;
    sess.touch();
    let job = sess.reconnect()?;
    Ok(json!({"job": job}))
}

fn db_test(sess: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"job": sess.submit("db.test", JobKind::Ping)?}))
}

fn prefs_get(sess: &mut Session, _: &Value) -> Result<Value> {
    serde_json::to_value(&sess.config.preferences).map_err(|e| Error::Param(e.to_string()))
}

fn prefs_set(sess: &mut Session, v: &Value) -> Result<Value> {
    let mut p = sess.config.preferences.clone();
    p.apply(v)?;
    sess.config.preferences = p;
    sess.save()?;
    sess.touch();
    prefs_get(sess, v)
}

fn comune_list(sess: &mut Session, v: &Value) -> Result<Value> {
    let query = address::normalize(&s(v, "query"));
    let regione = address::normalize(&s(v, "regione"));
    let provincia = address::normalize(&s(v, "provincia"));
    if !sess.state.comuni.is_empty() && !v.get("refresh").and_then(Value::as_bool).unwrap_or(false) {
        let within = sess
            .state
            .comuni
            .iter()
            .filter(|c| regione.is_empty() || c.regione == regione)
            .filter(|c| provincia.is_empty() || c.provincia == provincia || c.provincia_name == provincia);
        if v.get("regioni").and_then(Value::as_bool).unwrap_or(false) {
            let mut regioni: Vec<String> = sess.state.comuni.iter().map(|c| c.regione.clone()).filter(|r| !r.is_empty()).collect();
            regioni.sort();
            regioni.dedup();
            return Ok(json!({"regioni": regioni}));
        }
        if v.get("province").and_then(Value::as_bool).unwrap_or(false) {
            let mut province: Vec<(String, String)> = within.map(|c| (c.provincia.clone(), c.provincia_name.clone())).collect();
            province.sort();
            province.dedup();
            let list: Vec<Value> = province.iter().map(|(sigla, name)| json!({"sigla": sigla, "name": name})).collect();
            return Ok(json!({"province": list}));
        }
        let list: Vec<Value> = within
            .filter(|c| query.is_empty() || c.name.contains(&query) || c.provincia == query)
            .take(limit(v, 50))
            .map(crate::comune_json)
            .collect();
        return Ok(json!({"comuni": list}));
    }
    Ok(json!({"job": sess.submit("comune.list", JobKind::Comuni)?}))
}

fn street_search(sess: &mut Session, v: &Value) -> Result<Value> {
    let comune = s(v, "comune");
    if comune.is_empty() {
        return Err(Error::Param("`comune` is required".into()));
    }
    Ok(
        json!({"job": sess.submit("street.search", JobKind::Streets { comune, query: s(v, "query"), limit: limit(v, covercraft_core::limits::QUERY_ROWS) })?}),
    )
}

fn civico_list(sess: &mut Session, v: &Value) -> Result<Value> {
    let comune = s(v, "comune");
    let street_egon = s(v, "street_egon");
    if comune.is_empty() || street_egon.is_empty() {
        return Err(Error::Param("`comune` and `street_egon` are required".into()));
    }
    Ok(json!({"job": sess.submit("civico.list", JobKind::Civici { comune, street_egon, limit: limit(v, covercraft_core::limits::QUERY_ROWS) })?}))
}

fn address_resolve(sess: &mut Session, v: &Value) -> Result<Value> {
    let comune = s(v, "comune");
    let via = s(v, "via");
    if comune.is_empty() || via.is_empty() {
        return Err(Error::Param("`comune` and `via` are required".into()));
    }
    Ok(
        json!({"job": sess.submit("address.resolve", JobKind::Resolve { comune, via, civico: s(v, "civico"), limit: limit(v, covercraft_core::limits::QUERY_ROWS) })?}),
    )
}

fn coverage_verify(sess: &mut Session, v: &Value) -> Result<Value> {
    let (comune, via, civico, egon) = (s(v, "comune"), s(v, "via"), s(v, "civico"), s(v, "egon"));
    if egon.is_empty() && (comune.is_empty() || via.is_empty()) {
        return Err(Error::Param("give `comune` and `via` (and `civico`), or `egon`".into()));
    }
    let user = {
        let u = s(v, "user");
        if u.is_empty() { sess.user.clone() } else { u }
    };
    // A point given by hand (`lat`, `lon`), else one remembered for the EGON code.
    let num = |k: &str| {
        v.get(k).map(|x| {
            x.as_f64()
                .or_else(|| x.as_str().and_then(|t| t.trim().replace(',', ".").parse().ok()))
                .ok_or_else(|| Error::Param(format!("`{k}` must be a number")))
        })
    };
    let given = match (num("lat"), num("lon")) {
        (Some(lat), Some(lon)) => {
            let p = covercraft_core::geo::Point::new(lat?, lon?).ok_or_else(|| Error::Param("`lat`/`lon` out of range".into()))?;
            Some((p, "manual".to_string()))
        }
        (None, None) => {
            if egon.is_empty() {
                None
            } else {
                sess.cached_point(&egon)
            }
        }
        _ => return Err(Error::Param("give both `lat` and `lon`".into())),
    };
    let request = json!({"comune": comune, "via": via, "civico": civico, "egon": egon, "user": user});
    // The database log is opt-in (the verification user is often read-only); the local
    // history is kept by the session when the report lands.
    let log = sess.config.preferences.log_to_database && !v.get("log").is_some_and(|l| l.as_bool() == Some(false));
    let kind =
        JobKind::Verify(Box::new(crate::VerifyJob { comune, via, civico, egon, user, prefs: sess.config.preferences.clone(), log, request, given }));
    Ok(json!({"job": sess.submit("coverage.verify", kind)?}))
}

fn radio_list(sess: &mut Session, _: &Value) -> Result<Value> {
    Ok(json!({"job": sess.submit("radio.list", JobKind::Bridges)?}))
}

fn cell_find(sess: &mut Session, v: &Value) -> Result<Value> {
    let query = covercraft_core::cells::CellQuery::from_params(v)?;
    let token = sess.config.preferences.opencellid_token.clone();
    Ok(json!({"job": sess.submit("cell.find", JobKind::CellFind { query, token })?}))
}

fn history_list(sess: &mut Session, v: &Value) -> Result<Value> {
    // "Mine" is the session's user unless a `user` is named; `all` lists everyone's.
    let user = if v.get("all").and_then(Value::as_bool).unwrap_or(false) {
        String::new()
    } else {
        let named = s(v, "user");
        if named.is_empty() { sess.user.clone() } else { named }
    };
    sess.state.history = sess.history(&user, limit(v, 200));
    sess.touch();
    Ok(json!({"history": sess.state.history.iter().map(crate::history_json).collect::<Vec<_>>()}))
}

fn history_open(sess: &mut Session, v: &Value) -> Result<Value> {
    let id = v
        .get("id")
        .and_then(|i| i.as_i64().or_else(|| i.as_str().and_then(|t| t.parse().ok())))
        .ok_or_else(|| Error::Param("`id` is required".into()))?;
    let entry = sess
        .state
        .history
        .iter()
        .find(|h| h.id == id)
        .cloned()
        .ok_or_else(|| Error::Param(format!("no history entry {id} loaded: call history.list first")))?;
    let report = crate::report_from_json(&entry.answer).ok_or_else(|| Error::Param(format!("history entry {id} has no report")))?;
    let v = crate::report_json(&report);
    sess.state.report = Some(report);
    sess.state.civici.clear();
    sess.touch();
    Ok(v)
}

fn session_user(sess: &mut Session, v: &Value) -> Result<Value> {
    sess.set_user(&s(v, "user"));
    sess.touch();
    Ok(json!({"user": sess.user}))
}

fn state_get(sess: &mut Session, _: &Value) -> Result<Value> {
    let st = &sess.state;
    Ok(json!({
        "db_state": st.db_state, "db_message": st.db_message, "database": sess.config.db.display(),
        "comuni": st.comuni.len(), "report": st.report.as_ref().map(|r| r.civico.display()),
        "jobs_pending": sess.jobs_pending(), "user": sess.user, "last_error": st.last_error,
    }))
}

fn report_get(sess: &mut Session, _: &Value) -> Result<Value> {
    sess.state.report.as_ref().map(crate::report_json).ok_or_else(|| Error::Param("no report yet: run coverage.verify".into()))
}

pub const COMMANDS: &[Command] = &[
    Command { id: "db.get", label: "Database settings", params: "", local: true, run: db_get },
    Command {
        id: "db.configure",
        label: "Configure the database",
        params: "kind (mysql|sqlite|demo), host, port, user, password, database, path",
        local: false,
        run: db_configure,
    },
    Command { id: "db.test", label: "Test the database connection", params: "", local: false, run: db_test },
    Command { id: "prefs.get", label: "Preferences", params: "", local: true, run: prefs_get },
    Command {
        id: "prefs.set",
        label: "Change preferences",
        params: "language, theme, log_requests, log_to_database, geocode_url, opencellid_token, tiles_url, radio {frequency_ghz, channel_mhz, tx_power_dbm, bridge_gain_dbi, client_gain_dbi, cable_loss_db, fade_margin_db, min_rx_dbm, client_height_m}, radio_reach_km {A..D}, max_bridges, cells, cells_radius_km, max_cells, rules [{value, status}]",
        local: true,
        run: prefs_set,
    },
    Command {
        id: "comune.list",
        label: "Comuni",
        params: "query, regione, provincia, limit, refresh; regioni=true lists regions, province=true the provinces",
        local: true,
        run: comune_list,
    },
    Command { id: "street.search", label: "Search streets", params: "comune, query, limit", local: false, run: street_search },
    Command { id: "civico.list", label: "Civici of a street", params: "comune, street_egon, limit", local: false, run: civico_list },
    Command {
        id: "address.resolve",
        label: "Resolve an address to EGON civici",
        params: "comune, via, civico, limit",
        local: false,
        run: address_resolve,
    },
    Command {
        id: "coverage.verify",
        label: "Verify coverage",
        params: "comune, via, civico | egon; lat, lon (a point set by hand); user, log",
        local: false,
        run: coverage_verify,
    },
    Command { id: "report.get", label: "The last report", params: "", local: true, run: report_get },
    Command { id: "radio.list", label: "Radio bridges", params: "", local: false, run: radio_list },
    Command {
        id: "cell.find",
        label: "Find a mobile cell by its identifiers",
        params: "mcc (default 222), net (MNC), area (LAC/TAC; optional for cell_db, needed for OpenCelliD), cell (CID/ECI) | enb + sector, radio (GSM|UMTS|LTE|NR)",
        local: false,
        run: cell_find,
    },
    Command {
        id: "history.list",
        label: "Past verifications (local history)",
        params: "user (default: the session's), all, limit",
        local: true,
        run: history_list,
    },
    Command { id: "history.open", label: "Open a past verification", params: "id", local: true, run: history_open },
    Command { id: "session.user", label: "Who is verifying (for the log)", params: "user", local: true, run: session_user },
    Command { id: "state.get", label: "Session state", params: "", local: true, run: state_get },
];
