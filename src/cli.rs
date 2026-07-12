// Argument parsing only. Produces a Mode for the render layer to act on

use crate::color::Color;

#[derive(Debug)]
pub enum Mode {
    Blocks,
    Detailed,
    Long,
    LongBright,
    Rgb(Color),
}

#[derive(Debug)]
pub struct CliError(pub String);

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn parse(args: &[String]) -> Result<Mode, CliError> {
    match args {
        [] => Ok(Mode::Blocks),
        [flag] if flag == "-d" || flag == "--detailed" => Ok(Mode::Detailed),
        [flag] if flag == "-l" || flag == "--long" => Ok(Mode::Long),
        [flag] if flag == "-L" || flag == "--long-bright" => Ok(Mode::LongBright),
        [flag, value] if flag == "--rgb" => parse_rgb(value).map(Mode::Rgb),
        [unknown] => Err(CliError(format!(
            r#"unknown argument '{unknown}'
try:
  -d, --detailed
  -l, --long
  -L, --long-bright
      --rgb R,G,B"#
        ))),
        _ => Err(CliError("too many arguments".to_string())),
    }
}

fn parse_rgb(value: &str) -> Result<Color, CliError> {
    let parts: Vec<&str> = value.split(',').collect();
    let [r, g, b] = parts.as_slice() else {
        return Err(CliError(format!(
            "--rgb expects R,G,B (e.g. --rgb 255,0,128), got '{value}'"
        )));
    };
    let parse_channel = |s: &str| {
        s.trim()
            .parse::<u8>()
            .map_err(|_| CliError(format!("invalid RGB channel '{s}', expected 0-255")))
    };
    Ok(Color::Rgb(
        parse_channel(r)?,
        parse_channel(g)?,
        parse_channel(b)?,
    ))
}
