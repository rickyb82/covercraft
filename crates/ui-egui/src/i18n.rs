//! Interface languages. Labels are written in English in the code and looked up here; anything
//! without a translation shows in English. Messages with values use `{name}` placeholders.

mod it;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Language {
    #[default]
    En,
    It,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::En, Self::It];

    pub const fn name(self) -> &'static str {
        match self {
            Self::En => "English",
            Self::It => "Italiano",
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::It => "it",
        }
    }

    pub fn parse(tag: &str) -> Option<Self> {
        match tag.trim().split(['-', '_', '.']).next()?.to_ascii_lowercase().as_str() {
            "en" => Some(Self::En),
            "it" => Some(Self::It),
            _ => None,
        }
    }

    pub fn system() -> Self {
        sys_locale::get_locale().and_then(|tag| Self::parse(&tag)).unwrap_or_default()
    }

    pub fn tr(self, text: &str) -> &str {
        match self {
            Self::En => text,
            Self::It => it::translate(text).unwrap_or(text),
        }
    }

    pub fn fill(self, text: &str, values: &[(&str, &str)]) -> String {
        let mut out = self.tr(text).to_string();
        for (k, v) in values {
            out = out.replace(&format!("{{{k}}}"), v);
        }
        out
    }

    /// Translates the engine's error messages by their leading phrase, keeping the details.
    pub fn tr_message(self, text: &str) -> String {
        if self == Self::En {
            return text.to_string();
        }
        for (en, itx) in it::MESSAGES {
            if let Some(rest) = text.strip_prefix(en) {
                return format!("{itx}{rest}");
            }
        }
        text.to_string()
    }
}

/// Every label the interface translates (kept in step with the `it` table by a test).
pub const LABELS: &[&str] = &[
    "Verify",
    "Verification",
    "History",
    "Radio bridges",
    "Settings",
    "Light theme",
    "Dark theme",
    "Language",
    "Comune",
    "Street",
    "Number",
    "Type a comune",
    "Type a street",
    "e.g. 10, 10/A",
    "Check coverage",
    "Clear",
    "Choose the civico",
    "More than one civico matches the address: pick one.",
    "No verification yet",
    "Type comune, street and number above, then Check coverage.",
    "Best option",
    "Covered",
    "Planned",
    "Not covered",
    "Unknown",
    "EGON",
    "Coordinates",
    "from Open Fiber",
    "from the street",
    "not known",
    "Technology",
    "Status",
    "Speed",
    "Distance",
    "Planned for",
    "Raw value",
    "Details",
    "Civico facts",
    "Staircases",
    "Building",
    "Staircase",
    "POP",
    "Cluster",
    "Units",
    "RFC planned",
    "RFA actual",
    "Updated",
    "XGS-PON",
    "Bridge",
    "Band",
    "Bearing",
    "In reach",
    "Out of reach",
    "Reach",
    "Mobile cells",
    "Operator",
    "Radio",
    "Range",
    "In range",
    "No coordinates: radio bridges and cells not computed",
    "No TIM/FiberCop row for this civico",
    "No Open Fiber row for this civico",
    "No Fastweb row for this civico",
    "No radio bridge with coordinates",
    "Past verifications",
    "Everyone's",
    "Mine",
    "Refresh",
    "No verification recorded yet",
    "Date",
    "User",
    "Summary",
    "Open",
    "Altitude",
    "State",
    "Name",
    "No radio bridge in the database",
    "Database",
    "Kind",
    "Host",
    "Port",
    "Password",
    "Path",
    "Save and connect",
    "Test connection",
    "Connected",
    "Connection error",
    "Not tested",
    "Demo data (built-in city)",
    "MySQL (the netmap database)",
    "SQLite file (same schema)",
    "Password set",
    "Password not set",
    "leave empty to keep it",
    "Preferences",
    "Record verifications in the log",
    "Radio reach by band (km)",
    "Bridges listed",
    "Mobile cells around the address",
    "Cells radius (km)",
    "Cells listed",
    "Wording rules",
    "One per line: VALUE=covered|planned|not_covered|unknown (VALUE* for a prefix)",
    "Save preferences",
    "Saved",
    "Server unreachable ({url}): {error}. Start it with: covercraft-cli serve",
    "Server",
    "Checking…",
    "Loading…",
    "Jobs pending: {n}",
    "Ready",
    "Verified {address}",
    "CoverCraft checks the connectivity available at an address from your coverage database: TIM/FiberCop, Open Fiber, Fastweb, your own network, your radio bridges and mobile cells.",
    "Logged as {user}",
    "Not logged",
    "Nearest bridge",
    "km",
    "m",
    "Street code",
    "Civico code",
    "Source",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_label_has_an_italian_translation() {
        let missing: Vec<&str> = LABELS.iter().copied().filter(|l| it::translate(l).is_none()).collect();
        assert!(missing.is_empty(), "missing Italian: {missing:?}");
        assert_eq!(Language::parse("it-IT"), Some(Language::It));
        assert_eq!(Language::It.fill("Jobs pending: {n}", &[("n", "3")]), "Operazioni in corso: 3");
        assert!(Language::It.tr_message("not found: comune `x`").starts_with("non trovato"));
    }
}
