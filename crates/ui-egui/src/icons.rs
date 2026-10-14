//! The icon set, drawn in code (strokes on a 24-unit grid): no bitmap or vendor artwork.

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Logo,
    Search,
    Sync,
    Settings,
    Moon,
    Sun,
    Globe,
    Close,
    Chevron,
    ChevronRight,
    Back,
    Plus,
    Check,
    More,
    Warning,
    Print,
    Download,
    Keyboard,
    Save,
    Account,
    Pin,
    Antenna,
    History,
    Database,
    Signal,
    Fiber,
    Map,
    Verify,
}

/// Paints `icon` into `rect` with stroke colour `c`.
pub fn paint(p: &Painter, rect: Rect, icon: Icon, c: Color32) {
    let s = rect.width().min(rect.height()) / 24.0;
    let o = rect.center() - vec2(12.0, 12.0) * s;
    let at = |x: f32, y: f32| -> Pos2 { o + vec2(x, y) * s };
    let w = (1.7 * s).max(1.0);
    let st = Stroke::new(w, c);
    let line = |pts: &[(f32, f32)]| {
        p.add(Shape::line(pts.iter().map(|(x, y)| at(*x, *y)).collect(), st));
    };
    let rrect = |x0: f32, y0: f32, x1: f32, y1: f32, r: f32| {
        p.rect_stroke(Rect::from_min_max(at(x0, y0), at(x1, y1)), r * s, st, egui::StrokeKind::Middle);
    };
    let circle = |x: f32, y: f32, r: f32| {
        p.circle_stroke(at(x, y), r * s, st);
    };
    let dot = |x: f32, y: f32, r: f32| {
        p.circle_filled(at(x, y), r * s, c);
    };
    match icon {
        Icon::Logo => {
            // The CoverCraft mark: a map pin on a signal arc, with the craft-family spark.
            p.circle_filled(at(12.0, 12.0), 9.5 * s, Color32::from_rgb(0x22, 0xa0, 0x6b));
            p.add(Shape::line(arc(at(12.0, 14.0), 5.5 * s, 200.0, 340.0, 12), Stroke::new(w * 1.1, Color32::WHITE)));
            p.add(Shape::line(arc(at(12.0, 14.0), 2.8 * s, 200.0, 340.0, 8), Stroke::new(w * 1.1, Color32::WHITE)));
            p.circle_filled(at(12.0, 14.5), 1.4 * s, Color32::WHITE);
            p.circle_filled(at(19.0, 5.0), 3.2 * s, Color32::from_rgb(0xff, 0xb4, 0x3d));
        }
        Icon::Search => {
            circle(10.5, 10.5, 6.0);
            line(&[(15.0, 15.0), (20.5, 20.5)]);
        }
        Icon::Sync => {
            p.add(Shape::line(arc(at(12.0, 12.0), 7.5 * s, 210.0, 330.0, 14), st));
            p.add(Shape::line(arc(at(12.0, 12.0), 7.5 * s, 30.0, 150.0, 14), st));
            line(&[(15.5, 5.0), (18.5, 8.2), (14.8, 9.0)]);
            line(&[(8.5, 19.0), (5.5, 15.8), (9.2, 15.0)]);
        }
        Icon::Settings => {
            circle(12.0, 12.0, 3.2);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let (cx, cy) = (a.cos(), a.sin());
                line(&[(12.0 + cx * 6.0, 12.0 + cy * 6.0), (12.0 + cx * 9.0, 12.0 + cy * 9.0)]);
            }
            circle(12.0, 12.0, 6.5);
        }
        Icon::Moon => {
            p.add(Shape::line(arc(at(12.0, 12.0), 8.0 * s, 40.0, 320.0, 24), st));
            p.add(Shape::line(arc(at(17.0, 9.0), 6.4 * s, 100.0, 232.0, 16), st));
        }
        Icon::Sun => {
            circle(12.0, 12.0, 4.0);
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let (cx, cy) = (a.cos(), a.sin());
                line(&[(12.0 + cx * 7.0, 12.0 + cy * 7.0), (12.0 + cx * 9.5, 12.0 + cy * 9.5)]);
            }
        }
        Icon::Globe => {
            circle(12.0, 12.0, 8.5);
            line(&[(3.5, 12.0), (20.5, 12.0)]);
            p.add(Shape::line(
                arc(at(12.0, 12.0), 8.5 * s, -90.0, 90.0, 16)
                    .into_iter()
                    .map(|q| pos2(at(12.0, 0.0).x + (q.x - at(12.0, 0.0).x) * 0.45, q.y))
                    .collect(),
                st,
            ));
            p.add(Shape::line(
                arc(at(12.0, 12.0), 8.5 * s, 90.0, 270.0, 16)
                    .into_iter()
                    .map(|q| pos2(at(12.0, 0.0).x + (q.x - at(12.0, 0.0).x) * 0.45, q.y))
                    .collect(),
                st,
            ));
        }
        Icon::Close => {
            line(&[(6.0, 6.0), (18.0, 18.0)]);
            line(&[(18.0, 6.0), (6.0, 18.0)]);
        }
        Icon::Back => {
            line(&[(10.0, 6.0), (4.0, 12.0), (10.0, 18.0)]);
            line(&[(4.0, 12.0), (20.0, 12.0)]);
        }
        Icon::Plus => {
            line(&[(12.0, 5.0), (12.0, 19.0)]);
            line(&[(5.0, 12.0), (19.0, 12.0)]);
        }
        Icon::Check => {
            circle(12.0, 12.0, 8.5);
            line(&[(7.5, 12.5), (10.5, 15.5), (16.5, 9.0)]);
        }
        Icon::More => {
            dot(6.0, 12.0, 1.7);
            dot(12.0, 12.0, 1.7);
            dot(18.0, 12.0, 1.7);
        }
        Icon::Print => {
            rrect(4.0, 9.5, 20.0, 17.5, 1.5);
            line(&[(7.5, 9.5), (7.5, 4.5), (16.5, 4.5), (16.5, 9.5)]);
            line(&[(7.5, 14.5), (7.5, 20.0), (16.5, 20.0), (16.5, 14.5)]);
            dot(17.0, 12.0, 0.9);
        }
        Icon::Download => {
            line(&[(12.0, 4.0), (12.0, 15.0)]);
            line(&[(7.5, 10.5), (12.0, 15.0), (16.5, 10.5)]);
            line(&[(4.5, 19.5), (19.5, 19.5)]);
        }
        Icon::Keyboard => {
            rrect(3.0, 6.5, 21.0, 17.5, 1.5);
            for x in [6.5, 10.0, 13.5, 17.0] {
                dot(x, 10.0, 0.9);
            }
            line(&[(7.5, 14.0), (16.5, 14.0)]);
        }
        Icon::Save => {
            line(&[(4.0, 4.0), (16.5, 4.0), (20.0, 7.5), (20.0, 20.0), (4.0, 20.0), (4.0, 4.0)]);
            rrect(8.0, 4.0, 15.0, 9.0, 0.5);
            rrect(7.5, 13.0, 16.5, 20.0, 0.5);
        }
        Icon::Account => {
            circle(12.0, 8.5, 4.5);
            p.add(Shape::line(arc(at(12.0, 22.0), 8.0 * s, 200.0, 340.0, 14), st));
        }
        Icon::Chevron => {
            line(&[(7.0, 10.0), (12.0, 15.0), (17.0, 10.0)]);
        }
        Icon::ChevronRight => {
            line(&[(10.0, 7.0), (15.0, 12.0), (10.0, 17.0)]);
        }
        Icon::Warning => {
            line(&[(12.0, 3.5), (21.0, 20.0), (3.0, 20.0), (12.0, 3.5)]);
            line(&[(12.0, 9.5), (12.0, 14.5)]);
            dot(12.0, 17.2, 1.1);
        }
        Icon::Pin => {
            line(&[(12.0, 21.0), (6.0, 13.5), (5.0, 9.5), (7.0, 5.5), (12.0, 3.5), (17.0, 5.5), (19.0, 9.5), (18.0, 13.5), (12.0, 21.0)]);
            circle(12.0, 9.5, 2.5);
        }
        Icon::Antenna => {
            line(&[(12.0, 21.0), (12.0, 10.0)]);
            line(&[(8.0, 21.0), (12.0, 14.0), (16.0, 21.0)]);
            dot(12.0, 9.0, 1.6);
            p.add(Shape::line(arc(at(12.0, 9.0), 4.5 * s, 200.0, 340.0, 10), st));
            p.add(Shape::line(arc(at(12.0, 9.0), 7.5 * s, 205.0, 335.0, 12), st));
        }
        Icon::History => {
            p.add(Shape::line(arc(at(12.5, 12.0), 8.0 * s, 150.0, 480.0, 24), st));
            line(&[(4.5, 7.0), (4.5, 12.0), (9.5, 12.0)]);
            line(&[(12.5, 7.5), (12.5, 12.5), (16.0, 14.5)]);
        }
        Icon::Database => {
            p.add(Shape::closed_line(
                arc(at(12.0, 6.5), 7.5 * s, 0.0, 360.0, 24)
                    .into_iter()
                    .map(|q| Pos2::new(q.x, at(0.0, 6.5).y + (q.y - at(0.0, 6.5).y) * 0.4))
                    .collect(),
                st,
            ));
            line(&[(4.5, 6.5), (4.5, 17.5)]);
            line(&[(19.5, 6.5), (19.5, 17.5)]);
            p.add(Shape::line(
                arc(at(12.0, 17.5), 7.5 * s, 0.0, 180.0, 12)
                    .into_iter()
                    .map(|q| Pos2::new(q.x, at(0.0, 17.5).y + (q.y - at(0.0, 17.5).y) * 0.4))
                    .collect(),
                st,
            ));
            p.add(Shape::line(
                arc(at(12.0, 12.0), 7.5 * s, 0.0, 180.0, 12)
                    .into_iter()
                    .map(|q| Pos2::new(q.x, at(0.0, 12.0).y + (q.y - at(0.0, 12.0).y) * 0.4))
                    .collect(),
                st,
            ));
        }
        Icon::Signal => {
            line(&[(4.0, 19.0), (4.0, 15.5)]);
            line(&[(8.5, 19.0), (8.5, 12.0)]);
            line(&[(13.0, 19.0), (13.0, 8.5)]);
            line(&[(17.5, 19.0), (17.5, 5.0)]);
        }
        Icon::Fiber => {
            line(&[(3.5, 16.0), (8.0, 16.0), (11.0, 8.0), (14.0, 16.0), (17.0, 11.0), (20.5, 11.0)]);
            dot(3.5, 16.0, 1.3);
            dot(20.5, 11.0, 1.3);
        }
        Icon::Map => {
            line(&[(3.5, 6.0), (9.0, 4.0), (15.0, 6.5), (20.5, 4.5), (20.5, 18.0), (15.0, 20.0), (9.0, 17.5), (3.5, 19.5), (3.5, 6.0)]);
            line(&[(9.0, 4.0), (9.0, 17.5)]);
            line(&[(15.0, 6.5), (15.0, 20.0)]);
        }
        Icon::Verify => {
            circle(10.5, 10.5, 6.5);
            line(&[(15.5, 15.5), (20.5, 20.5)]);
            line(&[(7.5, 10.5), (9.8, 12.8), (13.5, 8.5)]);
        }
    }
}

/// Points on a circular arc, angles in degrees (0 = right, clockwise on screen).
fn arc(c: Pos2, r: f32, from: f32, to: f32, n: usize) -> Vec<Pos2> {
    (0..=n)
        .map(|i| {
            let a = (from + (to - from) * i as f32 / n as f32).to_radians();
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect()
}
