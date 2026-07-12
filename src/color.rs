pub const ANSI_RESET: &str = "\x1b[0m";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Base {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Base {
    pub const ALL: [Base; 8] = [
        Base::Black,
        Base::Red,
        Base::Green,
        Base::Yellow,
        Base::Blue,
        Base::Magenta,
        Base::Cyan,
        Base::White,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Base::Black => "black",
            Base::Red => "red",
            Base::Green => "green",
            Base::Yellow => "yellow",
            Base::Blue => "blue",
            Base::Magenta => "magenta",
            Base::Cyan => "cyan",
            Base::White => "white",
        }
    }

    // 30-37 for fg, 40-47 for bg, offset by 10 applied at call site
    fn code_offset(self) -> u8 {
        match self {
            Base::Black => 0,
            Base::Red => 1,
            Base::Green => 2,
            Base::Yellow => 3,
            Base::Blue => 4,
            Base::Magenta => 5,
            Base::Cyan => 6,
            Base::White => 7,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Color {
    Standard(Base),
    Bright(Base),
    Rgb(u8, u8, u8),
}

impl Color {
    pub fn name(self) -> String {
        match self {
            Color::Standard(b) => b.name().to_string(),
            Color::Bright(b) => format!("bright-{}", b.name()),
            Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
        }
    }

    pub fn fg_code(self) -> String {
        match self {
            Color::Standard(b) => format!("3{}", b.code_offset()),
            Color::Bright(b) => format!("9{}", b.code_offset()),
            Color::Rgb(r, g, b) => format!("38;2;{r};{g};{b}"),
        }
    }

    pub fn bg_code(self) -> String {
        match self {
            Color::Standard(b) => format!("4{}", b.code_offset()),
            Color::Bright(b) => format!("10{}", b.code_offset()),
            Color::Rgb(r, g, b) => format!("48;2;{r};{g};{b}"),
        }
    }
}

pub fn sgr(codes: &[String]) -> String {
    format!("\x1b[{}m", codes.join(";"))
}
