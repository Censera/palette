use crate::logic::color::{ANSI_RESET, Color, Format, NAMES, Palette, format_color};
use crate::logic::error::Error;
use std::fmt::Write;
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Depth {
    Console,
    Basic,
    Indexed,
    True,
}

#[derive(Clone, Copy)]
pub struct Style {
    pub color: bool,
    pub depth: Depth,
}

impl Style {
    fn literal_ok(self) -> bool {
        matches!(self.depth, Depth::True | Depth::Indexed)
    }
}

const CELL: &str = "   ";
const SPACES: &str = "                ";

const LABELS: [&str; 16] = [
    "[00]", "[01]", "[02]", "[03]", "[04]", "[05]", "[06]", "[07]", "[08]", "[09]", "[10]", "[11]",
    "[12]", "[13]", "[14]", "[15]",
];

const FG: [&str; 16] = [
    "30", "31", "32", "33", "34", "35", "36", "37", "90", "91", "92", "93", "94", "95", "96", "97",
];

const BG: [&str; 16] = [
    "40", "41", "42", "43", "44", "45", "46", "47", "100", "101", "102", "103", "104", "105",
    "106", "107",
];

#[derive(Clone, Copy)]
enum Bg {
    Index(usize),
    Rgb(u8, u8, u8),
}

const LEVELS: [i32; 6] = [0, 95, 135, 175, 215, 255];

fn nearest_level(v: i32) -> usize {
    let mut best = 0;
    for i in 1..6 {
        if (LEVELS[i] - v).abs() < (LEVELS[best] - v).abs() {
            best = i;
        }
    }
    best
}

fn to_256(r: u8, g: u8, b: u8) -> u8 {
    let (r, g, b) = (r as i32, g as i32, b as i32);
    let (ri, gi, bi) = (nearest_level(r), nearest_level(g), nearest_level(b));
    let dist = |cr: i32, cg: i32, cb: i32| (r - cr).pow(2) + (g - cg).pow(2) + (b - cb).pow(2);
    let cube = dist(LEVELS[ri], LEVELS[gi], LEVELS[bi]);
    let step = (((r + g + b) / 3 - 8 + 5) / 10).clamp(0, 23);
    let level = 8 + 10 * step;
    if dist(level, level, level) < cube {
        (232 + step) as u8
    } else {
        (16 + 36 * ri + 6 * gi + bi) as u8
    }
}

fn push_bg(out: &mut String, bg: Bg, depth: Depth) {
    match bg {
        Bg::Index(i) if i >= 8 && depth == Depth::Console => {
            out.push_str("5;");
            out.push_str(BG[i - 8]);
        }
        Bg::Index(i) => out.push_str(BG[i]),
        Bg::Rgb(r, g, b) if depth == Depth::Indexed => {
            let _ = write!(out, "48;5;{}", to_256(r, g, b));
        }
        Bg::Rgb(r, g, b) => {
            let _ = write!(out, "48;2;{r};{g};{b}");
        }
    }
}

fn push_cell(out: &mut String, bg: Bg, plain: &str, st: Style) {
    if st.color {
        out.push_str("\x1b[");
        push_bg(out, bg, st.depth);
        out.push('m');
        out.push_str(CELL);
        out.push_str(ANSI_RESET);
    } else {
        out.push_str(plain);
    }
}

fn push_cells(out: &mut String, bg: Bg, plain: &str, st: Style) {
    for _ in 0..3 {
        push_cell(out, bg, plain, st);
    }
}

fn push_name(out: &mut String, i: usize, width: usize, st: Style) {
    if st.color {
        out.push_str("\x1b[");
        out.push_str(FG[i]);
        out.push('m');
        out.push_str(NAMES[i]);
        out.push_str(ANSI_RESET);
    } else {
        out.push_str(NAMES[i]);
    }
    out.push_str(&SPACES[..width.saturating_sub(NAMES[i].len())]);
}

fn push_hex(out: &mut String, c: Color) -> Result<(), Error> {
    let (r, g, b) = c.srgb8().ok_or_else(|| {
        Error::runtime(
            "color is outside the sRGB gamut and cannot be written as HEX (use OKLab or OKLCH)",
        )
    })?;
    let _ = write!(out, "#{r:02x}{g:02x}{b:02x}");
    Ok(())
}

