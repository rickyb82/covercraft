//! The map (OpenStreetMap tiles through `walkers`): the customer's point, the bridges around
//! it and the links that work.

use egui::{Align2, Color32, FontId, Rect, Stroke, pos2, vec2};
use walkers::sources::{Attribution, TileSource};
use walkers::{HttpTiles, Map, MapMemory, TileId, lat_lon};

use crate::core::geo::Point;
use crate::core::model::{Bridge, Report};
use crate::theme::Tokens;

/// Any `{z}/{x}/{y}` tile server; OpenStreetMap when the template is empty.
struct UrlTiles {
    template: String,
}

impl TileSource for UrlTiles {
    fn tile_url(&self, id: TileId) -> String {
        let t = if self.template.trim().is_empty() { "https://tile.openstreetmap.org/{z}/{x}/{y}.png" } else { self.template.trim() };
        t.replace("{z}", &id.zoom.to_string()).replace("{x}", &id.x.to_string()).replace("{y}", &id.y.to_string())
    }

    fn attribution(&self) -> Attribution {
        Attribution { text: "© OpenStreetMap contributors", url: "https://www.openstreetmap.org/copyright", logo_light: None, logo_dark: None }
    }
}

/// The map's state across frames.
#[derive(Default)]
pub struct MapState {
    tiles: Option<HttpTiles>,
    tiles_template: String,
    pub memory: MapMemory,
    /// What the map was centred on last (the report's EGON code or `bridges`).
    centred_on: String,
    /// Fetch tiles over the network (off in tests and offscreen renders).
    pub tiles_enabled: bool,
}

impl MapState {
    pub fn new(tiles_enabled: bool) -> Self {
        Self { tiles_enabled, ..Default::default() }
    }

    /// Makes sure the tile fetcher exists for the template (when tiles are enabled).
    fn ensure_tiles(&mut self, ctx: &egui::Context, template: &str) {
        if !self.tiles_enabled {
            self.tiles = None;
            return;
        }
        if self.tiles.is_none() || self.tiles_template != template {
            self.tiles = Some(HttpTiles::with_options(UrlTiles { template: template.to_string() }, http_options(), ctx.clone()));
            self.tiles_template = template.to_string();
        }
    }

    /// Centres the map on `point` the first time `key` is shown.
    fn centre(&mut self, key: &str, point: Point, zoom: f64) {
        if self.centred_on != key {
            self.centred_on = key.to_string();
            self.memory.center_at(lat_lon(point.lat, point.lon));
            let _ = self.memory.set_zoom(zoom);
        }
    }
}

