#[cfg(feature = "std")]
use thiserror::Error;
#[cfg(not(feature = "std"))]
use thiserror_no_std::Error;

use super::memory_errors::MemoryError;
use super::vm_exception::VmException;
use crate::types::errors::program_errors::ProgramError;
use crate::vm::errors::{
    runner_errors::RunnerError, trace_errors::TraceError, vm_errors::VirtualMachineError,
};

#[derive(Debug, Error)]
pub enum CairoRunError {
    #[error(transparent)]
    Program(#[from] ProgramError),
    #[error(transparent)]
    VirtualMachine(#[from] VirtualMachineError),
    #[error(transparent)]
    Trace(#[from] TraceError),
    #[error(transparent)]
    Runner(#[from] RunnerError),
    #[error(transparent)]
    MemoryError(#[from] MemoryError),
    #[error(transparent)]
    VmException(#[from] VmException),
}
