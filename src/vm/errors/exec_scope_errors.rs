use std::prelude::v1::*;

#[derive(Eq, Hash, PartialEq, Debug)]
pub enum ExecScopeError {
    ExitMainScopeError,
    NoScopeError,
}

impl std::fmt::Display for ExecScopeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecScopeError::ExitMainScopeError => "Cannot exit main scope.".fmt(f),
            ExecScopeError::NoScopeError => {
                "Every enter_scope() requires a corresponding exit_scope().".fmt(f)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ExecScopeError {}
