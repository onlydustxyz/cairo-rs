use super::memory_errors::MemoryError;
use super::vm_exception::VmException;
use crate::types::errors::program_errors::ProgramError;
use crate::vm::errors::{
    runner_errors::RunnerError, trace_errors::TraceError, vm_errors::VirtualMachineError,
};
use std::prelude::v1::*;

#[derive(Debug)]
pub enum CairoRunError {
    Program(ProgramError),
    VirtualMachine(VirtualMachineError),
    Trace(TraceError),
    Runner(RunnerError),
    MemoryError(MemoryError),
    VmException(VmException),
}

impl std::fmt::Display for CairoRunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CairoRunError::Program(e) => e.fmt(f),
            CairoRunError::VirtualMachine(e) => e.fmt(f),
            CairoRunError::Trace(e) => e.fmt(f),
            CairoRunError::Runner(e) => e.fmt(f),
            CairoRunError::MemoryError(e) => e.fmt(f),
            CairoRunError::VmException(e) => e.fmt(f),
        }
    }
}

impl From<ProgramError> for CairoRunError {
    fn from(e: ProgramError) -> Self {
        Self::Program(e)
    }
}

impl From<VirtualMachineError> for CairoRunError {
    fn from(e: VirtualMachineError) -> Self {
        Self::VirtualMachine(e)
    }
}

impl From<TraceError> for CairoRunError {
    fn from(e: TraceError) -> Self {
        Self::Trace(e)
    }
}

impl From<RunnerError> for CairoRunError {
    fn from(e: RunnerError) -> Self {
        Self::Runner(e)
    }
}

impl From<MemoryError> for CairoRunError {
    fn from(e: MemoryError) -> Self {
        Self::MemoryError(e)
    }
}

impl From<VmException> for CairoRunError {
    fn from(e: VmException) -> Self {
        Self::VmException(e)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for CairoRunError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CairoRunError::Program(e) => Some(e),
            CairoRunError::VirtualMachine(e) => Some(e),
            CairoRunError::Trace(e) => Some(e),
            CairoRunError::Runner(e) => Some(e),
            CairoRunError::MemoryError(e) => Some(e),
            CairoRunError::VmException(e) => Some(e),
        }
    }
}
