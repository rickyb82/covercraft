//! The configuration: the database to query, preferences, the password kept apart.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::status::Rule;
use crate::{Error, Result};

/// Where the coverage tables are.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct DbConfig {
    /// `mysql` (the netmap database), `sqlite` (a file with the same schema), `demo` (in memory).
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub database: String,
    /// The SQLite file (`kind = sqlite`).
    pub path: String,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self { kind: "demo".into(), host: "127.0.0.1".into(), port: 3306, user: "netmap".into(), database: "netmap".into(), path: String::new() }
    }
}

impl DbConfig {
    pub fn is_demo(&self) -> bool {
        self.kind == "demo" || self.kind.is_empty()
    }

    /// `user@host:port/database`, never the password.
    pub fn display(&self) -> String {
        match self.kind.as_str() {
            "mysql" => format!("{}@{}:{}/{}", self.user, self.host, self.port, self.database),
            "sqlite" => format!("sqlite:{}", self.path),
            _ => "demo".into(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        match self.kind.as_str() {
            "mysql" => {
                if self.host.trim().is_empty() {
                    return Err(Error::Invalid("host".into()));
                }
                if self.database.trim().is_empty() {
                    return Err(Error::Invalid("database".into()));
                }
                if self.port == 0 {
                    return Err(Error::Invalid("port".into()));
                }
                Ok(())
            }
            "sqlite" => {
                if self.path.trim().is_empty() {
                    return Err(Error::Invalid("path".into()));
                }
                Ok(())
            }
            "demo" | "" => Ok(()),
            other => Err(Error::Invalid(format!("database kind `{other}` (mysql, sqlite, demo)"))),
        }
    }

    /// Applies the fields of a JSON object (only those given).
    pub fn apply(&mut self, v: &Value) -> Result<()> {
        if let Some(k) = v.get("kind").and_then(Value::as_str) {
            self.kind = k.trim().to_ascii_lowercase();
        }
        if let Some(h) = v.get("host").and_then(Value::as_str) {
            self.host = crate::limits::clip(h.trim());
        }
        if let Some(p) = v.get("port") {
            let p = p.as_u64().or_else(|| p.as_str().and_then(|s| s.trim().parse().ok())).ok_or_else(|| Error::Invalid("port".into()))?;
            self.port = u16::try_from(p).map_err(|_| Error::Invalid("port".into()))?;
        }
        if let Some(u) = v.get("user").and_then(Value::as_str) {
            self.user = crate::limits::clip(u.trim());
        }
        if let Some(d) = v.get("database").and_then(Value::as_str) {
            self.database = crate::limits::clip(d.trim());
        }
        if let Some(p) = v.get("path").and_then(Value::as_str) {
            self.path = crate::limits::clip(p.trim());
        }
        self.validate()
    }
}

/// Interface and analysis preferences.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Preferences {
    /// `en`, `it` or empty (the system's).
    pub language: String,
    /// `light`, `dark` or empty (the system's).
    pub theme: String,
    /// Keep a local history of verifications (a file next to the configuration).
    pub log_requests: bool,
    /// Also write verifications to the database's `requests_log` (needs INSERT rights).
    pub log_to_database: bool,
    /// Reach of a radio bridge by band (`fascia`), km.
    pub radio_reach_km: BTreeMap<String, f64>,
    /// Bridges listed in a report (the ones in reach always are).
    pub max_bridges: usize,
    /// Mobile cells looked up around the address.
    pub cells: bool,
    pub cells_radius_km: f64,
    pub max_cells: usize,
    /// Extra wording rules for coverage cells.
    pub rules: Vec<Rule>,
    /// The radio equipment, for the link estimate.
    pub radio: crate::radio::RadioParams,
    /// A Nominatim server (`http://nominatim:8080`) used when the tables give no point; empty = off.
    pub geocode_url: String,
    /// OpenCelliD API token: live cells around the point join the table's (empty = off).
    pub opencellid_token: String,
    /// Map tiles (`https://tile.openstreetmap.org/{z}/{x}/{y}.png` style); empty = OpenStreetMap.
    pub tiles_url: String,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: String::new(),
            theme: String::new(),
            log_requests: true,
            log_to_database: false,
            radio_reach_km: [("A", 20.0), ("B", 12.0), ("C", 7.0), ("D", 4.0)].into_iter().map(|(k, v)| (k.to_string(), v)).collect(),
            max_bridges: 8,
            cells: true,
            cells_radius_km: 3.0,
            max_cells: 12,
            rules: Vec::new(),
            radio: crate::radio::RadioParams::default(),
            geocode_url: String::new(),
            opencellid_token: String::new(),
            tiles_url: String::new(),
        }
    }
}