/// Tile fetching: a proper user agent and a disk cache natively; the browser's own on the web.
fn http_options() -> walkers::HttpOptions {
    #[cfg(not(target_arch = "wasm32"))]
    {
        walkers::HttpOptions {
            user_agent: Some(walkers::HeaderValue::from_static(concat!("covercraft/", env!("CARGO_PKG_VERSION")))),
            cache: Some(std::env::temp_dir().join("covercraft-tiles")),
            ..Default::default()
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        walkers::HttpOptions::default()
    }
}

fn marker(p: &egui::Painter, at: egui::Pos2, color: Color32, label: &str, t: &Tokens) {
    p.circle_filled(at, 7.0, color);
    p.circle_stroke(at, 7.0, Stroke::new(2.0, Color32::WHITE));
    if !label.is_empty() {
        let galley = p.layout_no_wrap(label.to_string(), FontId::proportional(11.5), t.ui_text);
        let rect = Rect::from_min_size(at + vec2(10.0, -galley.size().y / 2.0), galley.size()).expand2(vec2(4.0, 2.0));
        p.rect_filled(rect, 4.0, t.card.gamma_multiply(0.9));
        p.galley(rect.min + vec2(4.0, 2.0), galley, t.ui_text);
    }
}

/// The report's map: the civico in the middle, the bridges with their links.
pub fn report_map(state: &mut MapState, ui: &mut egui::Ui, t: &Tokens, r: &Report, template: &str, height: f32) {
    let Some(here) = r.point else { return };
    state.centre(&format!("report:{}", r.civico.egon), here, 12.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    state.ensure_tiles(ui.ctx(), template);
    let MapState { tiles, memory, .. } = state;
    let tiles = tiles.as_mut().map(|tl| tl as &mut dyn walkers::Tiles);
    let hits: Vec<(Point, Color32, String, bool)> = r
        .radio
        .iter()
        .filter_map(|h| {
            let p = h.bridge.point()?;
            let ok = h.in_reach && h.link.as_ref().is_none_or(|l| l.ok());
            let color = if ok {
                Color32::from_rgb(0x22, 0xa0, 0x6b)
            } else if h.in_reach {
                Color32::from_rgb(0xf0, 0xa2, 0x0c)
            } else {
                t.muted
            };
            let label = match &h.link {
                Some(l) if h.in_reach => format!("{} · {:.1} km · {:.0} dBm", h.bridge.name, h.distance_m / 1000.0, l.rx_dbm),
                _ => format!("{} · {:.1} km", h.bridge.name, h.distance_m / 1000.0),
            };
            Some((p, color, label, ok))
        })
        .collect();
    let address = r.civico.display();
    Map::new(tiles, memory, lat_lon(here.lat, here.lon)).show(&mut child, |ui, _resp, projector, _mem| {
        let p = ui.painter().with_clip_rect(rect);
        let centre = projector.project(lat_lon(here.lat, here.lon)).to_pos2();
        for (pt, color, label, ok) in &hits {
            let at = projector.project(lat_lon(pt.lat, pt.lon)).to_pos2();
            if *ok {
                p.line_segment([centre, at], Stroke::new(2.0, color.gamma_multiply(0.8)));
            } else {
                dashed(&p, centre, at, *color);
            }
            marker(&p, at, *color, label, t);
        }
        marker(&p, centre, t.accent, &address, t);
    });
    attribution(ui, rect, t);
}

/// Every bridge on a map (the bridges screen).
pub fn bridges_map(state: &mut MapState, ui: &mut egui::Ui, t: &Tokens, bridges: &[Bridge], template: &str, height: f32) {
    let points: Vec<(Point, &Bridge)> = bridges.iter().filter_map(|b| b.point().map(|p| (p, b))).collect();
    let Some(first) = points.first().map(|(p, _)| *p) else { return };
    let n = points.len() as f64;
    let (lat, lon) = points.iter().fold((0.0, 0.0), |(a, b), (p, _)| (a + p.lat, b + p.lon));
    let centre_pt = Point::new(lat / n, lon / n).unwrap_or(first);
    state.centre("bridges", centre_pt, 9.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    state.ensure_tiles(ui.ctx(), template);
    let MapState { tiles, memory, .. } = state;
    let tiles = tiles.as_mut().map(|tl| tl as &mut dyn walkers::Tiles);
    Map::new(tiles, memory, lat_lon(centre_pt.lat, centre_pt.lon)).show(&mut child, |ui, _resp, projector, _mem| {
        let p = ui.painter().with_clip_rect(rect);
        for (pt, b) in &points {
            let at = projector.project(lat_lon(pt.lat, pt.lon)).to_pos2();
            let color = match b.band.to_ascii_uppercase().as_str() {
                "A" => Color32::from_rgb(0x22, 0xa0, 0x6b),
                "B" => Color32::from_rgb(0x1f, 0x9a, 0xb8),
                "C" => Color32::from_rgb(0xf0, 0xa2, 0x0c),
                _ => t.muted,
            };
            marker(&p, at, color, &b.name, t);
        }
    });
    attribution(ui, rect, t);
}

fn dashed(p: &egui::Painter, a: egui::Pos2, b: egui::Pos2, color: Color32) {
    let len = a.distance(b);
    let n = (len / 10.0).clamp(1.0, 400.0) as usize;
    for i in (0..n).step_by(2) {
        let t0 = i as f32 / n as f32;
        let t1 = ((i + 1) as f32 / n as f32).min(1.0);
        p.line_segment([a.lerp(b, t0), a.lerp(b, t1)], Stroke::new(1.5, color));
    }
}

fn attribution(ui: &egui::Ui, rect: Rect, t: &Tokens) {
    let p = ui.painter();
    let text = "© OpenStreetMap contributors";
    let galley = p.layout_no_wrap(text.to_string(), FontId::proportional(10.0), t.muted);
    let r =
        Rect::from_min_size(pos2(rect.right() - galley.size().x - 8.0, rect.bottom() - galley.size().y - 4.0), galley.size()).expand2(vec2(4.0, 2.0));
    p.rect_filled(r, 3.0, t.card.gamma_multiply(0.85));
    p.galley(r.min + vec2(4.0, 2.0), galley, t.muted);
    let _ = Align2::LEFT_TOP;
}
