//! Line icons drawn with the painter, on a 24×24 grid like the mockup's SVGs.
//! Drawing them keeps them sharp at any scale and colored by the theme.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use egui::{Color32, Painter, Pos2, Rect, Shape, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Download,
    Music,
    Video,
    History,
    Sliders,
    Folder,
    Check,
    Close,
    Warning,
    Copy,
    Sun,
    Moon,
    Monitor,
    Link,
}

/// Maps the 24-unit grid onto `rect`.
struct Grid {
    rect: Rect,
    scale: f32,
}

impl Grid {
    fn p(&self, x: f32, y: f32) -> Pos2 {
        self.rect.min + egui::vec2(x, y) * self.scale
    }

    fn arc(&self, cx: f32, cy: f32, r: f32, from: f32, to: f32) -> Vec<Pos2> {
        let steps = ((to - from).abs() / TAU * 48.0).ceil().max(4.0) as usize;
        (0..=steps)
            .map(|i| {
                let a = from + (to - from) * i as f32 / steps as f32;
                self.p(cx + r * a.cos(), cy + r * a.sin())
            })
            .collect()
    }

    fn rounded_rect(&self, x: f32, y: f32, w: f32, h: f32, r: f32) -> Vec<Pos2> {
        let mut pts = Vec::new();
        pts.extend(self.arc(x + w - r, y + r, r, -FRAC_PI_2, 0.0));
        pts.extend(self.arc(x + w - r, y + h - r, r, 0.0, FRAC_PI_2));
        pts.extend(self.arc(x + r, y + h - r, r, FRAC_PI_2, PI));
        pts.extend(self.arc(x + r, y + r, r, PI, PI + FRAC_PI_2));
        pts
    }
}

pub fn paint(painter: &Painter, rect: Rect, icon: Icon, color: Color32) {
    let side = rect.width().min(rect.height());
    let rect = Rect::from_center_size(rect.center(), egui::vec2(side, side));
    let g = Grid {
        rect,
        scale: side / 24.0,
    };
    let stroke = Stroke::new((2.0 * g.scale).max(1.25), color);
    let line = |pts: Vec<Pos2>| painter.add(Shape::line(pts, stroke));
    let closed = |pts: Vec<Pos2>| painter.add(Shape::closed_line(pts, stroke));
    let circle = |cx: f32, cy: f32, r: f32| painter.circle_stroke(g.p(cx, cy), r * g.scale, stroke);
    let dot = |cx: f32, cy: f32, r: f32| painter.circle_filled(g.p(cx, cy), r * g.scale, color);

    match icon {
        Icon::Download => {
            line(vec![g.p(12.0, 4.0), g.p(12.0, 15.0)]);
            line(vec![g.p(7.0, 10.0), g.p(12.0, 15.0), g.p(17.0, 10.0)]);
            line(vec![g.p(5.0, 20.0), g.p(19.0, 20.0)]);
        }
        Icon::Music => {
            line(vec![
                g.p(9.0, 18.0),
                g.p(9.0, 5.0),
                g.p(21.0, 3.0),
                g.p(21.0, 16.0),
            ]);
            circle(6.0, 18.0, 3.0);
            circle(18.0, 16.0, 3.0);
        }
        Icon::Video => {
            closed(g.rounded_rect(2.0, 5.0, 14.0, 14.0, 2.0));
            closed(vec![
                g.p(16.0, 10.0),
                g.p(22.0, 7.0),
                g.p(22.0, 17.0),
                g.p(16.0, 14.0),
            ]);
        }
        Icon::History => {
            line(g.arc(12.0, 12.0, 9.0, PI, PI + TAU * 0.83));
            line(vec![g.p(3.0, 4.0), g.p(3.0, 9.0), g.p(8.0, 9.0)]);
            line(vec![g.p(12.0, 7.0), g.p(12.0, 12.0), g.p(15.0, 14.0)]);
        }
        Icon::Sliders => {
            line(vec![g.p(4.0, 7.0), g.p(14.0, 7.0)]);
            line(vec![g.p(18.0, 7.0), g.p(20.0, 7.0)]);
            line(vec![g.p(4.0, 17.0), g.p(8.0, 17.0)]);
            line(vec![g.p(12.0, 17.0), g.p(20.0, 17.0)]);
            circle(16.0, 7.0, 2.0);
            circle(10.0, 17.0, 2.0);
        }
        Icon::Folder => {
            closed(vec![
                g.p(3.0, 6.0),
                g.p(9.0, 6.0),
                g.p(11.0, 8.0),
                g.p(21.0, 8.0),
                g.p(21.0, 19.0),
                g.p(3.0, 19.0),
            ]);
        }
        Icon::Check => {
            line(vec![g.p(5.0, 12.0), g.p(10.0, 17.0), g.p(19.0, 7.0)]);
        }
        Icon::Close => {
            line(vec![g.p(6.0, 6.0), g.p(18.0, 18.0)]);
            line(vec![g.p(18.0, 6.0), g.p(6.0, 18.0)]);
        }
        Icon::Warning => {
            closed(vec![g.p(12.0, 3.5), g.p(22.0, 20.5), g.p(2.0, 20.5)]);
            line(vec![g.p(12.0, 9.5), g.p(12.0, 14.0)]);
            dot(12.0, 17.2, 1.1);
        }
        Icon::Copy => {
            closed(g.rounded_rect(9.0, 9.0, 12.0, 12.0, 2.0));
            line(vec![g.p(5.0, 15.0), g.p(5.0, 5.0), g.p(15.0, 5.0)]);
        }
        Icon::Sun => {
            circle(12.0, 12.0, 4.0);
            for i in 0..8 {
                let a = i as f32 * TAU / 8.0;
                line(vec![
                    g.p(12.0 + 7.0 * a.cos(), 12.0 + 7.0 * a.sin()),
                    g.p(12.0 + 9.5 * a.cos(), 12.0 + 9.5 * a.sin()),
                ]);
            }
        }
        Icon::Moon => {
            // Outer arc of the crescent (r 9 around the center), then the
            // inner bite (r 7) back to the start.
            let mut pts = g.arc(12.0, 12.0, 9.0, 0.087, 4.625);
            pts.extend(g.arc(16.85, 7.15, 7.0, 3.776, 0.936));
            closed(pts);
        }
        Icon::Monitor => {
            closed(g.rounded_rect(3.0, 4.0, 18.0, 12.0, 2.0));
            line(vec![g.p(8.0, 20.0), g.p(16.0, 20.0)]);
            line(vec![g.p(12.0, 16.0), g.p(12.0, 20.0)]);
        }
        Icon::Link => {
            line(g.arc(15.5, 8.5, 4.2, PI * 0.75, PI * 2.25));
            line(g.arc(8.5, 15.5, 4.2, -PI * 0.25, PI * 1.25));
            line(vec![g.p(9.5, 14.5), g.p(14.5, 9.5)]);
        }
    }
}
