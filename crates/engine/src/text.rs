//! The report in words: the one-line summary and the full text for the CLI, MCP and the log.

use covercraft_core::model::{Provider, Report, Service};
use covercraft_core::status::Status;

fn speed(s: &Service) -> String {
    match (s.speed_down.is_empty(), s.speed_up.is_empty()) {
        (true, true) => String::new(),
        (false, true) => format!(" {}", s.speed_down),
        (true, false) => format!(" ↑{}", s.speed_up),
        (false, false) => format!(" {}/{}", s.speed_down, s.speed_up),
    }
}

/// One line: the best option, or why there is none.
pub fn summary(r: &Report, italian: bool) -> String {
    let who = r.civico.display();
    match r.best() {
        Some(b) if b.status == Status::Covered => {
            let prov = b.provider.map(Provider::label).unwrap_or_default();
            if italian {
                format!("{who}: Coperto da {} {}{}", prov, b.technology, speed(b))
            } else {
                format!("{who}: Covered by {} {}{}", prov, b.technology, speed(b))
            }
        }
        Some(b) if b.status == Status::Planned => {
            let prov = b.provider.map(Provider::label).unwrap_or_default();
            let when = if b.planned.is_empty() { String::new() } else { format!(" ({})", b.planned) };
            if italian {
                format!("{who}: Pianificato {} {}{when}", prov, b.technology)
            } else {
                format!("{who}: Planned {} {}{when}", prov, b.technology)
            }
        }
        _ => {
            if let Some(h) = r.best_radio() {
                let km = h.distance_m / 1000.0;
                return if italian {
                    format!("{who}: Nessuna fibra o rame; ponte radio {} a {km:.1} km ({})", h.bridge.name, h.compass)
                } else {
                    format!("{who}: No wired service; radio bridge {} at {km:.1} km ({})", h.bridge.name, h.compass)
                };
            }
            if italian { format!("{who}: Non coperto") } else { format!("{who}: Not covered") }
        }
    }
}

fn light(s: Status) -> &'static str {
    match s {
        Status::Covered => "●",
        Status::Planned => "◐",
        Status::NotCovered => "○",
        Status::Unknown => "?",
    }
}

/// The full report as text.
pub fn report_text(r: &Report, italian: bool) -> String {
    let mut out = String::new();
    out.push_str(&summary(r, italian));
    out.push('\n');
    out.push_str(&format!("EGON {}", r.civico.egon));
    if let Some(p) = r.point {
        out.push_str(&format!("  ·  {:.5}, {:.5} ({})", p.lat, p.lon, r.point_source));
    }
    out.push('\n');
    for p in Provider::ALL {
        let services: Vec<&Service> = r.of(p).collect();
        if services.is_empty() {
            continue;
        }
        out.push_str(&format!("\n{}\n", p.label()));
        for s in services {
            let mut line = format!("  {} {:<26} {:<12}", light(s.status), s.technology, s.status.label(italian));
            if !s.raw.is_empty() && s.raw.to_uppercase() != s.status.label(italian).to_uppercase() {
                line.push_str(&format!(" [{}]", s.raw));
            }
            line.push_str(&speed(s));
            if let Some(d) = s.distance_m {
                line.push_str(&format!("  {d} m"));
            }
            if !s.planned.is_empty() {
                line.push_str(&format!("  {}: {}", if italian { "previsto" } else { "planned" }, s.planned));
            }
            out.push_str(&line);
            out.push('\n');
            if !s.details.is_empty() {
                let d: Vec<String> = s.details.iter().map(|(k, v)| format!("{k}: {v}")).collect();
                out.push_str(&format!("      {}\n", d.join(" · ")));
            }
        }
    }
    if !r.info.is_empty() {
        let d: Vec<String> = r.info.iter().map(|(k, v)| format!("{k}: {v}")).collect();
        out.push_str(&format!("\n{}: {}\n", if italian { "Civico" } else { "Civico facts" }, d.join(" · ")));
    }
    if !r.openfiber.is_empty() {
        out.push_str(&format!("\nOpen Fiber: {} {}\n", r.openfiber.len(), if italian { "scale" } else { "staircases" }));
        for s in &r.openfiber {
            out.push_str(&format!(
                "  {} {:<14} {:<10} {} / {}{}{}\n",
                light(s.scala_status.max(s.building_status)),
                s.row.id_scala,
                s.row.scala,
                s.building_text,
                s.scala_text,
                if s.row.rfa_actual.is_empty() { String::new() } else { format!("  RFA {}", s.row.rfa_actual) },
                if s.row.rfc_planned.is_empty() { String::new() } else { format!("  RFC {}", s.row.rfc_planned) },
            ));
        }
    }
    if !r.radio.is_empty() {
        out.push_str(&format!("\n{}\n", if italian { "Ponti radio" } else { "Radio bridges" }));
        for h in &r.radio {
            out.push_str(&format!(
                "  {} {:<22} {:>6.1} km  {:>3}° {:<2}  {} {}{}\n",
                if h.in_reach { "●" } else { "○" },
                h.bridge.name,
                h.distance_m / 1000.0,
                h.bearing_deg.round() as i64,
                h.compass,
                if italian { "fascia" } else { "band" },
                h.bridge.band,
                if h.bridge.state.is_empty() { String::new() } else { format!("  [{}]", h.bridge.state) },
            ));
        }
    }
    if !r.cells.is_empty() {
        out.push_str(&format!("\n{}\n", if italian { "Celle mobili" } else { "Mobile cells" }));
        for c in &r.cells {
            out.push_str(&format!(
                "  {} {:<12} {:<5} {:>5.0} m{}\n",
                if c.in_range { "●" } else { "○" },
                c.operator,
                c.cell.radio,
                c.distance_m,
                if c.cell.range.is_empty() { String::new() } else { format!("  (range {} m)", c.cell.range) }
            ));
        }
    }
    if !r.notes.is_empty() {
        let notes: Vec<String> = r.notes.iter().map(|n| note_text(n, italian)).collect();
        out.push_str(&format!("\n{}\n", notes.join("\n")));
    }
    out
}