fn index_row(out: &mut String, range: Range<usize>, st: Style) {
    for i in range {
        push_cell(out, Bg::Index(i), LABELS[i], st);
    }
}

pub fn preview(st: Style) -> String {
    let mut out = String::with_capacity(if st.color { 512 } else { 72 });
    index_row(&mut out, 0..8, st);
    out.push('\n');
    index_row(&mut out, 8..16, st);
    out.push('\n');
    out
}

pub fn expand(p: &Palette, bright: bool, st: Style) -> Result<String, Error> {
    let mut out = String::with_capacity(if st.color { 1536 } else { 512 });
    index_row(&mut out, 0..8, st);
    out.push('\n');
    if bright {
        index_row(&mut out, 8..16, st);
        out.push('\n');
    }
    out.push('\n');
    for i in 0..8 {
        push_name(&mut out, i, 9, st);
        push_hex(&mut out, p.colors[i])?;
        if bright {
            out.push_str("   ");
            push_name(&mut out, i + 8, 16, st);
            push_hex(&mut out, p.colors[i + 8])?;
        }
        out.push('\n');
    }
    Ok(out)
}

pub fn show(color: Color, index: Option<usize>, convert: bool, st: Style) -> Result<String, Error> {
    let (r, g, b) = color.srgb8().ok_or_else(|| {
        Error::runtime("color is outside the sRGB gamut and cannot be displayed (use `palette let` for OKLab or OKLCH)")
    })?;
    let swatch = match index {
        Some(i) => Some((Bg::Index(i), LABELS[i])),
        None if st.color && st.literal_ok() => Some((Bg::Rgb(r, g, b), "")),
        None => None,
    };

    let mut out = String::with_capacity(512);
    if !convert {
        if let Some((bg, plain)) = swatch {
            push_cells(&mut out, bg, plain, st);
            out.push_str("  ");
        }
        push_hex(&mut out, color)?;
        out.push('\n');
        return Ok(out);
    }
    if let Some((bg, plain)) = swatch {
        out.push('\n');
        push_cells(&mut out, bg, plain, st);
        out.push_str("\n\n");
    }
    for f in Format::ALL {
        let _ = write!(out, "{:<9}", f.label());
        out.push_str(&format_color(color, f)?);
        out.push('\n');
    }
    Ok(out)
}

pub fn swatch(color: Color, index: Option<usize>, st: Style) -> Option<String> {
    if !st.color {
        return None;
    }
    let bg = match index {
        Some(i) => Bg::Index(i),
        None if st.literal_ok() => {
            let (r, g, b) = color.srgb8()?;
            Bg::Rgb(r, g, b)
        }
        None => return None,
    };
    let mut out = String::with_capacity(96);
    push_cells(&mut out, bg, "", st);
    Some(out)
}

pub fn full(bright: bool, st: Style) -> String {
    let mut out = String::with_capacity(if st.color { 8192 } else { 256 });
    fg_block(&mut out, None, st);
    if st.color {
        for bg in 0..if bright { 16 } else { 8 } {
            out.push('\n');
            fg_block(&mut out, Some(bg), st);
        }
    }
    out
}

fn fg_block(out: &mut String, bg: Option<usize>, st: Style) {
    fg_line(out, 0..8, bg, st);
    out.push('\n');
    fg_line(out, 8..16, bg, st);
    out.push('\n');
}

