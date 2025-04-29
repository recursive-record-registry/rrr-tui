use kolor::ColorConversion;
use lazy_static::lazy_static;
use ratatui::style::Style;

pub trait Lerp {
    fn lerp(&self, rhs: &Self, t: f32) -> Self;
}

impl Lerp for f32 {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        (1.0 - t) * *self + t * *rhs
    }
}

#[derive(Debug, Clone)]
pub struct TextColor {
    pub fg: Color,
    pub bg: Color,
}

impl Default for TextColor {
    fn default() -> Self {
        Self {
            fg: ColorU8Rgb::new(0xFF, 0xFF, 0xFF).into(),
            bg: ColorU8Rgb::new(0x00, 0x00, 0x00).into(),
        }
    }
}

impl TextColor {
    pub fn invert(self) -> Self {
        Self {
            fg: self.bg,
            bg: self.fg,
        }
    }

    pub fn fg(self, fg: impl Into<Color>) -> Self {
        Self {
            fg: fg.into(),
            ..self
        }
    }

    pub fn bg(self, bg: impl Into<Color>) -> Self {
        Self {
            bg: bg.into(),
            ..self
        }
    }
}

impl Lerp for TextColor {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            fg: Lerp::lerp(&self.fg, &rhs.fg, t),
            bg: Lerp::lerp(&self.bg, &rhs.bg, t),
        }
    }
}

impl<'a> From<&'a TextColor> for Style {
    fn from(value: &'a TextColor) -> Self {
        Style::new().fg(value.fg.into()).bg(value.bg.into())
    }
}

impl From<TextColor> for Style {
    fn from(value: TextColor) -> Self {
        (&value).into()
    }
}

lazy_static! {
    static ref ENCODED_SRGB_TO_OKLCH: ColorConversion =
        kolor::ColorConversion::new(kolor::spaces::ENCODED_SRGB, kolor::spaces::OKLCH);
    static ref OKLCH_TO_ENCODED_SRGB: ColorConversion =
        kolor::ColorConversion::new(kolor::spaces::OKLCH, kolor::spaces::ENCODED_SRGB);
}

/// Represents a color in the Oklch color space.
/// To pick colors, use https://oklch.com/
#[derive(Debug, Clone, Copy)]
pub struct ColorOklch {
    /// Lightness in the range [0; 1].
    pub lightness: f32,
    /// Chroma in the range [0; 1].
    pub chroma: f32,
    /// Hue with a period of 1, starting with red at 0. Range (-Inf; Inf).
    pub hue: f32,
}

impl ColorOklch {
    pub fn new(lightness: f32, chroma: f32, hue: f32) -> Self {
        Self {
            lightness,
            chroma,
            hue,
        }
    }
}

impl Lerp for ColorOklch {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            lightness: Lerp::lerp(&self.lightness, &rhs.lightness, t),
            chroma: Lerp::lerp(&self.chroma, &rhs.chroma, t),
            // Interpolate in the direction that is closer.
            hue: {
                // Take the upper fractional part of the hue.
                let lhs_hue = self.hue - self.hue.floor();
                let rhs_hue = rhs.hue - rhs.hue.floor();
                let hue_diff = rhs_hue - lhs_hue;

                if hue_diff.abs() <= 0.5 {
                    Lerp::lerp(&self.hue, &rhs.hue, t)
                } else {
                    let (first, second) = if hue_diff > 0.0 {
                        (lhs_hue + 1.0, rhs_hue)
                    } else {
                        (lhs_hue, rhs_hue + 1.0)
                    };

                    Lerp::lerp(&first, &second, t)
                }
            },
        }
    }
}

impl From<ColorOklch> for Color {
    fn from(
        oklch @ ColorOklch {
            lightness,
            chroma,
            hue,
        }: ColorOklch,
    ) -> Self {
        let rgb = OKLCH_TO_ENCODED_SRGB.convert(kolor::Vec3 {
            x: lightness,
            y: chroma,
            z: hue * std::f32::consts::TAU,
        });
        Self {
            oklch,
            rgb: ColorU8Rgb::new_f32(rgb.x, rgb.y, rgb.z),
        }
    }
}

impl From<ColorOklch> for ratatui::style::Color {
    fn from(value: ColorOklch) -> Self {
        Color::from(value).into()
    }
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ColorU8Rgb {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl ColorU8Rgb {
    pub fn new(red: u8, green: u8, blue: u8) -> Self {
        Self { red, green, blue }
    }

    pub fn new_f32(red: f32, green: f32, blue: f32) -> Self {
        Self::new(
            (red * 0xFF as f32) as u8,
            (green * 0xFF as f32) as u8,
            (blue * 0xFF as f32) as u8,
        )
    }
}

impl From<ColorU8Rgb> for Color {
    fn from(rgb @ ColorU8Rgb { red, green, blue }: ColorU8Rgb) -> Self {
        let oklch = ENCODED_SRGB_TO_OKLCH.convert(kolor::Vec3::new(
            red as f32 / 0xFF as f32,
            green as f32 / 0xFF as f32,
            blue as f32 / 0xFF as f32,
        ));
        Self {
            oklch: ColorOklch::new(oklch.x, oklch.y, oklch.z / std::f32::consts::TAU),
            rgb,
        }
    }
}

impl From<ColorU8Rgb> for ratatui::style::Color {
    fn from(value: ColorU8Rgb) -> Self {
        Color::from(value).into()
    }
}

impl TryFrom<ratatui::style::Color> for ColorU8Rgb {
    type Error = ();

    fn try_from(value: ratatui::style::Color) -> Result<Self, Self::Error> {
        if let ratatui::style::Color::Rgb(r, g, b) = value {
            Ok(ColorU8Rgb::new(r, g, b))
        } else {
            Err(())
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    oklch: ColorOklch,
    rgb: ColorU8Rgb,
}

impl Lerp for Color {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        Lerp::lerp(&self.oklch, &rhs.oklch, t).into()
    }
}

impl TryFrom<ratatui::style::Color> for Color {
    type Error = ();

    fn try_from(value: ratatui::style::Color) -> Result<Self, Self::Error> {
        use ratatui::style::Color::*;
        match value {
            Rgb(r, g, b) => Ok(ColorU8Rgb::new(r, g, b).into()),
            _ => Err(()),
        }
    }
}

impl From<Color> for ratatui::style::Color {
    fn from(value: Color) -> Self {
        Self::Rgb(value.rgb.red, value.rgb.green, value.rgb.blue)
    }
}