/// A note of the report in words.
pub fn note_text(note: &str, italian: bool) -> String {
    match (note, italian) {
        ("tim_missing", true) => "Nessuna riga TIM/FiberCop per questo civico".into(),
        ("tim_missing", false) => "No TIM/FiberCop row for this civico".into(),
        ("openfiber_missing", true) => "Nessuna riga Open Fiber per questo civico".into(),
        ("openfiber_missing", false) => "No Open Fiber row for this civico".into(),
        ("fastweb_missing", true) => "Nessuna riga Fastweb per questo civico".into(),
        ("fastweb_missing", false) => "No Fastweb row for this civico".into(),
        ("no_point", true) => "Coordinate non note: ponti radio e celle non calcolati".into(),
        ("no_point", false) => "No coordinates: radio bridges and cells not computed".into(),
        (other, _) => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use covercraft_core::config::Preferences;
    use covercraft_core::mock::MockSource;
    use covercraft_core::source::CoverageSource;

    #[test]
    fn words_for_every_case() {
        let mut m = MockSource::demo();
        let cagliari = m.comuni().unwrap()[0].clone();
        let prefs = Preferences::default();
        let c = m.civico(None, "100010").unwrap().unwrap();
        let r = covercraft_core::report::gather(&mut m, &cagliari, &c, &prefs).unwrap();
        assert!(summary(&r, true).starts_with("VIA ROMA 10, CAGLIARI: Coperto da Open Fiber FTTH XGS-PON"), "{}", summary(&r, true));
        let t = report_text(&r, false);
        assert!(t.contains("TIM / FiberCop") && t.contains("Radio bridges") && t.contains("Mobile cells") && t.contains("Monte Urpinu"));
        let c = m.civico(None, "100014").unwrap().unwrap();
        let r = covercraft_core::report::gather(&mut m, &cagliari, &c, &prefs).unwrap();
        assert!(summary(&r, false).contains("Covered by TIM / FiberCop ADSL"), "{}", summary(&r, false));
        assert!(report_text(&r, true).contains("Rete propria") && report_text(&r, true).contains("Radio 5 GHz"));
        let r2 = Report::default();
        assert!(summary(&r2, true).ends_with("Non coperto"));
        assert!(summary(&r2, false).ends_with("Not covered"));
        assert_eq!(note_text("no_point", true), "Coordinate non note: ponti radio e celle non calcolati");
    }
}
