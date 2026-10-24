use eframe::egui::{Pos2, Rect, Vec2, pos2, vec2};

pub const MIN_SCALE: f32 = 0.01;
pub const MAX_SCALE: f32 = 32.0;
pub const STEP: f32 = 1.25;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Zoom {
    /// Multiple of the fit-to-window scale. Keeps the same framing when
    /// the next photo has a different resolution.
    Fit(f32),
    /// Image pixels per screen pixel.
    Pixel(f32),
}

/// Zoom and pan survive photo changes, which makes comparing a burst of
/// similar shots possible without zooming in again on each one.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct View {
    pub zoom: Zoom,
    /// Point of the image shown at the center of the window, 0..1 on both axes.
    pub center: Vec2,
}

impl Default for View {
    fn default() -> Self {
        Self {
            zoom: Zoom::Fit(1.0),
            center: vec2(0.5, 0.5),
        }
    }
}

/// Lengths are in physical pixels so that 100% means one image pixel per screen pixel.
pub fn fit_scale(image: Vec2, area: Vec2) -> f32 {
    (area.x / image.x).min(area.y / image.y).min(1.0)
}

impl View {
    pub fn scale(&self, image: Vec2, area: Vec2) -> f32 {
        let s = match self.zoom {
            Zoom::Fit(m) => fit_scale(image, area) * m,
            Zoom::Pixel(s) => s,
        };
        s.clamp(min_scale(image, area), MAX_SCALE)
    }

    pub fn is_fit(&self) -> bool {
        self.zoom == Zoom::Fit(1.0)
    }

    /// Where the image lands on screen. `area` and the result are in physical pixels.
    pub fn image_rect(&mut self, image: Vec2, area: Rect) -> Rect {
        let scale = self.scale(image, area.size());
        self.clamp_center(image, area.size(), scale);
        let size = image * scale;
        let min = area.center() - self.center * size;
        Rect::from_min_size(pos2(min.x.round(), min.y.round()), size)
    }

    /// Multiplies the zoom while the image point under `anchor` stays put.
    pub fn zoom_by(&mut self, factor: f32, anchor: Pos2, image: Vec2, area: Rect) {
        let old = self.scale(image, area.size());
        let new = (old * factor).clamp(min_scale(image, area.size()), MAX_SCALE);
        self.set_scale(new, anchor, image, area);
    }

    pub fn set_scale(&mut self, new: f32, anchor: Pos2, image: Vec2, area: Rect) {
        let old = self.scale(image, area.size());
        let offset = anchor - area.center();
        let under = self.center * image + offset / old;
        self.center = (under - offset / new) / image;
        self.zoom = match self.zoom {
            Zoom::Fit(_) => Zoom::Fit(new / fit_scale(image, area.size())),
            Zoom::Pixel(_) => Zoom::Pixel(new),
        };
        self.clamp_center(image, area.size(), new);
    }

    /// Moves the image by `delta` physical pixels.
    pub fn pan(&mut self, delta: Vec2, image: Vec2, area: Vec2) {
        let scale = self.scale(image, area);
        self.center -= delta / (image * scale);
        self.clamp_center(image, area, scale);
    }

    fn clamp_center(&mut self, image: Vec2, area: Vec2, scale: f32) {
        let half = area / (image * scale) * 0.5;
        for (c, h) in [(&mut self.center.x, half.x), (&mut self.center.y, half.y)] {
            *c = if h >= 0.5 { 0.5 } else { c.clamp(h, 1.0 - h) };
        }
    }
}

fn min_scale(image: Vec2, area: Vec2) -> f32 {
    (fit_scale(image, area) * 0.5).clamp(MIN_SCALE, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(1000.0, 800.0));

    #[test]
    fn fit_never_upscales() {
        assert_eq!(fit_scale(vec2(200.0, 100.0), AREA.size()), 1.0);
        assert_eq!(fit_scale(vec2(4000.0, 2000.0), AREA.size()), 0.25);
    }

    #[test]
    fn zoom_keeps_anchor_point_still() {
        let image = vec2(4000.0, 3000.0);
        let mut v = View::default();
        let anchor = pos2(700.0, 300.0);
        let before = v.image_rect(image, AREA);
        let point = (anchor - before.min) / before.size();
        v.zoom_by(3.0, anchor, image, AREA);
        let after = v.image_rect(image, AREA);
        let moved = after.min + point * after.size();
        assert!((moved - anchor).length() < 1.0, "{moved:?}");
    }

    #[test]
    fn framing_carries_over_to_a_different_resolution() {
        let mut v = View::default();
        v.zoom_by(4.0, pos2(250.0, 200.0), vec2(4000.0, 3000.0), AREA);
        let small = v.image_rect(vec2(2000.0, 1500.0), AREA);
        let big = v.image_rect(vec2(4000.0, 3000.0), AREA);
        assert_eq!(small, big);
    }

    #[test]
    fn small_image_stays_centered() {
        let mut v = View {
            center: vec2(0.9, 0.1),
            ..View::default()
        };
        let r = v.image_rect(vec2(200.0, 100.0), AREA);
        assert_eq!(r.center(), AREA.center());
    }
}
