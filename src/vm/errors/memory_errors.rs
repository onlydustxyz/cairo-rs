use crate::stdlib::prelude::*;

#[cfg(feature = "std")]
use thiserror::Error;
#[cfg(not(feature = "std"))]
use thiserror_no_std::Error;

use felt::Felt252;

use crate::types::{
    errors::math_errors::MathError,
    relocatable::{MaybeRelocatable, Relocatable},
};

#[derive(Debug, PartialEq, Error)]
pub enum MemoryError {
    #[error("Can't insert into segment #{0}; memory only has {1} segment")]
    UnallocatedSegment(usize, usize),
    #[error("Memory addresses must be relocatable")]
    AddressNotRelocatable,
    #[error("Range-check validation failed, number {0} is out of valid range [0, {1}]")]
    RangeCheckNumOutOfBounds(Felt252, Felt252),
    #[error("Range-check validation failed, encountered non-int value at address {0}")]
    RangeCheckFoundNonInt(Relocatable),
    #[error("Inconsistent memory assignment at address {0:?}. {1:?} != {2:?}")]
    InconsistentMemory(Relocatable, MaybeRelocatable, MaybeRelocatable),
    #[error("Inconsistent Relocation")]
    Relocation,
    #[error("Could not cast arguments")]
    WriteArg,
    #[error("Memory addresses mustn't be in a TemporarySegment, segment: {0}")]
    AddressInTemporarySegment(isize),
    #[error("Memory addresses must be in a TemporarySegment, segment: {0}")]
    AddressNotInTemporarySegment(isize),
    #[error("Temporary segment found while relocating (flattening), segment: {0}")]
    TemporarySegmentInRelocation(isize),
    #[error("The TemporarySegment: {0} doesn't have a relocation address")]
    NonZeroOffset(usize),
    #[error("Attempt to overwrite a relocation rule, segment: {0}")]
    DuplicatedRelocation(isize),
    #[error("Segment effective sizes haven't been calculated.")]
    MissingSegmentUsedSizes,
    #[error("Found a memory gap when calling get_continuous_range with base:{0} and size: {1}")]
    GetRangeMemoryGap(Relocatable, usize),
    #[error("Error calculating builtin memory units")]
    ErrorCalculatingMemoryUnits,
    #[error("Missing memory cells for builtin {0}")]
    MissingMemoryCells(&'static str),
    #[error("Missing memory cells for builtin {0}: {1:?}")]
    MissingMemoryCellsWithOffsets(&'static str, Vec<usize>),
    #[error("ErrorInitializing Verifying Key from public key: {0:?}")]
    InitializingVerifyingKey(Vec<u8>),
    #[error(
        "Signature {0}, is invalid, with respect to the public key {1}, 
    and the message hash {2}."
    )]
    InvalidSignature(String, Felt252, Felt252),
    #[error(
        "Signature hint is missing for ECDSA builtin at address {0}.
    Add it using 'ecdsa_builtin.add_signature'."
    )]
    SignatureNotFound(Relocatable),
    #[error("Could not create pubkey from: {0:?}")]
    ErrorParsingPubKey(String),
    #[error("Could not retrieve message from: {0:?}")]
    ErrorRetrievingMessage(String),
    #[error("Error verifying given signature")]
    ErrorVerifyingSignature,
    #[error("Couldn't obtain a mutable accessed offset")]
    CantGetMutAccessedOffset,
    #[error("ECDSA builtin: Expected public key at address {0} to be an integer")]
    PubKeyNonInt(Relocatable),
    #[error("ECDSA builtin: Expected message hash at address {0} to be an integer")]
    MsgNonInt(Relocatable),
    #[error("Failed to convert String: {0} to FieldElement")]
    FailedStringToFieldElementConversion(String),
    #[error("Failed to fetch {0} return values, ap is only {1}")]
    FailedToGetReturnValues(usize, Relocatable),
    #[error(transparent)]
    InsufficientAllocatedCells(#[from] InsufficientAllocatedCellsError),
    #[error("Segment {0} has {1} amount of accessed addresses but its size is only {2}.")]
    SegmentHasMoreAccessedAddressesThanSize(usize, usize, usize),
    #[error("gen_arg: found argument of invalid type.")]
    GenArgInvalidType,
    #[error(transparent)]
    Math(#[from] MathError),
    // Memory.get() errors
    #[error("Expected integer at address {0}")]
    ExpectedInteger(Relocatable),
    #[error("Expected relocatable at address {0}")]
    ExpectedRelocatable(Relocatable),
    #[error("Unknown memory cell at address {0}")]
    UnknownMemoryCell(Relocatable),
    // SegmentArenaBuiltin
    #[error("segment_arena_builtin: assert used >= INITIAL_SEGMENT_SIZE")]
    InvalidUsedSizeSegmentArena,
}

#[derive(Debug, PartialEq, Eq, Error)]
pub enum InsufficientAllocatedCellsError {
    #[error("Number of steps must be at least {0} for the {1} builtin.")]
    MinStepNotReached(usize, &'static str),
    #[error("The {0} builtin used {1} cells but the capacity is {2}.")]
    BuiltinCells(&'static str, usize, usize),
    #[error("There are only {0} cells to fill the range checks holes, but potentially {1} are required.")]
    RangeCheckUnits(usize, usize),
    #[error("There are only {0} cells to fill the diluted check holes, but potentially {1} are required.")]
    DilutedCells(usize, usize),
    #[error("There are only {0} cells to fill the memory address holes, but {1} are required.")]
    MemoryAddresses(u32, usize),
}
