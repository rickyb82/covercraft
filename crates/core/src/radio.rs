//! The radio link between a bridge and the customer: free-space path loss, received power,
//! Fresnel clearance and an estimated rate for a 5 GHz point-to-point link. Line of sight is
//! not known without terrain data: the estimate says what the link would do with clear sky.

use crate::Result;

/// The equipment on both ends; the sectors on the bridges use the same figures.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct RadioParams {
    /// Centre frequency, GHz.
    pub frequency_ghz: f64,
    /// Channel width, MHz (20, 40, 80).
    pub channel_mhz: f64,
    /// Transmit power at the radio, dBm.
    pub tx_power_dbm: f64,
    /// Antenna gain on the bridge (sector), dBi.
    pub bridge_gain_dbi: f64,
    /// Antenna gain at the customer, dBi.
    pub client_gain_dbi: f64,
    /// Cable and connector losses per side, dB.
    pub cable_loss_db: f64,
    /// Extra margin kept for rain, misalignment and interference, dB.
    pub fade_margin_db: f64,
    /// The weakest received power that still gives a usable link, dBm.
    pub min_rx_dbm: f64,
    /// Height of the customer's antenna above ground, m (for the terrain check).
    pub client_height_m: f64,
}

impl Default for RadioParams {
    fn default() -> Self {
        Self {
            frequency_ghz: 5.5,
            channel_mhz: 40.0,
            tx_power_dbm: 23.0,
            bridge_gain_dbi: 23.0,
            client_gain_dbi: 23.0,
            cable_loss_db: 1.0,
            fade_margin_db: 10.0,
            min_rx_dbm: -75.0,
            client_height_m: 6.0,
        }
    }
}

impl RadioParams {
    /// Applies the fields of a JSON object (only those given), each within sane bounds.
    pub fn apply(&mut self, v: &serde_json::Value) -> Result<()> {
        let Some(o) = v.as_object() else { return Err(crate::Error::Invalid("radio (object expected)".into())) };
        for (k, val) in o {
            let n = val
                .as_f64()
                .or_else(|| val.as_str().and_then(|s| s.trim().replace(',', ".").parse().ok()))
                .ok_or_else(|| crate::Error::Invalid(format!("radio {k}")))?;
            if !n.is_finite() {
                return Err(crate::Error::Invalid(format!("radio {k}")));
            }
            let (field, lo, hi): (&mut f64, f64, f64) = match k.as_str() {
                "frequency_ghz" => (&mut self.frequency_ghz, 0.1, 100.0),
                "channel_mhz" => (&mut self.channel_mhz, 5.0, 320.0),
                "tx_power_dbm" => (&mut self.tx_power_dbm, -30.0, 60.0),
                "bridge_gain_dbi" => (&mut self.bridge_gain_dbi, 0.0, 60.0),
                "client_gain_dbi" => (&mut self.client_gain_dbi, 0.0, 60.0),
                "cable_loss_db" => (&mut self.cable_loss_db, 0.0, 30.0),
                "fade_margin_db" => (&mut self.fade_margin_db, 0.0, 40.0),
                "min_rx_dbm" => (&mut self.min_rx_dbm, -120.0, 0.0),
                "client_height_m" => (&mut self.client_height_m, 0.0, 200.0),
                other => return Err(crate::Error::Invalid(format!("radio `{other}`"))),
            };
            if !(lo..=hi).contains(&n) {
                return Err(crate::Error::Invalid(format!("radio {k} ({lo}..{hi})")));
            }
            *field = n;
        }
        Ok(())
    }

    /// Radiated power of a bridge, dBm.
    pub fn eirp_dbm(&self) -> f64 {
        self.tx_power_dbm + self.bridge_gain_dbi - self.cable_loss_db
    }
}

/// How the link would perform with a clear path.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LinkEstimate {
    pub fspl_db: f64,
    pub rx_dbm: f64,
    /// Received power minus the minimum, minus the fade margin.
    pub margin_db: f64,
    /// First Fresnel zone radius at the middle of the path, m.
    pub fresnel_m: f64,
    /// Estimated throughput, Mbit/s (both directions summed as the radio reports it).
    pub rate_mbps: f64,
    /// `excellent`, `good`, `marginal`, `insufficient`.
    pub quality: String,
}

