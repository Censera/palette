mod cli;
mod color;
mod console;
mod render;

use std::process::ExitCode;

fn main() -> ExitCode {
    console::enable_ansi_support();

    let args: Vec<String> = std::env::args().skip(1).collect();

    let mode = match cli::parse(&args) {
        Ok(mode) => mode,
        Err(e) => {
            eprintln!("palette: {e}");
            return ExitCode::FAILURE;
        }
    };

    match mode {
        cli::Mode::Blocks => render::blocks(),
        cli::Mode::Detailed => render::detailed(),
        cli::Mode::Long => render::long(),
        cli::Mode::LongBright => render::long_bright(),
        cli::Mode::Rgb(color) => render::rgb_swatch(color),
    }

    ExitCode::SUCCESS
}
