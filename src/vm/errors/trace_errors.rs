use crate::vm::errors::memory_errors::MemoryError;

#[derive(Debug, PartialEq, Eq)]
pub enum TraceError {
    TraceNotEnabled,
    AlreadyRelocated,
    RegNotRelocatable,
    NoRelocationFound,
    MemoryError(MemoryError),
}

impl std::fmt::Display for TraceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TraceError::TraceNotEnabled => "Trace is not enabled for this run".fmt(f),
            TraceError::AlreadyRelocated => "Trace is already relocated".fmt(f),
            TraceError::RegNotRelocatable => "Trace register must be relocatable".fmt(f),
            TraceError::NoRelocationFound => "No relocation found for this segment".fmt(f),
            TraceError::MemoryError(e) => e.fmt(f),
        }
    }
}

impl From<MemoryError> for TraceError {
    fn from(value: MemoryError) -> Self {
        Self::MemoryError(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TraceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TraceError::MemoryError(e) => Some(e),
            _ => None,
        }
    }
}
