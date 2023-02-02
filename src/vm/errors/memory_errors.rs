use std::prelude::v1::*;

use crate::types::relocatable::{MaybeRelocatable, Relocatable};

#[derive(Debug, PartialEq, Eq)]
pub enum MemoryError {
    UnallocatedSegment(usize, usize),
    AddressNotRelocatable,
    NumOutOfBounds,
    FoundNonInt,
    InconsistentMemory(MaybeRelocatable, MaybeRelocatable, MaybeRelocatable),
    EffectiveSizesNotCalled,
    Relocation,
    WriteArg,
    AddressInTemporarySegment(isize),
    AddressNotInTemporarySegment(isize),
    TemporarySegmentInRelocation(isize),
    NonZeroOffset(usize),
    DuplicatedRelocation(isize),
    MissingAccessedAddresses,
    MissingSegmentUsedSizes,
    SegmentNotFinalized(usize),
    InvalidMemoryValue(Relocatable, MaybeRelocatable),
    GetRangeMemoryGap,
    ErrorCalculatingMemoryUnits,
    InsufficientAllocatedCells,
    MissingMemoryCells(&'static str),
    MissingMemoryCellsWithOffsets(&'static str, Vec<usize>),
    InitializingVerifyingKey(Vec<u8>),
    InvalidSignature,
    SignatureNotFound,
    ErrorParsingPubKey(String),
    ErrorRetrievingMessage(String),
    ErrorVerifyingSignature,
    CantGetMutAccessedOffset,
    FailedStringToFieldElementConversion(String),
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MemoryError::UnallocatedSegment(v0, v1) => {
                format!("Can't insert into segment #{v0}; memory only has {v1} segment").fmt(f)
            }
            MemoryError::AddressNotRelocatable => "Memory addresses must be relocatable".fmt(f),
            MemoryError::NumOutOfBounds => {
                "Range-check validation failed, number is out of valid range".fmt(f)
            }
            MemoryError::FoundNonInt => {
                "Range-check validation failed, encountered non-int value".fmt(f)
            }
            MemoryError::InconsistentMemory(v0, v1, v2) => {
                format!("Inconsistent memory assignment at address {v0:?}. {v1:?} != {v2:?}").fmt(f)
            }
            MemoryError::EffectiveSizesNotCalled => {
                "compute_effective_sizes should be called before relocate_segments".fmt(f)
            }
            MemoryError::Relocation => "Inconsistent Relocation".fmt(f),
            MemoryError::WriteArg => "Could not cast arguments".fmt(f),
            MemoryError::AddressInTemporarySegment(v) => {
                format!("Memory addresses mustn't be in a TemporarySegment, segment: {v}").fmt(f)
            }
            MemoryError::AddressNotInTemporarySegment(v) => {
                format!("Memory addresses must be in a TemporarySegment, segment: {v}").fmt(f)
            }
            MemoryError::TemporarySegmentInRelocation(v) => {
                format!("Temporary segment found while relocating (flattening), segment: {v}")
                    .fmt(f)
            }
            MemoryError::NonZeroOffset(v) => {
                format!("The TemporarySegment: {v} doesn't have a relocation address").fmt(f)
            }
            MemoryError::DuplicatedRelocation(v) => {
                format!("Attempt to overwrite a relocation rule, segment: {v}").fmt(f)
            }
            MemoryError::MissingAccessedAddresses => "accessed_addresses is None.".fmt(f),
            MemoryError::MissingSegmentUsedSizes => {
                "Segment effective sizes haven't been calculated".fmt(f)
            }
            MemoryError::SegmentNotFinalized(v) => {
                format!("Segment at index {v} either doesn't exist or is not finalized").fmt(f)
            }
            MemoryError::InvalidMemoryValue(v0, v1) => {
                format!("Invalid memory value at address {v0:?}: {v1:?}").fmt(f)
            }
            MemoryError::GetRangeMemoryGap => {
                "Found a memory gap when calling get_continuous_range".fmt(f)
            }
            MemoryError::ErrorCalculatingMemoryUnits => {
                "Error calculating builtin memory units".fmt(f)
            }
            MemoryError::InsufficientAllocatedCells => {
                "Number of steps is insufficient in the builtin".fmt(f)
            }
            MemoryError::MissingMemoryCells(v) => {
                format!("Missing memory cells for builtin {v}").fmt(f)
            }
            MemoryError::MissingMemoryCellsWithOffsets(v0, v1) => {
                format!("Missing memory cells for builtin {v0}: {v1:?}").fmt(f)
            }
            MemoryError::InitializingVerifyingKey(v) => {
                format!("ErrorInitializing Verifying Key from public key: {v:?}").fmt(f)
            }
            MemoryError::InvalidSignature => "Invalid Signature".fmt(f),
            MemoryError::SignatureNotFound => "Signature not found".fmt(f),
            MemoryError::ErrorParsingPubKey(v) => {
                format!("Could not create pubkey from: {v:?}").fmt(f)
            }
            MemoryError::ErrorRetrievingMessage(v) => {
                format!("Could not retrieve message from: {v:?}").fmt(f)
            }
            MemoryError::ErrorVerifyingSignature => "Error verifying given signature".fmt(f),
            MemoryError::CantGetMutAccessedOffset => {
                "Couldn't obtain a mutable accessed offset".fmt(f)
            }
            MemoryError::FailedStringToFieldElementConversion(v) => {
                format!("Failed to convert String: {v} to FieldElement").fmt(f)
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for MemoryError {}
