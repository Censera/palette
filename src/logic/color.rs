use super::error::Error;

pub const ANSI_RESET: &str = "\x1b[0m";

pub const NAMES: [&str; 16] = [
    "black",
    "red",
    "green",
    "yellow",
    "blue",
    "magenta",
    "cyan",
    "white",
    "bright-black",
    "bright-red",
    "bright-green",
    "bright-yellow",
    "bright-blue",
    "bright-magenta",
    "bright-cyan",
    "bright-white",
];

const GAMUT_EPS: f64 = 5e-4;

const ACHROMATIC_EPS: f64 = 1e-4;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

#[derive(Clone, Debug)]
pub struct Palette {
    pub colors: [Color; 16],
}

fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f64) -> f64 {
    if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

impl Color {
    pub fn from_encoded(r: f64, g: f64, b: f64) -> Self {
        Color {
            r: srgb_to_linear(r),
            g: srgb_to_linear(g),
            b: srgb_to_linear(b),
        }
    }

    pub fn from_srgb8(r: u8, g: u8, b: u8) -> Self {
        Self::from_encoded(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0)
    }

    pub fn encoded(self) -> Option<[f64; 3]> {
        let ch = [self.r, self.g, self.b];
        if ch
            .iter()
            .any(|c| !(-GAMUT_EPS..=1.0 + GAMUT_EPS).contains(c))
        {
            return None;
        }
        Some(ch.map(|c| linear_to_srgb(c.clamp(0.0, 1.0))))
    }

    pub fn srgb8(self) -> Option<(u8, u8, u8)> {
        let [r, g, b] = self.encoded()?;
        let q = |c: f64| (c * 255.0).round() as u8;
        Some((q(r), q(g), q(b)))
    }

    pub fn from_hsl(h: f64, s: f64, l: f64) -> Self {
        let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
        let [r, g, b] = hue_to_rgb(h, c, l - c / 2.0);
        Self::from_encoded(r, g, b)
    }

    pub fn from_hsv(h: f64, s: f64, v: f64) -> Self {
        let c = v * s;
        let [r, g, b] = hue_to_rgb(h, c, v - c);
        Self::from_encoded(r, g, b)
    }

    pub fn to_hsl(self) -> Option<(f64, f64, f64)> {
        let [r, g, b] = self.encoded()?;
        let (max, min) = (r.max(g).max(b), r.min(g).min(b));
        let delta = max - min;
        let l = (max + min) / 2.0;
        let s = if delta < ACHROMATIC_EPS {
            0.0
        } else {
            delta / (1.0 - (2.0 * l - 1.0).abs())
        };
        Some((hue_of(r, g, b, max, delta), s, l))
    }

    pub fn to_hsv(self) -> Option<(f64, f64, f64)> {
        let [r, g, b] = self.encoded()?;
        let max = r.max(g).max(b);
        let delta = max - r.min(g).min(b);
        let s = if max <= 0.0 { 0.0 } else { delta / max };
        Some((hue_of(r, g, b, max, delta), s, max))
    }

    pub fn from_oklab(l: f64, a: f64, b: f64) -> Self {
        let l_ = l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
        let m_ = l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
        let s_ = l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
        let (l3, m3, s3) = (l_ * l_ * l_, m_ * m_ * m_, s_ * s_ * s_);
        Color {
            r: 4.076_741_662_1 * l3 - 3.307_711_591_3 * m3 + 0.230_969_929_2 * s3,
            g: -1.268_438_004_6 * l3 + 2.609_757_401_1 * m3 - 0.341_319_396_5 * s3,
            b: -0.004_196_086_3 * l3 - 0.703_418_614_7 * m3 + 1.707_614_701_0 * s3,
        }
    }

    pub fn to_oklab(self) -> (f64, f64, f64) {
        let l = 0.412_221_470_8 * self.r + 0.536_332_536_3 * self.g + 0.051_445_992_9 * self.b;
        let m = 0.211_903_498_2 * self.r + 0.680_699_545_1 * self.g + 0.107_396_956_6 * self.b;
        let s = 0.088_302_461_9 * self.r + 0.281_718_837_6 * self.g + 0.629_978_700_5 * self.b;
        let (l_, m_, s_) = (l.cbrt(), m.cbrt(), s.cbrt());
        (
            0.210_454_255_3 * l_ + 0.793_617_785_0 * m_ - 0.004_072_046_8 * s_,
            1.977_998_495_1 * l_ - 2.428_592_205_0 * m_ + 0.450_593_709_9 * s_,
            0.025_904_037_1 * l_ + 0.782_771_766_2 * m_ - 0.808_675_766_0 * s_,
        )
    }

    pub fn from_oklch(l: f64, c: f64, h: f64) -> Self {
        let (sin, cos) = h.to_radians().sin_cos();
        Self::from_oklab(l, c * cos, c * sin)
    }

    pub fn to_oklch(self) -> (f64, f64, f64) {
        let (l, a, b) = self.to_oklab();
        let c = a.hypot(b);
        let h = if c < ACHROMATIC_EPS {
            0.0
        } else {
            b.atan2(a).to_degrees().rem_euclid(360.0)
        };
        (l, c, h)
    }
}

fn hue_of(r: f64, g: f64, b: f64, max: f64, delta: f64) -> f64 {
    if delta < ACHROMATIC_EPS {
        return 0.0;
    }
    let h = if max == r {
        ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        (b - r) / delta + 2.0
    } else {
        (r - g) / delta + 4.0
    };
    (h * 60.0).rem_euclid(360.0)
}

fn hue_to_rgb(h: f64, c: f64, m: f64) -> [f64; 3] {
    let h6 = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (h6 % 2.0 - 1.0).abs());
    let (r, g, b) = match h6 as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Rgb,
    Hex,
    Hsl,
    Hsv,
    Oklch,
    Oklab,
}

