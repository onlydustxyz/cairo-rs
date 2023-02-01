use std::prelude::v1::*;

use felt::Felt;
use num_bigint::{BigInt, BigUint};

use crate::types::relocatable::{MaybeRelocatable, Relocatable};

use super::{exec_scope_errors::ExecScopeError, vm_errors::VirtualMachineError};

#[derive(Debug, PartialEq)]
pub enum HintError {
    WrongHintData,
    FailedToGetIds,
    NoRegisterInReference,
    CustomHint(String),
    MissingConstant(&'static str),
    FailedToGetConstant,
    ArcTooBig(Felt, Felt, Felt, Felt),
    ExcludedNot2(Felt),
    ValueOutside250BitRange(Felt),
    ScopeError,
    VariableNotInScopeError(String),
    CantCreateDictionaryOnTakenSegment(isize),
    NoDictTracker(isize),
    NoValueForKey(MaybeRelocatable),
    NoValueForKeyFindElement(Felt),
    AssertLtFelt(Felt, Felt),
    FindElemMaxSize(Felt, Felt),
    InvalidIndex(Felt, Felt, Felt),
    KeyNotFound,
    NoneApTrackingData,
    InvalidTrackingGroup(usize, usize),
    InvalidApValue(MaybeRelocatable),
    NoInitialDict,
    NoKeyInAccessIndices(Felt),
    EmptyAccessIndices,
    EmptyCurrentAccessIndices,
    CurrentAccessIndicesNotEmpty,
    WrongPrevValue(MaybeRelocatable, MaybeRelocatable, MaybeRelocatable),
    NumUsedAccessesAssertFail(Felt, usize, Felt),
    KeysNotEmpty,
    EmptyKeys,
    PtrDiffNotDivisibleByDictAccessSize,
    SquashDictMaxSizeExceeded(Felt, Felt),
    NAccessesTooBig(Felt),
    Internal(VirtualMachineError),
    BigintToUsizeFail,
    UsortOutOfRange(u64, Felt),
    UnexpectedPositionsDictFail,
    PositionsNotFound,
    PositionsLengthNotZero,
    CouldntPopPositions,
    LastPosNotFound,
    InvalidSetRange(MaybeRelocatable, MaybeRelocatable),
    FixedSizeArrayFail(usize),
    AssertionFailed(String),
    MismatchedDictPtr(Relocatable, Relocatable),
    SecpSplitNegative(BigInt),
    SecpSplitOutOfRange(BigUint),
    SecpVerifyZero(BigInt),
    KeccakMaxSize(Felt, Felt),
    InvalidWordSize(Felt),
    InvalidKeccakInputLength(Felt),
    FromScopeError(ExecScopeError),
    AssertNotEqualFail(MaybeRelocatable, MaybeRelocatable),
    SplitIntNotZero,
    SplitIntLimbOutOfRange(Felt),
    InvalidKeccakStateSizeFelts(Felt),
    InvalidBlockSize(Felt),
    BigintToU32Fail,
    BigIntToBigUintFail,
    ValueOutOfRange(Felt),
    AssertNNValueOutOfRange(Felt),
    AssertNotZero(Felt, String),
    OutOfValidRange(Felt, Felt),
    ValueOutsideValidRange(Felt),
    NonLeFelt(Felt, Felt),
    UnknownHint(String),
}

impl std::fmt::Display for HintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            HintError::WrongHintData => "HintProcessor failed retrieve the compiled data necessary for hint execution".fmt(f),
            HintError::FailedToGetIds => "Failed to get ids for hint execution".fmt(f),
            HintError::NoRegisterInReference => "Tried to compute an address but there was no register in the reference".fmt(f),
            HintError::CustomHint(v) => format!("Custom Hint Error: {v}").fmt(f),
            HintError::MissingConstant(v) => format!("Missing constant: {v}").fmt(f),
            HintError::FailedToGetConstant => "Fail to get constants for hint execution".fmt(f),
            HintError::ArcTooBig(v0,v1,v2, v3) => format!("Arc too big, {v0} must be <= {v1} and {v2} <= {v3}").fmt(f),
            HintError::ExcludedNot2(v) => format!("Excluded is supposed to be 2, got {v}").fmt(f),
            HintError::ValueOutside250BitRange(v) => format!("Value: {v} is outside of the range [0, 2**250)").fmt(f),
            HintError::ScopeError => "Failed to get scope variables".fmt(f),
            HintError::VariableNotInScopeError(v) => format!("Variable {v} not present in current execution scope").fmt(f),
            HintError::CantCreateDictionaryOnTakenSegment(v) => format!("DictManagerError: Tried to create tracker for a dictionary on segment: {v} when there is already a tr.fmt(f)acker for a dictionary on this segment").fmt(f),
            HintError::NoDictTracker(v) => format!("Dict Error: No dict tracker found for segment {v}").fmt(f),
            HintError::NoValueForKey(v) => format!("Dict Error: No value found for key: {v}").fmt(f),
            HintError::NoValueForKeyFindElement(v) => format!("find_element(): No value found for key: {v}").fmt(f),
            HintError::AssertLtFelt(a, b) => format!("Assertion failed, a = {a} % PRIME is not less than b = {b} % PRIME").fmt(f),
            HintError::FindElemMaxSize(v0, v1) => format!("find_element() can only be used with n_elms <= {v0}.\nGot: n_elms = {v1}").fmt(f),
            HintError::InvalidIndex(index, expected, found) => format!("Invalid index found in find_element_index. Index: {index}.\nExpected key: {expected}, found_key {found}").fmt(f),
            HintError::KeyNotFound => "Found Key is None".fmt(f),
            HintError::NoneApTrackingData => "AP tracking data is None; could not apply correction to address".fmt(f),
            HintError::InvalidTrackingGroup(v0, v1) => format!("Tracking groups should be the same, got {v0} and {v1}").fmt(f),
            HintError::InvalidApValue(v) => format!("Expected relocatable for ap, got {v}").fmt(f),
            HintError::NoInitialDict => "Dict Error: Tried to create a dict whithout an initial dict".fmt(f),
            HintError::NoKeyInAccessIndices(v) => format!("squash_dict_inner fail: couldnt find key {v} in accesses_indices").fmt(f),
            HintError::EmptyAccessIndices => "squash_dict_inner fail: local accessed_indices is empty".fmt(f),
            HintError::EmptyCurrentAccessIndices => "squash_dict_inner fail: local current_accessed_indices is empty".fmt(f),
            HintError::CurrentAccessIndicesNotEmpty => "squash_dict_inner fail: local current_accessed_indices not empty, loop ended with remaining unaccounted elements".fmt(f),
            HintError::WrongPrevValue(expected, got, key) => format!("Dict Error: Got the wrong value for dict_update, expected value: {expected}, got: {got} for key: {key}").fmt(f),
            HintError::NumUsedAccessesAssertFail(used_accesses, length, key) => format!("squash_dict_inner fail: Number of used accesses:{used_accesses} doesnt match the length: {length} of the access_indices at key: {key}").fmt(f),
            HintError::KeysNotEmpty => "squash_dict_inner fail: local keys is not empty".fmt(f),
            HintError::EmptyKeys => "squash_dict_inner fail: No keys left but remaining_accesses > 0".fmt(f),
            HintError::PtrDiffNotDivisibleByDictAccessSize => "squash_dict fail: Accesses array size must be divisible by DictAccess.SIZE".fmt(f),
            HintError::SquashDictMaxSizeExceeded(v0, v1) => format!("squash_dict() can only be used with n_accesses<={v0}. ' \nGot: n_accesses={v1}").fmt(f),
            HintError::NAccessesTooBig(v0) => format!("squash_dict fail: n_accesses: {v0} is too big to be converted into an iterator").fmt(f),
            HintError::Internal(e) => e.fmt(f),
            HintError::BigintToUsizeFail => "Couldn't convert BigInt to usize".fmt(f),
            HintError::UsortOutOfRange(v0, v1) => format!("usort() can only be used with input_len<={v0}. Got: input_len={v1}.").fmt(f),
            HintError::UnexpectedPositionsDictFail => "unexpected usort fail: positions_dict or key value pair not found".fmt(f),
            HintError::PositionsNotFound => "unexpected verify multiplicity fail: positions not found".fmt(f),
            HintError::PositionsLengthNotZero => "unexpected verify multiplicity fail: positions length != 0".fmt(f),
            HintError::CouldntPopPositions => "unexpected verify multiplicity fail: couldn't pop positions".fmt(f),
            HintError::LastPosNotFound => "unexpected verify multiplicity fail: last_pos not found".fmt(f),
            HintError::InvalidSetRange(start, end) => format!("Set's starting point {start} is bigger it's ending point {end}").fmt(f),
            HintError::FixedSizeArrayFail(v) => format!("Failed to construct a fixed size array of size: {v}").fmt(f),
            HintError::AssertionFailed(v) => v.fmt(f),
            HintError::MismatchedDictPtr(got, expected) => format!{"Wrong dict pointer supplied. Got {got}, expected {expected}."}.fmt(f),
            HintError::SecpSplitNegative(v) => format!{"Integer must be postive or zero, got: {v}"}.fmt(f),
            HintError::SecpSplitOutOfRange(v) => format!{"Integer: {v} out of range"}.fmt(f),
            HintError::SecpVerifyZero(v) => format!("verify_zero: Invalid input {v}").fmt(f),
            HintError::KeccakMaxSize(v0, v1) => format!("unsafe_keccak() can only be used with length<={v0}. Got: length={v1}").fmt(f),
            HintError::InvalidWordSize(size) => format!("Invalid word size: {size}").fmt(f),
            HintError::InvalidKeccakInputLength(length) => format!("Invalid input length, Got: length={length}").fmt(f),
            HintError::FromScopeError(e) =>  e.fmt(f),
            HintError::AssertNotEqualFail(v0, v1) => format!("assert_not_equal failed: {v0} = {v1}").fmt(f),
            HintError::SplitIntNotZero => "split_int(): value is out of range".fmt(f),
            HintError::SplitIntLimbOutOfRange(v) => format!("split_int(): Limb {v} is out of range").fmt(f),
            HintError::InvalidKeccakStateSizeFelts(v) => format!("Expected size to be in the range from [0, 100), got: {v}").fmt(f),
            HintError::InvalidBlockSize(v) => format!("Expected size to be in range from [0, 10), got: {v}").fmt(f),
            HintError::BigintToU32Fail => "Couldn't convert BigInt to u32".fmt(f),
            HintError::BigIntToBigUintFail => "BigInt to BigUint failed, BigInt is negative".fmt(f),
            HintError::ValueOutOfRange(v) => format!("Assertion failed, 0 <= ids.a % PRIME < range_check_builtin.bound \n a = {v} is out of range").fmt(f),
            HintError::AssertNNValueOutOfRange(v) => format!("Assertion failed, 0 <= ids.a % PRIME < range_check_builtin.bound \n a = {v} is out of range").fmt(f),
            HintError::AssertNotZero(v0, v1) => format!("Assertion failed, {v0} % {v1} is equal to 0").fmt(f),
            HintError::OutOfValidRange(v0, v1) => format!("Div out of range: 0 < {v0} <= {v1}").fmt(f),
            HintError::ValueOutsideValidRange(v) => format!("Value: {v} is outside valid range").fmt(f),
            HintError::NonLeFelt(v0, v1) => format!("Assertion failed, {v0}, is not less or equal to {v1}").fmt(f),
            HintError::UnknownHint(v) => format!("Unknown Hint: {v}").fmt(f),
        }
    }
}

impl From<VirtualMachineError> for HintError {
    fn from(value: VirtualMachineError) -> Self {
        Self::Internal(value)
    }
}

impl From<ExecScopeError> for HintError {
    fn from(value: ExecScopeError) -> Self {
        Self::FromScopeError(value)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HintError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HintError::FromScopeError(e) => Some(e),
            HintError::Internal(e) => Some(e),
            _ => None,
        }
    }
}
