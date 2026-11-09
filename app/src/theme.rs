use eframe::egui::{Color32, Context, Stroke};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Accent {
    #[default]
    Violet,
    Blue,
    Teal,
    Green,
    Rose,
    Amber,
}
impl Accent {
    pub const ALL: [Self; 6] = [Self::Violet, Self::Blue, Self::Teal, Self::Green, Self::Rose, Self::Amber];
    pub fn name(self) -> &'static str {
        match self { Self::Violet => "Violet", Self::Blue => "Blue", Self::Teal => "Teal", Self::Green => "Green", Self::Rose => "Rose", Self::Amber => "Amber" }
    }
    pub fn from_name(name: &str) -> Self {
        Self::ALL.into_iter().find(|color| color.name() == name).unwrap_or_default()
    }
    pub fn color(self) -> Color32 {
        let [r, g, b] = match self {
            Self::Violet => [124, 58, 237], Self::Blue => [37, 99, 235],
            Self::Teal => [13, 128, 116], Self::Green => [22, 128, 61],
            Self::Rose => [225, 29, 72], Self::Amber => [190, 85, 0],
        };
        Color32::from_rgb(r, g, b)
    }
    pub fn tint(self, amount: f32) -> Color32 {
        let c = self.color();
        let blend = |v: u8| (255. - (255. - v as f32) * amount).round() as u8;
        Color32::from_rgb(blend(c.r()), blend(c.g()), blend(c.b()))
    }
    pub fn apply(self, ctx: &Context) {
        ctx.style_mut(|style| {
            let v = &mut style.visuals;
            v.hyperlink_color = self.color();
            v.selection.bg_fill = self.tint(0.16);
            v.selection.stroke = Stroke::new(1., self.color());
            v.widgets.hovered.bg_fill = self.tint(0.08);
            v.widgets.hovered.weak_bg_fill = self.tint(0.08);
            v.widgets.hovered.bg_stroke = Stroke::new(1., self.color());
            v.widgets.active.bg_fill = self.tint(0.14);
            v.widgets.active.weak_bg_fill = self.tint(0.14);
            v.widgets.active.bg_stroke = Stroke::new(1., self.color());
            v.widgets.active.fg_stroke = Stroke::new(1., self.color());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn presets_keep_text_readable() {
        let luminance = |c: Color32| {
            let linear = |v: u8| { let s = v as f64 / 255.; if s <= 0.04045 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) } };
            0.2126 * linear(c.r()) + 0.7152 * linear(c.g()) + 0.0722 * linear(c.b())
        };
        for preset in Accent::ALL {
            assert!(1.05 / (luminance(preset.color()) + 0.05) >= 4.5, "{} white button text", preset.name());
            assert!((luminance(preset.tint(0.16)) + 0.05) / (luminance(crate::INK) + 0.05) >= 4.5);
            assert_eq!(Accent::from_name(preset.name()), preset);
        }
        let ratio = |a: Color32, b: Color32| {
            let (la, lb) = (luminance(a), luminance(b));
            (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
        };
        assert!(ratio(crate::MUTED, Color32::WHITE) >= 4.5, "secondary text on white");
        assert!(ratio(crate::MUTED, crate::SIDEBAR_BG) >= 4.5, "secondary text on the sidebar");
        // True orange: WCAG non-text (icon) contrast; the label is medium weight.
        assert!(ratio(Color32::WHITE, crate::SWIPE_ARCHIVE) >= 3.0, "Archive swipe icon");
        assert!(ratio(Color32::WHITE, crate::SWIPE_DELETE) >= 4.5, "Delete swipe label");
    }
}
