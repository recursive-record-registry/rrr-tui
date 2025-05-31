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
    fn from(color: &'a TextColor) -> Self {
        Style::new().fg(color.fg.into()).bg(color.bg.into())
    }
}

impl From<TextColor> for Style {
    fn from(color: TextColor) -> Self {
        (&color).into()
    }
}

lazy_static! {
    static ref ENCODED_SRGB_TO_OKLCH: ColorConversion =
        kolor::ColorConversion::new(kolor::spaces::ENCODED_SRGB, kolor::spaces::OKLCH);
    static ref OKLCH_TO_ENCODED_SRGB: ColorConversion =
        kolor::ColorConversion::new(kolor::spaces::OKLCH, kolor::spaces::ENCODED_SRGB);
    static ref OKLCH_TO_OKLAB: ColorConversion =
        kolor::ColorConversion::new(kolor::spaces::OKLCH, kolor::spaces::OKLAB);
    static ref OKLAB_TO_OKLCH: ColorConversion =
        kolor::ColorConversion::new(kolor::spaces::OKLAB, kolor::spaces::OKLCH);
    static ref ENCODED_SRGB_TO_OKLAB: ColorConversion =
        kolor::ColorConversion::new(kolor::spaces::ENCODED_SRGB, kolor::spaces::OKLAB);
    static ref OKLAB_TO_ENCODED_SRGB: ColorConversion =
        kolor::ColorConversion::new(kolor::spaces::OKLAB, kolor::spaces::ENCODED_SRGB);
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

impl From<kolor::Vec3> for ColorOklch {
    fn from(vec: kolor::Vec3) -> Self {
        Self::new(vec.x, vec.y, vec.z / std::f32::consts::TAU)
    }
}

impl From<ColorOklch> for kolor::Vec3 {
    fn from(oklch: ColorOklch) -> Self {
        kolor::Vec3::new(
            oklch.lightness,
            oklch.chroma,
            oklch.hue * std::f32::consts::TAU,
        )
    }
}

impl From<ColorOklch> for Color {
    fn from(oklch: ColorOklch) -> Self {
        Self {
            oklch,
            rgb: OKLCH_TO_ENCODED_SRGB.convert(oklch.into()).into(),
        }
    }
}

impl From<Color> for ColorOklch {
    fn from(color: Color) -> Self {
        color.oklch
    }
}

impl From<ColorU8Rgb> for ColorOklch {
    fn from(u8rgb: ColorU8Rgb) -> Self {
        ENCODED_SRGB_TO_OKLCH.convert(u8rgb.into()).into()
    }
}

impl From<ColorOklab> for ColorOklch {
    fn from(oklab: ColorOklab) -> Self {
        OKLAB_TO_OKLCH.convert(oklab.into()).into()
    }
}

impl From<ColorOklch> for ratatui::style::Color {
    fn from(oklch: ColorOklch) -> Self {
        ColorU8Rgb::from(oklch).into()
    }
}

/// Represents a color in the Oklab color space.
/// Mainly used for color blending.
#[derive(Debug, Clone, Copy)]
pub struct ColorOklab {
    pub lightness: f32,
    pub chroma_a: f32,
    pub chroma_b: f32,
}

impl ColorOklab {
    pub fn new(lightness: f32, chroma_a: f32, chroma_b: f32) -> Self {
        Self {
            lightness,
            chroma_a,
            chroma_b,
        }
    }
}

impl Lerp for ColorOklab {
    fn lerp(&self, rhs: &Self, t: f32) -> Self {
        Self {
            lightness: Lerp::lerp(&self.lightness, &rhs.lightness, t),
            chroma_a: Lerp::lerp(&self.chroma_a, &rhs.chroma_a, t),
            chroma_b: Lerp::lerp(&self.chroma_b, &rhs.chroma_b, t),
        }
    }
}

impl From<kolor::Vec3> for ColorOklab {
    fn from(vec: kolor::Vec3) -> Self {
        Self::new(vec.x, vec.y, vec.z)
    }
}

impl From<ColorOklab> for kolor::Vec3 {
    fn from(oklab: ColorOklab) -> Self {
        kolor::Vec3::new(oklab.lightness, oklab.chroma_a, oklab.chroma_b)
    }
}

impl From<ColorOklab> for Color {
    fn from(oklab: ColorOklab) -> Self {
        Self {
            oklch: oklab.into(),
            rgb: oklab.into(),
        }
    }
}

impl From<Color> for ColorOklab {
    fn from(color: Color) -> Self {
        color.oklch.into()
    }
}

impl From<ColorU8Rgb> for ColorOklab {
    fn from(u8rgb: ColorU8Rgb) -> Self {
        ENCODED_SRGB_TO_OKLAB.convert(u8rgb.into()).into()
    }
}

impl From<ColorOklch> for ColorOklab {
    fn from(oklch: ColorOklch) -> Self {
        OKLCH_TO_OKLAB.convert(oklch.into()).into()
    }
}

impl From<ColorOklab> for ratatui::style::Color {
    fn from(oklab: ColorOklab) -> Self {
        ColorU8Rgb::from(oklab).into()
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

impl From<kolor::Vec3> for ColorU8Rgb {
    fn from(vec: kolor::Vec3) -> Self {
        Self::new_f32(vec.x, vec.y, vec.z)
    }
}

impl From<ColorU8Rgb> for kolor::Vec3 {
    fn from(u8rgb: ColorU8Rgb) -> Self {
        kolor::Vec3::new(
            u8rgb.red as f32 / 0xFF as f32,
            u8rgb.green as f32 / 0xFF as f32,
            u8rgb.blue as f32 / 0xFF as f32,
        )
    }
}

impl From<ColorU8Rgb> for Color {
    fn from(rgb: ColorU8Rgb) -> Self {
        Self {
            oklch: rgb.into(),
            rgb,
        }
    }
}

impl From<Color> for ColorU8Rgb {
    fn from(color: Color) -> Self {
        color.rgb
    }
}

impl From<ColorOklab> for ColorU8Rgb {
    fn from(oklab: ColorOklab) -> Self {
        OKLAB_TO_ENCODED_SRGB.convert(oklab.into()).into()
    }
}

impl From<ColorOklch> for ColorU8Rgb {
    fn from(oklch: ColorOklch) -> Self {
        OKLCH_TO_ENCODED_SRGB.convert(oklch.into()).into()
    }
}

impl From<ColorU8Rgb> for ratatui::style::Color {
    fn from(u8rgb: ColorU8Rgb) -> Self {
        Self::Rgb(u8rgb.red, u8rgb.green, u8rgb.blue)
    }
}

impl TryFrom<ratatui::style::Color> for ColorU8Rgb {
    type Error = ();

    fn try_from(color: ratatui::style::Color) -> Result<Self, Self::Error> {
        if let ratatui::style::Color::Rgb(r, g, b) = color {
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

    fn try_from(color: ratatui::style::Color) -> Result<Self, Self::Error> {
        Ok(ColorU8Rgb::try_from(color)?.into())
    }
}

impl From<Color> for ratatui::style::Color {
    fn from(color: Color) -> Self {
        color.rgb.into()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Blended<T> {
    pub color: T,
    pub alpha: f32,
}

impl<T> Blended<T> {
    pub fn new(color: T, alpha: f32) -> Self {
        Self { color, alpha }
    }

    pub fn cast<U>(self) -> Blended<U>
    where
        U: From<T>,
    {
        Blended {
            color: self.color.into(),
            alpha: self.alpha,
        }
    }
}

// TODO: Conflicts with `impl<T> From<T> for T`
// impl<T, U: From<T>> From<Blended<T>> for Blended<U> {
//     fn from(value: Blended<T>) -> Self {
//         Blended {
//             color: value.color.into(),
//             alpha: value.alpha,
//         }
//     }
// }

impl<T> From<T> for Blended<T> {
    fn from(color: T) -> Self {
        Self { color, alpha: 1.0 }
    }
}

pub trait Over<T> {
    type Output;
    fn over(&self, under: &T) -> Self::Output;
}

impl Over<ColorOklab> for Blended<ColorOklab> {
    type Output = ColorOklab;

    fn over(&self, under: &ColorOklab) -> Self::Output {
        Lerp::lerp(under, &self.color, self.alpha)
    }
}
