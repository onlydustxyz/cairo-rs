use std::prelude::v1::*;

use std::collections::HashSet;

use super::memory_errors::MemoryError;
use crate::types::relocatable::MaybeRelocatable;
use felt::Felt;

#[derive(Debug, PartialEq, Eq)]
pub enum RunnerError {
    NoExecBase,
    NoExecBaseForEntrypoint,
    NoProgBase,
    MissingMain,
    UninitializedBase,
    BaseNotFinished,
    WriteFail,
    NoPC,
    NoAP,
    NoFP,
    MemoryValidationError(MemoryError),
    MemoryInitializationError(MemoryError),
    NonRelocatableAddress,
    RunnerInTemporarySegment(isize),
    FailedStringConversion,
    ExpectedInteger(MaybeRelocatable),
    MemoryGet(MaybeRelocatable),
    FailedMemoryGet(MemoryError),
    EcOpBuiltinScalarLimit(Felt),
    DisorderedBuiltins,
    IntegerBiggerThanPowerOfTwo(MaybeRelocatable, u32, Felt),
    EcOpSameXCoordinate(String),
    PointNotOnCurve((Felt, Felt)),
    NoBuiltinForInstance(HashSet<String>, String),
    InvalidLayoutName(String),
    RunAlreadyFinished,
    FinalizeNoEndRun,
    ReadReturnValuesNoEndRun,
    BuiltinNotIncluded(String),
    BuiltinSegmentNameCollision(&'static str),
    FinalizeSegements(MemoryError),
    FinalizeSegmentsNoProofMode,
    FinalStack,
    InvalidStopPointer(String),
    NoProgramStart,
    NoProgramEnd,
    SliceToArrayError,
    MissingBuiltin(String),
    FailedAddingReturnValues,
    NoExecPublicMemory,
    CouldntParsePrime,
    MaybeRelocVecToU64ArrayError,
    FoundNonInt,
    SafeDivFailUsize(usize, usize),
    MemoryError(MemoryError),
    NegBuiltinBase,
}

impl std::fmt::Display for RunnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RunnerError::NoExecBase => "Can't initialize state without an execution base".fmt(f),
            RunnerError::NoExecBaseForEntrypoint => {
                "Can't initialize the function entrypoint without an execution base".fmt(f)
            }

            RunnerError::NoProgBase => "Initialization failure: No program base".fmt(f),
            RunnerError::MissingMain => "Missing main()".fmt(f),

            RunnerError::UninitializedBase => "Uninitialized base for builtin".fmt(f),
            RunnerError::BaseNotFinished => "Base for builtin is not finished".fmt(f),
            RunnerError::WriteFail => "Failed to write program output".fmt(f),
            RunnerError::NoPC => "Found None PC during VM initialization".fmt(f),
            RunnerError::NoAP => "Found None AP during VM initialization".fmt(f),
            RunnerError::NoFP => "Found None FP during VM initialization".fmt(f),
            RunnerError::MemoryValidationError(e) => {
                format!("Memory validation failed during VM initialization: {}", e).fmt(f)
            }
            RunnerError::MemoryInitializationError(e) => {
                format!("Memory loading failed during state initialization: {}", e).fmt(f)
            }
            RunnerError::NonRelocatableAddress => "Memory addresses must be relocatable".fmt(f),
            RunnerError::RunnerInTemporarySegment(seg) => format!(
                "Runner base mustn't be in a TemporarySegment, segment: {}",
                seg
            )
            .fmt(f),
            RunnerError::FailedStringConversion => {
                "Failed to convert string to FieldElement".fmt(f)
            }
            RunnerError::ExpectedInteger(addr) => {
                format!("Expected integer at address {:?}", addr).fmt(f)
            }
            RunnerError::MemoryGet(addr) => {
                format!("Failed to retrieve value from address {:?}", addr).fmt(f)
            }
            RunnerError::FailedMemoryGet(e) => {
                format!("Failed to retrieve value from memory: {}", e).fmt(f)
            }
            RunnerError::EcOpBuiltinScalarLimit(limit) => {
                format!("EcOpBuiltin: m should be at most {}", limit).fmt(f)
            }
            RunnerError::DisorderedBuiltins => "Given builtins are not in appropiate order".fmt(f),
            RunnerError::IntegerBiggerThanPowerOfTwo(addr, power, value) => format!(
                "Expected integer at address {:?} to be smaller than 2^{}, Got {}",
                addr, power, value
            )
            .fmt(f),
            RunnerError::EcOpSameXCoordinate(msg) => {
                format!("EcOpBuiltin: point is not on the curve: {}", msg).fmt(f)
            }

