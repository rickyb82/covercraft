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
    "Region",
    "Province",
    "Any",
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
    "Keep a local history of verifications",
    "Also write them to the database log (requests_log; needs INSERT rights)",
    "from the street's buildings",
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
    "Map",
    "Drag to move, scroll with Ctrl to zoom. Lines: links with a working budget; dashed: within reach but weak.",
    "Link budget",
    "min RX",
    "fade margin",
    "Margin",
    "Rate",
    "Quality",
    "Excellent",
    "Good",
    "Marginal",
    "Insufficient",
    "Clear path assumed: the terrain is not checked yet.",
    "Set the point by hand",
    "Use these coordinates",
    "Open in OpenStreetMap",
    "Coordinates must be two numbers, e.g. 39.2238 and 9.1217",
    "Geocoder (Nominatim URL)",
    "OpenCelliD token (live cells)",
    "With a token (free at opencellid.org), every verification also asks OpenCelliD for the cells within about 1 km of the point; they join the table's.",
    "live",
    "Map tiles URL",
    "The geocoder is asked only when no table places the civico; the answer is remembered.",
    "Radio links (5 GHz)",
    "The equipment on both ends; the sectors on the bridges use the same figures. Presets: 5.5 GHz, 40 MHz, 23 dBm, 23 dBi.",
    "Save radio settings",
    "Frequency (GHz)",
    "Channel width (MHz)",
    "TX power (dBm)",
    "Bridge/sector antenna gain (dBi)",
    "Customer antenna gain (dBi)",
    "Cable losses per side (dB)",
    "Fade margin (dB)",
    "Minimum RX for a usable link (dBm)",
    "Customer antenna height (m)",
    "geocoded",
    "by hand",
    "Find a mobile cell",
    "MNC: TIM 1, Vodafone 10, WindTre 88, Iliad 50. Area is the LAC (GSM/UMTS) or TAC (LTE/NR); the cell id is the CID, for LTE the ECI (eNodeB × 256 + sector).",
    "MCC",
    "MNC",
    "Area (LAC/TAC)",
    "Cell (CID/ECI)",
    "eNodeB",
    "Sector",
    "Find",
    "Cell",
    "No cell found",
    "Estimated position",
    "Range (m)",
    "Samples",
    "The position is OpenCelliD's estimate (a centroid of user measurements): the error is of the order of the range.",
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
