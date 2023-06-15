#[cfg(feature = "std")]
use thiserror::Error;
#[cfg(not(feature = "std"))]
use thiserror_no_std::Error;

use crate::vm::errors::memory_errors::MemoryError;

#[derive(Debug, PartialEq, Error)]
pub enum TraceError {
    #[error("Trace is not enabled for this run")]
    TraceNotEnabled,
    #[error("Trace is already relocated")]
    AlreadyRelocated,
    #[error("No relocation found for execution segment")]
    NoRelocationFound,
    #[error(transparent)]
    MemoryError(#[from] MemoryError),
    #[error("Trace not relocated")]
    TraceNotRelocated,
}