impl Preferences {
    /// Reach of a band in metres (unknown band: the smallest configured).
    pub fn reach_m(&self, band: &str) -> f64 {
        let b = band.trim().to_ascii_uppercase();
        let km = self.radio_reach_km.get(&b).copied().or_else(|| self.radio_reach_km.values().copied().reduce(f64::min)).unwrap_or(4.0);
        km.clamp(0.0, 500.0) * 1000.0
    }

    /// Applies the fields of a JSON object (only those given).
    pub fn apply(&mut self, v: &Value) -> Result<()> {
        let Some(o) = v.as_object() else { return Err(Error::Invalid("preferences (object expected)".into())) };
        for (k, val) in o {
            match k.as_str() {
                "language" | "lingua" => {
                    let l = val.as_str().unwrap_or_default().trim().to_ascii_lowercase();
                    self.language = if l == "system" || l == "auto" { String::new() } else { l.chars().take(5).collect() };
                }
                "theme" | "tema" => {
                    let t = val.as_str().unwrap_or_default().trim().to_ascii_lowercase();
                    self.theme = match t.as_str() {
                        "light" | "dark" => t,
                        _ => String::new(),
                    };
                }
                "log_requests" => self.log_requests = as_bool(val)?,
                "log_to_database" => self.log_to_database = as_bool(val)?,
                "cells" => self.cells = as_bool(val)?,
                "max_bridges" => self.max_bridges = as_count(val, 200)?,
                "max_cells" => self.max_cells = as_count(val, 500)?,
                "cells_radius_km" => self.cells_radius_km = as_f64(val, 50.0)?,
                "radio_reach_km" => {
                    let Some(m) = val.as_object() else { return Err(Error::Invalid("radio_reach_km (object band → km)".into())) };
                    for (band, km) in m {
                        let band = band.trim().to_ascii_uppercase();
                        if band.is_empty() || band.len() > 2 {
                            return Err(Error::Invalid(format!("band `{band}`")));
                        }
                        self.radio_reach_km.insert(band, as_f64(km, 500.0)?);
                    }
                }
                "radio" => self.radio.apply(val)?,
                "geocode_url" => self.geocode_url = crate::limits::clip(val.as_str().unwrap_or_default().trim().trim_end_matches('/')),
                "opencellid_token" => self.opencellid_token = crate::limits::clip(val.as_str().unwrap_or_default().trim()),
                "tiles_url" => self.tiles_url = crate::limits::clip(val.as_str().unwrap_or_default().trim()),
                "rules" => {
                    let Some(list) = val.as_array() else { return Err(Error::Invalid("rules (array of {value, status})".into())) };
                    let mut rules = Vec::new();
                    for r in list.iter().take(200) {
                        let value = crate::limits::clip(r.get("value").and_then(Value::as_str).unwrap_or_default().trim());
                        let status = match r.get("status").and_then(Value::as_str).unwrap_or_default() {
                            "covered" => crate::status::Status::Covered,
                            "planned" => crate::status::Status::Planned,
                            "not_covered" => crate::status::Status::NotCovered,
                            "unknown" => crate::status::Status::Unknown,
                            other => return Err(Error::Invalid(format!("rule status `{other}`"))),
                        };
                        if !value.is_empty() {
                            rules.push(Rule { value, status });
                        }
                    }
                    self.rules = rules;
                }
                other => return Err(Error::Invalid(format!("preference `{other}`"))),
            }
        }
        Ok(())
    }
}

fn as_bool(v: &Value) -> Result<bool> {
    v.as_bool()
        .or_else(|| v.as_str().map(|s| matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes" | "si" | "on")))
        .ok_or_else(|| Error::Invalid("boolean".into()))
}

fn as_count(v: &Value, max: usize) -> Result<usize> {
    let n = v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok())).ok_or_else(|| Error::Invalid("count".into()))?;
    usize::try_from(n).ok().filter(|n| *n <= max).ok_or_else(|| Error::Invalid(format!("count (max {max})")))
}