fn fg_line(out: &mut String, range: Range<usize>, bg: Option<usize>, st: Style) {
    for i in range {
        if st.color {
            out.push_str("\x1b[");
            out.push_str(FG[i]);
            if let Some(b) = bg {
                out.push(';');
                push_bg(out, Bg::Index(b), st.depth);
            }
            out.push('m');
            out.push_str(NAMES[i]);
            out.push(' ');
            out.push_str(ANSI_RESET);
        } else {
            out.push_str(NAMES[i]);
            out.push(' ');
        }
    }
    if !st.color {
        out.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAIN: Style = Style {
        color: false,
        depth: Depth::True,
    };
    const COLOR: Style = Style {
        color: true,
        depth: Depth::True,
    };
    const INDEXED: Style = Style {
        color: true,
        depth: Depth::Indexed,
    };
    const BASIC: Style = Style {
        color: true,
        depth: Depth::Basic,
    };
    const CONSOLE: Style = Style {
        color: true,
        depth: Depth::Console,
    };

    fn sample() -> Palette {
        let hexes = [
            "242120", "c6685d", "839c7a", "cb9168", "67808a", "a27886", "769f93", "e5dec9",
            "36312f", "db8177", "9cb793", "e2aa82", "809ba6", "bc919f", "92b9ad", "f4eedc",
        ];
        Palette {
            colors: std::array::from_fn(|i| {
                let v = u32::from_str_radix(hexes[i], 16).unwrap();
                Color::from_srgb8((v >> 16) as u8, (v >> 8) as u8, v as u8)
            }),
        }
    }

    #[test]
    fn plain_preview() {
        assert_eq!(
            preview(PLAIN),
            "[00][01][02][03][04][05][06][07]\n[08][09][10][11][12][13][14][15]\n"
        );
    }

    #[test]
    fn colored_swatches_are_three_painted_spaces() {
        let out = preview(COLOR);
        assert!(out.contains("\x1b[40m   \x1b[0m"), "normal slots use 40-47");
        assert!(
            out.contains("\x1b[101m   \x1b[0m"),
            "bright slots use 100-107"
        );
        assert!(
            !out.contains("[00]") && !out.contains("38;2"),
            "no labels or foreground"
        );
        let lit = show(Color::from_srgb8(219, 129, 119), None, false, COLOR).unwrap();
        assert!(
            lit.contains("\x1b[48;2;219;129;119m   \x1b[0m"),
            "literals use 24-bit color"
        );
        assert_eq!(lit.matches("\x1b[48;2;").count(), 3, "three swatches");
        assert!(lit.ends_with("  #db8177\n"));
    }

    #[test]
    fn plain_expand_has_no_bright_colors_and_no_header() {
        let expected = "\
[00][01][02][03][04][05][06][07]

black    #242120
red      #c6685d
green    #839c7a
yellow   #cb9168
blue     #67808a
magenta  #a27886
cyan     #769f93
white    #e5dec9
";
        assert_eq!(expand(&sample(), false, PLAIN).unwrap(), expected);
    }

    #[test]
    fn plain_expand_bright_adds_the_bright_colors() {
        let expected = "\
[00][01][02][03][04][05][06][07]
[08][09][10][11][12][13][14][15]

black    #242120   bright-black    #36312f
red      #c6685d   bright-red      #db8177
green    #839c7a   bright-green    #9cb793
yellow   #cb9168   bright-yellow   #e2aa82
blue     #67808a   bright-blue     #809ba6
magenta  #a27886   bright-magenta  #bc919f
cyan     #769f93   bright-cyan     #92b9ad
white    #e5dec9   bright-white    #f4eedc
";
        assert_eq!(expand(&sample(), true, PLAIN).unwrap(), expected);
    }

    #[test]
    fn expand_colors_the_names() {
        let normal = expand(&sample(), false, COLOR).unwrap();
        assert!(
            normal.contains("\x1b[31mred\x1b[0m  "),
            "name colored, padding outside the escape"
        );
        assert!(!normal.contains("\x1b[91m"), "no bright colors by default");
        let both = expand(&sample(), true, COLOR).unwrap();
        assert!(both.contains("\x1b[91mbright-red\x1b[0m"));
        assert!(
            !both.contains("Normal") && !both.contains("Bright"),
            "no headers"
        );
    }

    #[test]
    fn show_plain_and_convert() {
        let c = sample().colors[9];
        assert_eq!(
            show(c, Some(9), false, PLAIN).unwrap(),
            "[09][09][09]  #db8177\n"
        );
        let out = show(c, Some(9), true, PLAIN).unwrap();
        assert!(out.starts_with("\n[09][09][09]\n\nRGB      rgb(219 129 119)\nHEX      #db8177\n"));
        assert!(out.contains("\nOKLCH    oklch("));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn plain_literal_has_no_swatch() {
        let c = Color::from_srgb8(219, 129, 119);
        assert_eq!(show(c, None, false, PLAIN).unwrap(), "#db8177\n");
        assert!(
            show(c, None, true, PLAIN)
                .unwrap()
                .starts_with("RGB      rgb(219 129 119)\n")
        );
    }

    #[test]
    fn out_of_gamut_cannot_be_shown_or_swatched() {
        let c = Color::from_oklch(0.6, 0.4, 25.0);
        assert!(show(c, None, false, PLAIN).is_err());
        assert!(swatch(c, None, COLOR).is_none());
    }

    #[test]
    fn let_swatch_is_three_cells_and_only_with_color() {
        let c = Color::from_srgb8(219, 129, 119);
        assert!(swatch(c, None, PLAIN).is_none());
        let s = swatch(c, None, COLOR).unwrap();
        assert_eq!(s, "\x1b[48;2;219;129;119m   \x1b[0m".repeat(3));
        assert!(
            swatch(c, Some(9), COLOR)
                .unwrap()
                .starts_with("\x1b[101m   \x1b[0m")
        );
    }

    #[test]
    fn plain_full_is_two_lines_of_names() {
        assert_eq!(
            full(false, PLAIN),
            "black red green yellow blue magenta cyan white\n\
             bright-black bright-red bright-green bright-yellow bright-blue bright-magenta bright-cyan bright-white\n"
        );
        assert_eq!(full(true, PLAIN), full(false, PLAIN));
    }

    #[test]
    fn colored_full_tries_every_name_on_each_background() {
        let out = full(false, COLOR);
        assert!(out.starts_with("\x1b[30mblack \x1b[0m\x1b[31mred \x1b[0m"));
        assert!(out.contains("\x1b[30;40mblack \x1b[0m"));
        assert!(out.contains("\x1b[97;47mbright-white \x1b[0m"));
        assert!(!out.contains(";100m"));
        assert_eq!(out.matches("mblack \x1b[0m").count(), 9);

        let more = full(true, COLOR);
        assert!(more.contains("\x1b[30;100mblack \x1b[0m"));
        assert!(more.contains("\x1b[97;107mbright-white \x1b[0m"));
        assert_eq!(more.matches("mblack \x1b[0m").count(), 17);
    }

    #[test]
    fn linux_console_gets_bright_backgrounds_through_blink() {
        let out = preview(CONSOLE);
        assert!(out.contains("\x1b[40m   \x1b[0m"), "normal slot unchanged");
        assert!(out.contains("\x1b[5;40m   \x1b[0m"), "slot 8 = blink + 40");
        assert!(out.contains("\x1b[5;47m   \x1b[0m"), "slot 15 = blink + 47");
        assert!(
            !out.contains("100m") && !out.contains("107m"),
            "100-107 render as normal there"
        );

        let more = full(true, CONSOLE);
        assert!(more.contains("\x1b[30;5;40mblack \x1b[0m"));
        assert!(more.contains("\x1b[97;5;47mbright-white \x1b[0m"));
        assert!(!more.contains(";100m"));

        let names = expand(&sample(), true, CONSOLE).unwrap();
        assert!(
            names.contains("\x1b[91mbright-red\x1b[0m"),
            "bright foreground is plain 9x"
        );
    }

    #[test]
    fn other_terminals_keep_100_to_107() {
        for style in [COLOR, INDEXED, BASIC] {
            let out = preview(style);
            assert!(out.contains("\x1b[100m   \x1b[0m") && !out.contains("5;4"));
        }
    }

    #[test]
    fn literals_are_drawn_only_where_they_can_be_drawn_accurately() {
        let c = Color::from_srgb8(219, 129, 119);
        let n = to_256(219, 129, 119);
        let indexed = show(c, None, false, INDEXED).unwrap();
        assert!(
            indexed.contains(&format!("\x1b[48;5;{n}m   \x1b[0m")),
            "256-color approximation"
        );
        assert!(!indexed.contains("48;2"));
        assert!(swatch(c, None, INDEXED).is_some());
        for style in [CONSOLE, BASIC] {
            assert_eq!(show(c, None, false, style).unwrap(), "#db8177\n");
            assert!(show(c, None, true, style).unwrap().starts_with("RGB      "));
            assert!(swatch(c, None, style).is_none());
            assert!(
                swatch(c, Some(9), style).is_some(),
                "palette slots always work"
            );
        }
    }

    #[test]
    fn nearest_256_color() {
        assert_eq!(to_256(255, 0, 0), 196);
        assert_eq!(to_256(255, 255, 255), 231);
        assert_eq!(to_256(0, 0, 0), 16);
        assert_eq!(to_256(128, 128, 128), 244);
        assert_eq!(to_256(95, 135, 175), 67);
        assert_eq!(to_256(10, 10, 10), 232);
    }
}