impl Format {
    pub const ALL: [Format; 6] = [
        Format::Rgb,
        Format::Hex,
        Format::Hsl,
        Format::Hsv,
        Format::Oklch,
        Format::Oklab,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Format::Rgb => "RGB",
            Format::Hex => "HEX",
            Format::Hsl => "HSL",
            Format::Hsv => "HSV",
            Format::Oklch => "OKLCH",
            Format::Oklab => "OKLab",
        }
    }

    pub fn from_name(s: &str) -> Option<Format> {
        Format::ALL
            .into_iter()
            .find(|f| f.label().eq_ignore_ascii_case(s))
    }
}

fn num(x: f64, decimals: usize) -> String {
    let s = format!("{x:.decimals$}");
    let s = if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        &s
    };
    if s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

pub fn format_color(c: Color, f: Format) -> Result<String, Error> {
    let gamut = || {
        Error::runtime(format!(
            "color is outside the sRGB gamut and cannot be written as {} (use OKLab or OKLCH)",
            f.label()
        ))
    };
    Ok(match f {
        Format::Hex => {
            let (r, g, b) = c.srgb8().ok_or_else(gamut)?;
            format!("#{r:02x}{g:02x}{b:02x}")
        }
        Format::Rgb => {
            let (r, g, b) = c.srgb8().ok_or_else(gamut)?;
            format!("rgb({r} {g} {b})")
        }
        Format::Hsl => {
            let (h, s, l) = c.to_hsl().ok_or_else(gamut)?;
            format!(
                "hsl({}deg {}% {}%)",
                num(h, 1),
                num(s * 100.0, 1),
                num(l * 100.0, 1)
            )
        }
        Format::Hsv => {
            let (h, s, v) = c.to_hsv().ok_or_else(gamut)?;
            format!(
                "hsv({}deg {}% {}%)",
                num(h, 1),
                num(s * 100.0, 1),
                num(v * 100.0, 1)
            )
        }
        Format::Oklab => {
            let (l, a, b) = c.to_oklab();
            format!("oklab({}% {} {})", num(l * 100.0, 3), num(a, 5), num(b, 5))
        }
        Format::Oklch => {
            let (l, ch, h) = c.to_oklch();
            format!("oklch({}% {} {})", num(l * 100.0, 3), num(ch, 5), num(h, 3))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    fn grid() -> impl Iterator<Item = (u8, u8, u8)> {
        let steps: Vec<u8> = (0..=255u16)
            .step_by(15)
            .map(|v| v as u8)
            .chain([255])
            .collect();
        let mut v = Vec::new();
        for &r in &steps {
            for &g in &steps {
                for &b in &steps {
                    v.push((r, g, b));
                }
            }
        }
        v.into_iter()
    }

    #[test]
    fn srgb8_round_trip_is_exact() {
        for (r, g, b) in (0..=255u8).map(|v| (v, 255 - v, v / 2)) {
            assert_eq!(Color::from_srgb8(r, g, b).srgb8(), Some((r, g, b)));
        }
    }

    #[test]
    fn oklab_matches_reference_values_for_red() {
        let red = Color::from_srgb8(255, 0, 0);
        let (l, a, b) = red.to_oklab();
        assert!(close(l, 0.62796, 1e-4) && close(a, 0.22486, 1e-4) && close(b, 0.12585, 1e-4));
        let (l, c, h) = red.to_oklch();
        assert!(close(l, 0.62796, 1e-4) && close(c, 0.2577, 1e-4) && close(h, 29.23, 0.05));
    }

    #[test]
    fn white_and_black_are_achromatic() {
        let white = Color::from_srgb8(255, 255, 255);
        assert_eq!(
            format_color(white, Format::Oklch).unwrap(),
            "oklch(100% 0 0)"
        );
        assert_eq!(
            format_color(white, Format::Hsl).unwrap(),
            "hsl(0deg 0% 100%)"
        );
        assert_eq!(
            format_color(white, Format::Hsv).unwrap(),
            "hsv(0deg 0% 100%)"
        );
        let black = Color::from_srgb8(0, 0, 0);
        assert_eq!(format_color(black, Format::Oklch).unwrap(), "oklch(0% 0 0)");
        let gray = Color::from_srgb8(128, 128, 128);
        assert_eq!(gray.to_hsl().unwrap().1, 0.0);
    }

    #[test]
    fn hsl_hsv_known_values() {
        let c = Color::from_srgb8(219, 129, 119);
        assert_eq!(
            format_color(c, Format::Hsl).unwrap(),
            "hsl(6deg 58.1% 66.3%)"
        );
        assert_eq!(
            format_color(c, Format::Hsv).unwrap(),
            "hsv(6deg 45.7% 85.9%)"
        );
        assert!(close(
            Color::from_srgb8(0, 255, 0).to_hsl().unwrap().0,
            120.0,
            1e-9
        ));
        assert!(close(
            Color::from_srgb8(0, 0, 255).to_hsv().unwrap().0,
            240.0,
            1e-9
        ));
    }

    #[test]
    fn numeric_round_trips_keep_precision() {
        for (r, g, b) in grid() {
            let c = Color::from_srgb8(r, g, b);
            let (h, s, l) = c.to_hsl().unwrap();
            let (h2, s2, v2) = c.to_hsv().unwrap();
            let (ol, oa, ob) = c.to_oklab();
            let (cl, cc, ch) = c.to_oklch();
            for back in [
                Color::from_hsl(h, s, l),
                Color::from_hsv(h2, s2, v2),
                Color::from_oklab(ol, oa, ob),
                Color::from_oklch(cl, cc, ch),
            ] {
                assert!(
                    close(back.r, c.r, 1e-6)
                        && close(back.g, c.g, 1e-6)
                        && close(back.b, c.b, 1e-6),
                    "round trip drifted for {r},{g},{b}"
                );
            }
        }
    }

    #[test]
    fn hue_wraps() {
        let a = Color::from_oklch(0.6, 0.1, 25.0);
        let b = Color::from_oklch(0.6, 0.1, 25.0 + 360.0);
        assert!(close(a.r, b.r, 1e-9) && close(a.g, b.g, 1e-9) && close(a.b, b.b, 1e-9));
        let c = Color::from_hsl(-60.0, 1.0, 0.5);
        let d = Color::from_hsl(300.0, 1.0, 0.5);
        assert!(close(c.r, d.r, 1e-9) && close(c.g, d.g, 1e-9) && close(c.b, d.b, 1e-9));
    }

    #[test]
    fn out_of_gamut_is_reported_not_clamped() {
        let c = Color::from_oklch(0.6, 0.4, 25.0);
        assert!(c.srgb8().is_none());
        assert!(format_color(c, Format::Hex).is_err());
        assert!(format_color(c, Format::Hsl).is_err());
        assert!(format_color(c, Format::Oklch).is_ok());
        assert!(format_color(c, Format::Oklab).is_ok());
    }

    #[test]
    fn number_formatting_trims() {
        assert_eq!(num(58.30, 1), "58.3");
        assert_eq!(num(100.0, 2), "100");
        assert_eq!(num(-0.00001, 4), "0");
        assert_eq!(num(0.25, 4), "0.25");
    }

    #[test]
    fn format_names_are_case_insensitive() {
        assert_eq!(Format::from_name("oklab"), Some(Format::Oklab));
        assert_eq!(Format::from_name("OKLCH"), Some(Format::Oklch));
        assert_eq!(Format::from_name("hex"), Some(Format::Hex));
        assert_eq!(Format::from_name("OCLCH"), None);
    }
}
