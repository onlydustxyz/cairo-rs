use std::prelude::v1::*;

use felt::PRIME_STR;
use std::io;

#[derive(Debug)]
pub enum ProgramError {
    IO(io::Error),
    Parse(serde_json::Error),
    EntrypointNotFound(String),
    ConstWithoutValue(String),
    PrimeDiffers(String),
}

impl std::fmt::Display for ProgramError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProgramError::IO(e) => e.fmt(f),
            ProgramError::Parse(e) => e.fmt(f),
            ProgramError::EntrypointNotFound(v) => format!("Entrypoint {v} not found").fmt(f),
            ProgramError::ConstWithoutValue(v) => format!("Constant {v} has no value").fmt(f),
            ProgramError::PrimeDiffers(v) => format!("Expected prime {PRIME_STR}, got {v}").fmt(f),
        }
    }
}

impl From<io::Error> for ProgramError {
    fn from(value: io::Error) -> Self {
        Self::IO(value)
    }
}

impl From<serde_json::Error> for ProgramError {
    fn from(value: serde_json::Error) -> Self {
        Self::Parse(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ProgramError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ProgramError::IO(e) => Some(e),
            ProgramError::Parse(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_entrypoint_not_found_error() {
        let error = ProgramError::EntrypointNotFound(String::from("my_function"));
        let formatted_error = format!("{}", error);
        assert_eq!(formatted_error, "Entrypoint my_function not found");
    }
}