            RunnerError::PointNotOnCurve(point) => {
                format!("EcOpBuiltin: point {:?} is not on the curve", point).fmt(f)
            }

            RunnerError::NoBuiltinForInstance(builtins, layout) => {
                format!("Builtin(s) {:?} not present in layout {}", builtins, layout).fmt(f)
            }

            RunnerError::InvalidLayoutName(layout) => format!("Invalid layout {}", layout).fmt(f),

            RunnerError::RunAlreadyFinished => "Run has already ended.".fmt(f),

            RunnerError::FinalizeNoEndRun => {
                "end_run must be called before finalize_segments.".fmt(f)
            }

            RunnerError::ReadReturnValuesNoEndRun => {
                "end_run must be called before read_return_values.".fmt(f)
            }

            RunnerError::BuiltinNotIncluded(builtin) => {
                format!("Builtin {} not included.", builtin).fmt(f)
            }

            RunnerError::BuiltinSegmentNameCollision(name) => {
                format!("Builtin segment name collision on '{}'", name).fmt(f)
            }

            RunnerError::FinalizeSegements(e) => {
                format!("Failed to finalize segments: {}", e).fmt(f)
            }

            RunnerError::FinalizeSegmentsNoProofMode => {
                "finalize_segments can only be called in proof mode.".fmt(f)
            }

            RunnerError::FinalStack => "Final stack is not empty.".fmt(f),

            RunnerError::InvalidStopPointer(msg) => format!("Invalid stop pointer: {}", msg).fmt(f),

            RunnerError::NoProgramStart => "No program start.".fmt(f),

            RunnerError::NoProgramEnd => "No program end.".fmt(f),

            RunnerError::SliceToArrayError => "Failed to convert slice to array.".fmt(f),

            RunnerError::MissingBuiltin(builtin) => format!("Missing builtin: {}", builtin).fmt(f),

            RunnerError::FailedAddingReturnValues => {
                "Failed to add return values to memory.".fmt(f)
            }

            RunnerError::NoExecPublicMemory => "No public memory in execution mode.".fmt(f),

            RunnerError::CouldntParsePrime => "Could not parse prime.".fmt(f),

            RunnerError::MaybeRelocVecToU64ArrayError => {
                "Failed to convert relocatable vector to u64 array.".fmt(f)
            }

            RunnerError::FoundNonInt => "Found non integer.".fmt(f),

            RunnerError::SafeDivFailUsize(a, b) => format!("Safe div failed: {} / {}", a, b).fmt(f),

            RunnerError::MemoryError(e) => format!("Memory error: {}", e).fmt(f),

            RunnerError::NegBuiltinBase => "Neg builtin base.".fmt(f),
        }
    }
}

impl From<MemoryError> for RunnerError {
    fn from(e: MemoryError) -> Self {
        RunnerError::MemoryError(e)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for RunnerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RunnerError::MemoryValidationError(e) => Some(e),
            RunnerError::MemoryInitializationError(e) => Some(e),
            RunnerError::FailedMemoryGet(e) => Some(e),
            RunnerError::FinalizeSegements(e) => Some(e),
            RunnerError::MemoryError(e) => Some(e),
            _ => None,
        }
    }
}
