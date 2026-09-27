//! Neutral RGBA and the WCAG 2.x contrast maths every calculated colour is built on.

/// A colour as 8-bit RGBA. Neutral on purpose: each Client converts it into its own colour
/// type (`gpui` RGBA on the Desktop, a ratatui `Color` in the TUI).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    /// 255 is opaque. Only the translucent calculated colours (borders, hover, scrim, shadows)
    /// carry less; [`Rgba::over`] flattens one for a Client that cannot blend.
    pub a: u8,
}

impl Rgba {
    /// An opaque colour.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    /// The same colour at `opacity` (0.0–1.0).
    pub fn with_opacity(self, opacity: f32) -> Self {
        Self {
            a: to_channel(opacity * 255.0),
            ..self
        }
    }

    /// Composites this colour over an opaque `ground`, giving the opaque colour it shows as.
    pub fn over(self, ground: Self) -> Self {
        mix(ground, Self { a: 255, ..self }, f32::from(self.a) / 255.0)
    }

    /// WCAG 2.x relative luminance of the opaque colour.
    pub fn relative_luminance(self) -> f32 {
        let linear = |channel: u8| {
            let c = f32::from(channel) / 255.0;
            if c <= 0.040_45 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(self.r) + 0.7152 * linear(self.g) + 0.0722 * linear(self.b)
    }
}

/// `from` moved `amount` (0.0–1.0) of the way toward `to`, as opaque sRGB.
pub fn mix(from: Rgba, to: Rgba, amount: f32) -> Rgba {
    let channel = |a: u8, b: u8| to_channel(f32::from(a) + (f32::from(b) - f32::from(a)) * amount);
    Rgba::rgb(
        channel(from.r, to.r),
        channel(from.g, to.g),
        channel(from.b, to.b),
    )
}

/// WCAG 2.x contrast ratio between two opaque colours, from 1.0 to 21.0.
pub fn contrast_ratio(a: Rgba, b: Rgba) -> f32 {
    let (la, lb) = (a.relative_luminance(), b.relative_luminance());
    let (light, dark) = if la > lb { (la, lb) } else { (lb, la) };
    (light + 0.05) / (dark + 0.05)
}

// Clamped to 0.0..=255.0 first, so the cast neither truncates nor wraps.
fn to_channel(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_on_white_is_21() {
        assert!((contrast_ratio(Rgba::BLACK, Rgba::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast_ratio(Rgba::WHITE, Rgba::WHITE) - 1.0).abs() < 0.001);
    }

    #[test]
    fn mix_ends_are_the_inputs() {
        let a = Rgba::rgb(10, 20, 30);
        let b = Rgba::rgb(200, 100, 0);
        assert_eq!(mix(a, b, 0.0), a);
        assert_eq!(mix(a, b, 1.0), b);
        assert_eq!(mix(Rgba::BLACK, Rgba::WHITE, 0.5), Rgba::rgb(128, 128, 128));
    }

    #[test]
    fn over_flattens_translucency() {
        let ink = Rgba::BLACK.with_opacity(0.5);
        assert_eq!(ink.over(Rgba::WHITE), Rgba::rgb(127, 127, 127));
        assert_eq!(Rgba::BLACK.over(Rgba::WHITE), Rgba::BLACK);
    }
}