impl LinkEstimate {
    pub fn ok(&self) -> bool {
        self.margin_db >= 0.0
    }
}

/// Free-space path loss, dB.
pub fn fspl_db(distance_m: f64, frequency_ghz: f64) -> f64 {
    let d_km = (distance_m / 1000.0).max(0.001);
    let f_mhz = (frequency_ghz * 1000.0).max(1.0);
    20.0 * d_km.log10() + 20.0 * f_mhz.log10() + 32.44
}

/// First Fresnel zone radius at mid-path, m.
pub fn fresnel_m(distance_m: f64, frequency_ghz: f64) -> f64 {
    let d_km = (distance_m / 1000.0).max(0.0);
    8.657 * (d_km / frequency_ghz.max(0.1)).sqrt()
}

/// Rate from received power: an 802.11ac-class 2×2 MIMO table at 40 MHz, scaled by the
/// channel width. The thresholds are generous on purpose: the field decides.
pub fn rate_mbps(rx_dbm: f64, channel_mhz: f64) -> f64 {
    const TABLE_40: [(f64, f64); 10] = [
        (-93.0, 15.0),
        (-90.0, 30.0),
        (-88.0, 45.0),
        (-85.0, 60.0),
        (-81.0, 90.0),
        (-77.0, 120.0),
        (-75.0, 135.0),
        (-73.0, 150.0),
        (-69.0, 180.0),
        (-65.0, 200.0),
    ];
    let scale = (channel_mhz / 40.0).clamp(0.25, 4.0);
    // A wider channel needs ~3 dB more signal per doubling.
    let rx = rx_dbm - 3.0 * (channel_mhz / 40.0).max(0.25).log2();
    TABLE_40.iter().filter(|(th, _)| rx >= *th).map(|(_, r)| r * scale).fold(0.0, f64::max)
}

/// The link estimate for a distance.
pub fn estimate(distance_m: f64, p: &RadioParams) -> LinkEstimate {
    let fspl = fspl_db(distance_m, p.frequency_ghz);
    let rx = p.eirp_dbm() + p.client_gain_dbi - p.cable_loss_db - fspl;
    let margin = rx - p.min_rx_dbm - p.fade_margin_db;
    let quality = if margin >= 15.0 {
        "excellent"
    } else if margin >= 6.0 {
        "good"
    } else if margin >= 0.0 {
        "marginal"
    } else {
        "insufficient"
    };
    LinkEstimate {
        fspl_db: fspl,
        rx_dbm: rx,
        margin_db: margin,
        fresnel_m: fresnel_m(distance_m, p.frequency_ghz),
        rate_mbps: rate_mbps(rx, p.channel_mhz),
        quality: quality.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_at_five_ghz() {
        let p = RadioParams::default();
        assert!((fspl_db(1000.0, 5.5) - 107.25).abs() < 0.1, "{}", fspl_db(1000.0, 5.5));
        assert!((p.eirp_dbm() - 45.0).abs() < 1e-9);
        let near = estimate(2_000.0, &p);
        assert!(near.ok() && near.quality == "excellent", "{near:?}");
        assert!(near.rate_mbps >= 150.0);
        let far = estimate(25_000.0, &p);
        assert!(far.rx_dbm < near.rx_dbm);
        assert!((far.fresnel_m - 18.5).abs() < 0.5, "{}", far.fresnel_m);
        let very_far = estimate(80_000.0, &p);
        assert!(!very_far.ok() && very_far.quality == "insufficient");
        assert_eq!(rate_mbps(-100.0, 40.0), 0.0);
        assert!(rate_mbps(-60.0, 80.0) > rate_mbps(-60.0, 40.0));
        let mut q = p.clone();
        q.apply(&serde_json::json!({"tx_power_dbm": "27", "bridge_gain_dbi": 19})).unwrap();
        assert!((q.eirp_dbm() - 45.0).abs() < 1e-9);
        assert!(q.apply(&serde_json::json!({"tx_power_dbm": 99})).is_err());
        assert!(q.apply(&serde_json::json!({"nope": 1})).is_err());
        assert!(q.apply(&serde_json::json!(3)).is_err());
    }
}
