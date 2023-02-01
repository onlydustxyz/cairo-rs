use std::prelude::v1::*;

use felt::PRIME_STR;
use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProgramError {
    #[error(transparent)]
    IO(#[from] io::Error),
    #[error(transparent)]
    Parse(#[from] serde_json::Error),
    #[error("Entrypoint {0} not found")]
    EntrypointNotFound(String),
    #[error("Constant {0} has no value")]
    ConstWithoutValue(String),
    #[error("Expected prime {PRIME_STR}, got {0}")]
    PrimeDiffers(String),
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
