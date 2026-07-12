use crate::color::{sgr, Base, Color, ANSI_RESET};

const SWATCH: &str = "   ";

pub fn blocks() {
    print_swatch_row(Base::ALL.map(Color::Standard));
    print_swatch_row(Base::ALL.map(Color::Bright));
}

fn print_swatch_row(colors: [Color; 8]) {
    for color in colors {
        print!("{}{SWATCH}{ANSI_RESET}", sgr(&[color.bg_code()]));
    }
    println!();
}

pub fn detailed() {
    print_swatch_row(Base::ALL.map(Color::Standard));
    print_swatch_row(Base::ALL.map(Color::Bright));
    println!();
    for base in Base::ALL {
        print_name_pair(Color::Standard(base), Color::Bright(base));
    }
}

fn print_name_pair(standard: Color, bright: Color) {
    let std_label = format!(
        "{}{}{ANSI_RESET}",
        sgr(&[standard.fg_code()]),
        standard.name()
    );
    let bright_label = format!("{}{}{ANSI_RESET}", sgr(&[bright.fg_code()]), bright.name());
    println!("{std_label:<20} {bright_label}");
}

pub fn long() {
    print_fg_row(None);
    for bg_base in Base::ALL {
        print_fg_row(Some(Color::Standard(bg_base)));
    }
}

pub fn long_bright() {
    print_fg_row(None);
    for bg_base in Base::ALL {
        print_fg_row(Some(Color::Standard(bg_base)));
    }
    for bg_base in Base::ALL {
        print_fg_row(Some(Color::Bright(bg_base)));
    }
}

fn print_fg_row(bg: Option<Color>) {
    print_fg_line(Base::ALL.map(Color::Standard), bg);
    print_fg_line(Base::ALL.map(Color::Bright), bg);
    println!();
}

fn print_fg_line(colors: [Color; 8], bg: Option<Color>) {
    for color in colors {
        let mut codes = vec![color.fg_code()];
        if let Some(bg) = bg {
            codes.push(bg.bg_code());
        }
        print!("{}{} {ANSI_RESET}", sgr(&codes), color.name());
    }
    println!();
}

pub fn rgb_swatch(color: Color) {
    print!("{}{SWATCH}{SWATCH}{ANSI_RESET} ", sgr(&[color.bg_code()]));
    println!("{}", color.name());
}
