use super::color::{Color, Format, NAMES};
use super::error::Error;

#[derive(Debug, PartialEq)]
pub enum Arg {
    Index(usize),
    Literal(Color),
}

pub fn parse_arg(s: &str) -> Result<Arg, Error> {
    let s = s.trim();
    if s.starts_with('#') || s.contains('(') {
        return parse_literal(s).map(Arg::Literal);
    }
    if !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()) {
        return match s.parse::<usize>() {
            Ok(i) if i < 16 => Ok(Arg::Index(i)),
            _ => Err(Error::usage(format!(
                "palette index '{s}' is out of range (expected 0-15)"
            ))),
        };
    }
    match NAMES.iter().position(|n| n.eq_ignore_ascii_case(s)) {
        Some(i) => Ok(Arg::Index(i)),
        None => Err(Error::usage(format!(
            "unknown color '{s}' (expected a palette index 0-15, a name such as bright-red, \
             or a color literal such as #db8177)"
        ))),
    }
}

pub fn parse_format(s: &str) -> Result<Format, Error> {
    Format::from_name(s.trim()).ok_or_else(|| {
        Error::usage(format!(
            "invalid color format '{s}' (expected one of: HEX, RGB, HSL, HSV, OKLab, OKLCH)"
        ))
    })
}

pub fn parse_literal(s: &str) -> Result<Color, Error> {
    let s = s.trim();
    if let Some(digits) = s.strip_prefix('#') {
        return parse_hex(digits, s);
    }
    let malformed = || {
        Error::usage(format!(
            "malformed color '{s}' (expected e.g. #db8177 or rgb(219 129 119))"
        ))
    };
    let open = s.find('(').ok_or_else(malformed)?;
    if !s.ends_with(')') {
        return Err(malformed());
    }
    let name = s[..open].trim();
    let inner = &s[open + 1..s.len() - 1];
    if inner.contains('/') {
        return Err(Error::usage(
            "alpha is not supported (colors must be opaque)",
        ));
    }

    let mut args = [""; 3];
    let mut count = 0;
    for tok in inner
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|t| !t.is_empty())
    {
        if count < 3 {
            args[count] = tok;
        }
        count += 1;
    }
    if count != 3 {
        return Err(Error::usage(format!(
            "{}() expects 3 values, got {count} in '{s}'",
            name.to_ascii_lowercase()
        )));
    }
    let [a, b, c] = args;

    if name.eq_ignore_ascii_case("rgb") {
        Ok(Color::from_encoded(channel(a)?, channel(b)?, channel(c)?))
    } else if name.eq_ignore_ascii_case("hsl") {
        Ok(Color::from_hsl(
            hue(a)?,
            percent(b, "saturation")?,
            percent(c, "lightness")?,
        ))
    } else if name.eq_ignore_ascii_case("hsv") {
        Ok(Color::from_hsv(
            hue(a)?,
            percent(b, "saturation")?,
            percent(c, "value")?,
        ))
    } else if name.eq_ignore_ascii_case("oklab") {
        Ok(Color::from_oklab(
            lightness(a)?,
            number(b, "a")?,
            number(c, "b")?,
        ))
    } else if name.eq_ignore_ascii_case("oklch") {
        let chroma = number(b, "chroma")?;
        if chroma < 0.0 {
            return Err(Error::usage(format!(
                "invalid chroma '{b}' (must not be negative)"
            )));
        }
        Ok(Color::from_oklch(lightness(a)?, chroma, hue(c)?))
    } else {
        Err(Error::usage(format!(
            "unknown color notation '{}' (expected rgb, hsl, hsv, oklab, or oklch)",
            name.to_ascii_lowercase()
        )))
    }
}

fn parse_hex(digits: &str, whole: &str) -> Result<Color, Error> {
    let bad = || {
        Error::usage(format!(
            "malformed HEX color '{whole}' (expected #rgb or #rrggbb)"
        ))
    };
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(bad());
    }
    let pair = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).unwrap();
    let one = |i: usize| u8::from_str_radix(&digits[i..i + 1], 16).unwrap() * 17;
    match digits.len() {
        3 => Ok(Color::from_srgb8(one(0), one(1), one(2))),
        6 => Ok(Color::from_srgb8(pair(0), pair(2), pair(4))),
        4 | 8 => Err(Error::usage(
            "alpha is not supported (colors must be opaque)",
        )),
        _ => Err(bad()),
    }
}

fn number(tok: &str, what: &str) -> Result<f64, Error> {
    match tok.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(Error::usage(format!("invalid {what} value '{tok}'"))),
    }
}

fn hue(tok: &str) -> Result<f64, Error> {
    let bytes = tok.as_bytes();
    let digits = if bytes.len() >= 3 && bytes[bytes.len() - 3..].eq_ignore_ascii_case(b"deg") {
        &tok[..tok.len() - 3]
    } else {
        tok
    };
    number(digits, "hue (degrees)")
}

fn percent(tok: &str, what: &str) -> Result<f64, Error> {
    let v = number(tok.strip_suffix('%').unwrap_or(tok), what)?;
    if !(0.0..=100.0).contains(&v) {
        return Err(Error::usage(format!(
            "{what} '{tok}' is out of range (expected 0-100%)"
        )));
    }
    Ok(v / 100.0)
}

