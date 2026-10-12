use crate::console;
use crate::logic::color::{self, Color};
use crate::logic::error::Error;
use crate::logic::parse;
use crate::render::{self, Depth, Style};
use crate::source;
use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

pub const USAGE: &str = "\
palette - terminal palette inspection and color conversion

usage:
  palette                          compact 16-color preview
  palette expand [--bright]        swatches with colored names and HEX values
  palette show <color> [--convert] inspect a palette entry or a color
  palette let <input> <output>     convert a color (swatch shown on a terminal)
  palette full [--bright]          names on each background; --bright adds bright ones
  palette help                     print this help
  palette version                  print the version

<color> is a palette index (09), a name (bright-red), or a literal such as
#db8177, rgb(219 129 119), hsl(...), hsv(...), oklab(...) or oklch(...).
<output> is HEX, RGB, HSL, HSV, OKLab, or OKLCH (case-insensitive).

options:
  --color=<auto|always|never>      ANSI colors in output (default: auto)

NO_COLOR is respected in auto mode.";

#[derive(Debug, PartialEq)]
pub enum Command {
    Preview,
    Expand { bright: bool },
    Show { color: String, convert: bool },
    Let { input: String, output: String },
    Full { bright: bool },
    Help,
    Version,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
}

#[derive(Debug)]
pub struct Invocation {
    pub command: Command,
    pub color: ColorChoice,
}

pub fn parse_args(args: &[String]) -> Result<Invocation, Error> {
    let mut positional: Vec<&str> = Vec::new();
    let mut bright = false;
    let mut convert = false;
    let mut help = false;
    let mut version = false;
    let mut color = ColorChoice::Auto;

    for arg in args {
        match arg.as_str() {
            "--bright" => bright = true,
            "--convert" => convert = true,
            "-h" | "--help" => help = true,
            "-V" | "--version" => version = true,
            a if a.starts_with("--color=") => {
                color = match &a["--color=".len()..] {
                    "auto" => ColorChoice::Auto,
                    "always" => ColorChoice::Always,
                    "never" => ColorChoice::Never,
                    other => {
                        return Err(Error::usage(format!(
                            "invalid value '{other}' for --color (expected auto, always, or never)"
                        )));
                    }
                };
            }
            a if a.starts_with('-') && a.len() > 1 => {
                return Err(Error::usage(format!(
                    "unknown option '{a}' (try 'palette help')"
                )));
            }
            a => positional.push(a),
        }
    }

    let command = if help {
        Command::Help
    } else if version {
        Command::Version
    } else {
        let name = positional.first().copied().unwrap_or("");
        let operands = positional.get(1..).unwrap_or(&[]);
        match name {
            "" => {
                reject(&[("--bright", bright), ("--convert", convert)], "palette")?;
                Command::Preview
            }
            "expand" => {
                reject(&[("--convert", convert)], name)?;
                expect_operands(name, operands, 0, "")?;
                Command::Expand { bright }
            }
            "full" => {
                reject(&[("--convert", convert)], name)?;
                expect_operands(name, operands, 0, "")?;
                Command::Full { bright }
            }
            "show" => {
                reject(&[("--bright", bright)], name)?;
                expect_operands(name, operands, 1, "<color>")?;
                Command::Show {
                    color: operands[0].to_string(),
                    convert,
                }
            }
            "let" => {
                reject(&[("--bright", bright), ("--convert", convert)], name)?;
                expect_operands(name, operands, 2, "<input> <output>")?;
                Command::Let {
                    input: operands[0].to_string(),
                    output: operands[1].to_string(),
                }
            }
            "help" => Command::Help,
            "version" => Command::Version,
            other => {
                return Err(Error::usage(format!(
                    "unknown command '{other}' (try 'palette help')"
                )));
            }
        }
    };

    Ok(Invocation { command, color })
}

fn reject(flags: &[(&str, bool)], command: &str) -> Result<(), Error> {
    match flags.iter().find(|(_, set)| *set) {
        Some((flag, _)) => Err(Error::usage(format!(
            "option {flag} does not apply to '{command}'"
        ))),
        None => Ok(()),
    }
}

fn expect_operands(
    command: &str,
    operands: &[&str],
    want: usize,
    shape: &str,
) -> Result<(), Error> {
    if operands.len() == want {
        return Ok(());
    }
    let usage = if shape.is_empty() {
        format!("palette {command}")
    } else {
        format!("palette {command} {shape}")
    };
    Err(Error::usage(format!(
        "{command}: expected {want} argument{}, got {} (usage: {usage})",
        if want == 1 { "" } else { "s" },
        operands.len()
    )))
}

pub fn run() -> ExitCode {
    console::enable_ansi_support();

    let args: Vec<String> = std::env::args().skip(1).collect();
    match execute(&args) {
        Ok(text) => write_stdout(&text),
        Err(e) => {
            eprintln!("palette: {e}");
            ExitCode::from(e.exit_code())
        }
    }
}