fn as_f64(v: &Value, max: f64) -> Result<f64> {
    let n = v.as_f64().or_else(|| v.as_str().and_then(|s| s.trim().replace(',', ".").parse().ok())).ok_or_else(|| Error::Invalid("number".into()))?;
    (n.is_finite() && (0.0..=max).contains(&n)).then_some(n).ok_or_else(|| Error::Invalid(format!("number (0..{max})")))
}

/// Everything saved in `config.json`.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Config {
    pub db: DbConfig,
    pub preferences: Preferences,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| Error::Invalid(format!("{}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::Io(format!("{}: {e}", path.display()))),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| Error::Invalid(e.to_string()))?;
        std::fs::write(path, text)?;
        Ok(())
    }
}

/// The database password, in a user-only file.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Secrets {
    pub db_password: String,
}

impl Secrets {
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).map_err(|e| Error::Invalid(format!("{}: {e}", path.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(Error::Io(format!("{}: {e}", path.display()))),
        }
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let text = serde_json::to_string_pretty(self).map_err(|e| Error::Invalid(e.to_string()))?;
        write_private(path, text.as_bytes())
    }
}

/// Writes a file readable by its owner only.
pub fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
        f.write_all(bytes)?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, bytes)?;
        Ok(())
    }
}

/// Where the files live.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paths {
    pub config: PathBuf,
    pub secrets: PathBuf,
}

impl Paths {
    pub fn under(dir: &Path) -> Self {
        Self { config: dir.join("config.json"), secrets: dir.join("secrets.json") }
    }

    /// `$COVERCRAFT_HOME`, else the platform's configuration directory.
    pub fn default_dirs() -> Self {
        if let Some(h) = std::env::var_os("COVERCRAFT_HOME") {
            return Self::under(Path::new(&h));
        }
        let base = if cfg!(windows) {
            std::env::var_os("APPDATA").map(PathBuf::from)
        } else if cfg!(target_os = "macos") {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library/Application Support"))
        } else {
            std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        };
        Self::under(&base.unwrap_or_else(|| PathBuf::from(".")).join("covercraft"))
    }
}

/// A lower-case file-name-safe id for a name (users, history files).
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in name.trim().chars().take(64) {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn config_round_trips_and_validates() {
        let dir = std::env::temp_dir().join(format!("covercraft-test-{}", std::process::id()));
        let paths = Paths::under(&dir);
        let mut c = Config::default();
        assert!(c.db.is_demo());
        c.db.apply(&json!({"kind": "mysql", "host": "192.168.10.24", "port": "3306", "user": "u", "database": "netmap"})).unwrap();
        assert_eq!(c.db.display(), "u@192.168.10.24:3306/netmap");
        assert!(c.db.apply(&json!({"kind": "oracle"})).is_err());
        c.db.kind = "mysql".into();
        assert!(c.db.apply(&json!({"port": 70000})).is_err());
        c.preferences.apply(&json!({"language": "it", "theme": "dark", "radio_reach_km": {"a": 25}, "max_bridges": 5, "rules": [{"value": "X", "status": "covered"}], "radio": {"tx_power_dbm": 20}, "geocode_url": "http://nominatim:8080/", "opencellid_token": " pk.abc "})).unwrap();
        assert_eq!(c.preferences.radio.tx_power_dbm, 20.0);
        assert_eq!(c.preferences.geocode_url, "http://nominatim:8080");
        assert_eq!(c.preferences.opencellid_token, "pk.abc");
        assert_eq!(c.preferences.reach_m("a"), 25_000.0);
        assert_eq!(c.preferences.reach_m("Z"), 4_000.0);
        assert!(c.preferences.apply(&json!({"nope": 1})).is_err());
        assert!(c.preferences.apply(&json!({"max_bridges": 1000})).is_err());
        c.save(&paths.config).unwrap();
        let back = Config::load(&paths.config).unwrap();
        assert_eq!(back, c);
        Secrets { db_password: "pw".into() }.save(&paths.secrets).unwrap();
        assert_eq!(Secrets::load(&paths.secrets).unwrap().db_password, "pw");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&paths.secrets).unwrap().permissions().mode() & 0o777, 0o600);
        }
        assert_eq!(Config::load(&dir.join("missing.json")).unwrap(), Config::default());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(slug("Anna Rossi!"), "anna-rossi");
    }
}