fn channel(tok: &str) -> Result<f64, Error> {
    let v = number(tok, "RGB channel")?;
    if !(0.0..=255.0).contains(&v) {
        return Err(Error::usage(format!(
            "RGB channel '{tok}' is out of range (expected 0-255)"
        )));
    }
    Ok(v / 255.0)
}

fn lightness(tok: &str) -> Result<f64, Error> {
    let v = match tok.strip_suffix('%') {
        Some(n) => number(n, "lightness")? / 100.0,
        None => number(tok, "lightness")?,
    };
    if !(0.0..=1.0).contains(&v) {
        return Err(Error::usage(format!(
            "lightness '{tok}' is out of range (expected 0%-100% or 0-1)"
        )));
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::color::format_color;

    fn hex(s: &str) -> String {
        format_color(parse_literal(s).unwrap(), Format::Hex).unwrap()
    }

    #[test]
    fn parses_hex_short_and_long() {
        assert_eq!(hex("#db8177"), "#db8177");
        assert_eq!(hex("#DB8177"), "#db8177");
        assert_eq!(hex("#fa0"), "#ffaa00");
        assert!(parse_literal("#12345").is_err());
        assert!(parse_literal("#gggggg").is_err());
        assert!(parse_literal("#").is_err());
    }

    #[test]
    fn rejects_alpha() {
        assert!(parse_literal("#db817780").is_err());
        assert!(parse_literal("#db87").is_err());
        assert!(parse_literal("rgb(1 2 3 / 50%)").is_err());
        assert!(parse_literal("rgb(1 2 3 4)").is_err());
    }

    #[test]
    fn accepts_whitespace_case_and_commas() {
        assert_eq!(hex("rgb(198 104 93)"), "#c6685d");
        assert_eq!(hex("  RGB( 198 ,104,  93 ) "), "#c6685d");
        assert_eq!(hex("HSL(7DEG 48% 57%)"), hex("hsl(7 48 57)"));
        assert_eq!(hex("oklch(100% 0 0)"), "#ffffff");
        assert_eq!(hex("oklch(1 0 0)"), "#ffffff");
        assert_eq!(hex("oklab(0% 0 0)"), "#000000");
    }

    #[test]
    fn rejects_malformed_expressions() {
        for bad in [
            "rgb(1 2)",
            "rgb(256 0 0)",
            "rgb(-1 0 0)",
            "rgb(a b c)",
            "rgb(nan 0 0)",
            "rgb 1 2 3",
            "rgb(1 2 3",
            "hsl(0 101% 50%)",
            "hsl(0rad 50% 50%)",
            "oklch(60% -0.1 25)",
            "oklch(120% 0.1 25)",
            "oklch(inf 0 0)",
            "foo(1 2 3)",
            "()",
            "hsl(éé 50% 50%)",
            "hsl(é 50% 50%)",
            "hsl(1é 50% 50%)",
        ] {
            assert!(parse_literal(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn resolves_arguments() {
        assert_eq!(parse_arg("09").unwrap(), Arg::Index(9));
        assert_eq!(parse_arg("9").unwrap(), Arg::Index(9));
        assert_eq!(parse_arg("bright-red").unwrap(), Arg::Index(9));
        assert_eq!(parse_arg("Bright-Red").unwrap(), Arg::Index(9));
        assert_eq!(parse_arg("white").unwrap(), Arg::Index(7));
        assert!(matches!(parse_arg("#db8177").unwrap(), Arg::Literal(_)));
        assert!(matches!(parse_arg("rgb(1 2 3)").unwrap(), Arg::Literal(_)));
    }

    #[test]
    fn rejects_bad_arguments_with_usage_errors() {
        for bad in ["16", "99999999999999999999", "bright-purple", "", "red!"] {
            let e = parse_arg(bad).unwrap_err();
            assert_eq!(e.exit_code(), 2, "{bad:?}");
        }
        assert!(
            parse_arg("bright-purple")
                .unwrap_err()
                .to_string()
                .contains("unknown color 'bright-purple'")
        );
    }

    #[test]
    fn parses_format_names() {
        assert_eq!(parse_format("hex").unwrap(), Format::Hex);
        assert_eq!(parse_format("OKLab").unwrap(), Format::Oklab);
        assert!(
            parse_format("OCLCH")
                .unwrap_err()
                .to_string()
                .contains("invalid color format 'OCLCH'")
        );
    }

    #[test]
    fn text_round_trips_through_every_notation() {
        let steps: Vec<u8> = (0..=255u16).step_by(17).map(|v| v as u8).collect();
        for &r in &steps {
            for &g in &steps {
                for &b in &steps {
                    let c = Color::from_srgb8(r, g, b);
                    for f in Format::ALL {
                        let text = format_color(c, f).unwrap();
                        let back = parse_literal(&text).unwrap().srgb8().unwrap();
                        let off = |x: u8, y: u8| (x as i32 - y as i32).abs();
                        let tol = if matches!(f, Format::Hex | Format::Rgb) {
                            0
                        } else {
                            1
                        };
                        assert!(
                            off(back.0, r) <= tol && off(back.1, g) <= tol && off(back.2, b) <= tol,
                            "{f:?} round trip of ({r},{g},{b}) gave {text} -> {back:?}"
                        );
                    }
                }
            }
        }
    }
}