fn execute(args: &[String]) -> Result<String, Error> {
    let invocation = parse_args(args)?;
    let style = Style {
        color: use_color(invocation.color),
        depth: detect_depth(),
    };

    match invocation.command {
        Command::Help => Ok(format!("{USAGE}\n")),
        Command::Version => Ok(format!("palette {}\n", env!("CARGO_PKG_VERSION"))),
        Command::Preview => Ok(render::preview(style)),
        Command::Expand { bright } => render::expand(&source::detect()?, bright, style),
        Command::Full { bright } => Ok(render::full(bright, style)),
        Command::Show { color, convert } => {
            let (c, index) = resolve(parse::parse_arg(&color)?)?;
            render::show(c, index, convert, style)
        }
        Command::Let { input, output } => {
            let format = parse::parse_format(&output)?;
            let (c, index) = resolve(parse::parse_arg(&input)?)?;
            let value = color::format_color(c, format)?;
            Ok(match render::swatch(c, index, style) {
                Some(swatch) => format!("{value}  {swatch}\n"),
                None => format!("{value}\n"),
            })
        }
    }
}

fn resolve(arg: parse::Arg) -> Result<(Color, Option<usize>), Error> {
    match arg {
        parse::Arg::Literal(c) => Ok((c, None)),
        parse::Arg::Index(i) => Ok((source::detect()?.colors[i], Some(i))),
    }
}

fn env(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

fn use_color(choice: ColorChoice) -> bool {
    match choice {
        ColorChoice::Always => true,
        ColorChoice::Never => false,
        ColorChoice::Auto => auto_color(
            io::stdout().is_terminal(),
            std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()),
            &env("TERM"),
        ),
    }
}

fn auto_color(is_terminal: bool, no_color: bool, term: &str) -> bool {
    let known_terminal = cfg!(windows) || !(term.is_empty() || term == "dumb");
    is_terminal && !no_color && known_terminal
}

fn detect_depth() -> Depth {
    let windows_terminal = std::env::var_os("WT_SESSION").is_some_and(|v| !v.is_empty());
    depth_from(&env("TERM"), &env("COLORTERM"), windows_terminal)
}

fn depth_from(term: &str, colorterm: &str, windows_terminal: bool) -> Depth {
    if term == "linux" {
        Depth::Console
    } else if colorterm.eq_ignore_ascii_case("truecolor")
        || colorterm.eq_ignore_ascii_case("24bit")
        || windows_terminal
        || cfg!(windows)
        || term == "xterm-kitty"
        || term.ends_with("-direct")
    {
        Depth::True
    } else if term.contains("256color") {
        Depth::Indexed
    } else {
        Depth::Basic
    }
}

fn write_stdout(text: &str) -> ExitCode {
    let mut out = io::stdout().lock();
    match out.write_all(text.as_bytes()).and_then(|()| out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("palette: cannot write output: {e}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Invocation, Error> {
        parse_args(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_every_command() {
        assert_eq!(p(&[]).unwrap().command, Command::Preview);
        assert_eq!(
            p(&["expand"]).unwrap().command,
            Command::Expand { bright: false }
        );
        assert_eq!(
            p(&["expand", "--bright"]).unwrap().command,
            Command::Expand { bright: true }
        );
        assert_eq!(
            p(&["full", "--bright"]).unwrap().command,
            Command::Full { bright: true }
        );
        assert_eq!(
            p(&["show", "09", "--convert"]).unwrap().command,
            Command::Show {
                color: "09".into(),
                convert: true
            }
        );
        assert_eq!(
            p(&["let", "#ffffff", "oklch"]).unwrap().command,
            Command::Let {
                input: "#ffffff".into(),
                output: "oklch".into()
            }
        );
        assert_eq!(p(&["help"]).unwrap().command, Command::Help);
        assert_eq!(p(&["--version"]).unwrap().command, Command::Version);
    }

    #[test]
    fn color_option() {
        assert_eq!(p(&["--color=never"]).unwrap().color, ColorChoice::Never);
        assert_eq!(
            p(&["expand", "--color=always"]).unwrap().color,
            ColorChoice::Always
        );
        assert!(p(&["--color=sometimes"]).is_err());
    }

    #[test]
    fn bad_input_is_a_usage_error() {
        for bad in [
            &["frobnicate"][..],
            &["--converte"],
            &["show"],
            &["show", "1", "2"],
            &["let", "#fff"],
            &["expand", "--convert"],
            &["show", "1", "--bright"],
            &["--bright"],
            &["-d"],
        ] {
            assert_eq!(p(bad).unwrap_err().exit_code(), 2, "{bad:?}");
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn auto_color_needs_a_known_terminal() {
        assert!(auto_color(true, false, "xterm-256color"));
        assert!(auto_color(true, false, "linux"));
        assert!(!auto_color(false, false, "xterm-256color"), "pipe");
        assert!(!auto_color(true, true, "xterm-256color"), "NO_COLOR");
        assert!(!auto_color(true, false, "dumb"));
        assert!(!auto_color(true, false, ""), "TERM unset");
    }

    #[cfg(not(windows))]
    #[test]
    fn depth_detection() {
        assert_eq!(depth_from("linux", "truecolor", false), Depth::Console);
        assert_eq!(
            depth_from("xterm-256color", "truecolor", false),
            Depth::True
        );
        assert_eq!(depth_from("xterm-256color", "24BIT", false), Depth::True);
        assert_eq!(depth_from("xterm-kitty", "", false), Depth::True);
        assert_eq!(depth_from("xterm-direct", "", false), Depth::True);
        assert_eq!(depth_from("xterm-256color", "", true), Depth::True);
        assert_eq!(depth_from("tmux-256color", "", false), Depth::Indexed);
        assert_eq!(depth_from("xterm", "", false), Depth::Basic);
        assert_eq!(depth_from("", "", false), Depth::Basic);
        assert_eq!(depth_from("vt100", "", false), Depth::Basic);
    }
}
