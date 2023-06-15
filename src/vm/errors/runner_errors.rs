use crate::stdlib::{collections::HashSet, prelude::*};

#[cfg(feature = "std")]
use thiserror::Error;
#[cfg(not(feature = "std"))]
use thiserror_no_std::Error;

use super::memory_errors::MemoryError;
use crate::types::{errors::math_errors::MathError, relocatable::Relocatable};
use felt::Felt252;

#[derive(Debug, PartialEq, Error)]
pub enum RunnerError {
    #[error("Initialization failure: No execution base")]
    NoExecBase,
    #[error("Initialization failure: No program base")]
    NoProgBase,
    #[error("Missing main()")]
    MissingMain,
    #[error("Found None PC during VM initialization")]
    NoPC,
    #[error("Found None AP during VM initialization")]
    NoAP,
    #[error("Found None FP during VM initialization")]
    NoFP,
    #[error("Memory validation failed during VM initialization: {0}")]
    MemoryValidationError(MemoryError),
    #[error("Memory loading failed during state initialization: {0}")]
    MemoryInitializationError(MemoryError),
    #[error("Failed to convert string to FieldElement")]
    FailedStringConversion,
    #[error("EcOpBuiltin: m should be at most {0}")]
    EcOpBuiltinScalarLimit(Felt252),
    #[error("Given builtins are not in appropiate order")]
    DisorderedBuiltins,
    #[error("Expected integer at address {0:?} to be smaller than 2^{1}, Got {2}")]
    IntegerBiggerThanPowerOfTwo(Relocatable, u32, Felt252),
    #[error("{0}")]
    EcOpSameXCoordinate(String),
    #[error("EcOpBuiltin: point {0:?} is not on the curve")]
    PointNotOnCurve((Felt252, Felt252)),
    #[error("Builtin(s) {0:?} not present in layout {1}")]
    NoBuiltinForInstance(HashSet<&'static str>, String),
    #[error("Invalid layout {0}")]
    InvalidLayoutName(String),
    #[error("end_run called twice.")]
    EndRunCalledTwice,
    #[error("end_run must be called before finalize_segments.")]
    FinalizeNoEndRun,
    #[error("end_run must be called before read_return_values.")]
    ReadReturnValuesNoEndRun,
    #[error("Error while finalizing segments: {0}")]
    FinalizeSegements(MemoryError),
    #[error("finalize_segments called but proof_mode is not enabled")]
    FinalizeSegmentsNoProofMode,
    #[error("Invalid stop pointer for {0}: Stop pointer has value {1} but builtin segment is {2}")]
    InvalidStopPointerIndex(&'static str, Relocatable, usize),
    #[error("Invalid stop pointer for {0}. Expected: {1}, found: {2}")]
    InvalidStopPointer(&'static str, Relocatable, Relocatable),
    #[error("No stop pointer found for builtin {0}")]
    NoStopPointer(&'static str),
    #[error("Running in proof-mode but no __start__ label found, try compiling with proof-mode")]
    NoProgramStart,
    #[error("Running in proof-mode but no __end__ label found, try compiling with proof-mode")]
    NoProgramEnd,
    #[error("Could not convert slice to array")]
    SliceToArrayError,
    #[error("Cannot add the return values to the public memory after segment finalization.")]
    FailedAddingReturnValues,
    #[error("Missing execution public memory")]
    NoExecPublicMemory,
    #[error("Coulnd't parse prime from felt lib")]
    CouldntParsePrime,
    #[error("Could not convert vec with Maybe Relocatables into u64 array")]
    MaybeRelocVecToU64ArrayError,
    #[error("Expected Integer value, got Relocatable instead")]
    FoundNonInt,
    #[error(transparent)]
    Memory(#[from] MemoryError),
    #[error(transparent)]
    Math(#[from] MathError),
    #[error("keccak_builtin: Failed to get first input address")]
    KeccakNoFirstInput,
    #[error("{0}: Expected integer at address {1}")]
    BuiltinExpectedInteger(&'static str, Relocatable),
    #[error("keccak_builtin: Failed to convert input cells to u64 values")]
    KeccakInputCellsNotU64,
}
