use felt::Felt252;
use num_bigint::{BigInt, BigUint};

#[cfg(feature = "std")]
use thiserror::Error;
#[cfg(not(feature = "std"))]
use thiserror_no_std::Error;

use crate::types::relocatable::{MaybeRelocatable, Relocatable};

#[derive(Debug, Error, PartialEq)]
pub enum MathError {
    // Math functions
    #[error("Can't calculate the square root of negative number: {0})")]
    SqrtNegative(Felt252),
    #[error("{0} is not divisible by {1}")]
    SafeDivFail(Felt252, Felt252),
    #[error("{0} is not divisible by {1}")]
    SafeDivFailBigInt(BigInt, BigInt),
    #[error("{0} is not divisible by {1}")]
    SafeDivFailBigUint(BigUint, BigUint),
    #[error("{0} is not divisible by {1}")]
    SafeDivFailU32(u32, u32),
    #[error("{0} is not divisible by {1}")]
    SafeDivFailUsize(usize, usize),
    #[error("Attempted to divide by zero")]
    DividedByZero,
    #[error("Failed to calculate the square root of: {0})")]
    FailedToGetSqrt(BigUint),
    #[error("is_quad_residue: p must be > 0")]
    IsQuadResidueZeroPrime,
    // Relocatable Operations
    #[error("Cant convert felt: {0} to Relocatable")]
    Felt252ToRelocatable(Felt252),
    #[error("Operation failed: {0} - {1}, offsets cant be negative")]
    RelocatableSubNegOffset(Relocatable, usize),
    #[error("Operation failed: {0} + {1}, maximum offset value exceeded")]
    RelocatableAddFelt252OffsetExceeded(Relocatable, Felt252),
    #[error("Operation failed: {0} + {1}, maximum offset value exceeded")]
    RelocatableAddUsizeOffsetExceeded(Relocatable, usize),
    #[error("Operation failed: {0} + {1}, can't add two relocatable values")]
    RelocatableAdd(Relocatable, Relocatable),
    #[error("Operation failed: {0} - {1}, can't subtract two relocatable values with different segment indexes")]
    RelocatableSubDiffIndex(Relocatable, Relocatable),
    #[error(
        "Operation failed: {0}.divmod({1}, divmod can only be performed between two integer values"
    )]
    DivModWrongType(MaybeRelocatable, MaybeRelocatable),
    #[error("Operation failed {0} - {1}, can't subtract a relocatable value from an integer")]
    SubRelocatableFromInt(Felt252, Relocatable),
    // Type conversions
    #[error("Conversion to i32 failed for Felt252 {0}")]
    Felt252ToI32Conversion(Felt252),
    #[error("Conversion to u32 failed for Felt252 {0}")]
    Felt252ToU32Conversion(Felt252),
    #[error("Conversion to usize failed for Felt252 {0}")]
    Felt252ToUsizeConversion(Felt252),
    #[error("Conversion to u64 failed for Felt252 {0}")]
    Felt252ToU64Conversion(Felt252),
}
