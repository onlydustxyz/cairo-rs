#[cfg(feature = "std")]
use thiserror::Error;
#[cfg(not(feature = "std"))]
use thiserror_no_std::Error;

#[derive(Eq, Hash, PartialEq, Debug, Error)]
pub enum ExecScopeError {
    #[error("Cannot exit main scope.")]
    ExitMainScopeError,
    #[error("Every enter_scope() requires a corresponding exit_scope().")]
    NoScopeError,
}
