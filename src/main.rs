mod cli;
mod console;
mod logic;
mod render;
mod source;

use std::process::ExitCode;

fn main() -> ExitCode {
    cli::run()
}
