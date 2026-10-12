// One error type for the whole tool. The variant decides the exit status.

use std::fmt;

#[derive(Debug)]
pub enum Error {
    /// Bad command, option, or argument. Exit status 2.
    Usage(String),
    /// Palette detection or conversion failure. Exit status 1.
    Runtime(String),
}

impl Error {
    pub fn usage(msg: impl Into<String>) -> Self {
        Error::Usage(msg.into())
    }

    pub fn runtime(msg: impl Into<String>) -> Self {
        Error::Runtime(msg.into())
    }

    pub fn exit_code(&self) -> u8 {
        match self {
            Error::Usage(_) => 2,
            Error::Runtime(_) => 1,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Usage(m) | Error::Runtime(m) => f.write_str(m),
        }
    }
}
