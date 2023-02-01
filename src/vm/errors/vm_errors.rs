use std::prelude::v1::*;

use crate::{
    types::relocatable::{MaybeRelocatable, Relocatable},
    vm::errors::{
        exec_scope_errors::ExecScopeError, hint_errors::HintError, memory_errors::MemoryError,
        runner_errors::RunnerError, trace_errors::TraceError,
    },
};
use felt::Felt;
use num_bigint::{BigInt, BigUint};

#[derive(Debug, PartialEq)]
pub enum VirtualMachineError {
    InvalidInstructionEncoding,
    InvalidOp1Reg(i64),
    ImmShouldBe1,
    UnknownOp0,
    InvalidApUpdate(i64),
    InvalidPcUpdate(i64),
    UnconstrainedResAdd,
    UnconstrainedResJump,
    UnconstrainedResJumpRel,
    UnconstrainedResAssertEq,
    FailedToComputeOperands(String, Relocatable),
    DiffAssertValues(MaybeRelocatable, MaybeRelocatable),
    CantWriteReturnPc(MaybeRelocatable, MaybeRelocatable),
    CantWriteReturnFp(MaybeRelocatable, MaybeRelocatable),
    NoDst,
    PureValue,
    InvalidRes(i64),
    InvalidOpcode(i64),
    RelocatableAdd,
    OffsetExceeded(Felt),
    NotImplemented,
    DiffIndexSub,
    InconsistentAutoDeduction(String, MaybeRelocatable, Option<MaybeRelocatable>),
    RunnerError(RunnerError),
    InvalidHintEncoding(MaybeRelocatable),
    MemoryError(MemoryError),
    NoRangeCheckBuiltin,
    NoSignatureBuiltin,
    MemoryGet(MaybeRelocatable),
    ExpectedInteger(MaybeRelocatable),
    ExpectedRelocatable(MaybeRelocatable),
    ValueNotPositive(Felt),
    OutOfValidRange(Felt, Felt),
    DiffTypeComparison(MaybeRelocatable, MaybeRelocatable),
    DiffIndexComp(Relocatable, Relocatable),
    BigintToUsizeFail,
    BigintToU64Fail,
    BigintToU32Fail,
    NoneInMemoryRange,
    UsizeToU32Fail,
    SqrtNegative(Felt),
    SafeDivFail(Felt, Felt),
    SafeDivFailBigInt(BigInt, BigInt),
    SafeDivFailBigUint(BigUint, BigUint),
    SafeDivFailU32(u32, u32),
    SafeDivFailUsize(usize, usize),
    DividedByZero,
    FailedToGetSqrt(BigUint),
    ExpectedIntAtRange(Option<MaybeRelocatable>),
    SliceToArrayError,
    CompileHintFail(String),
    NoImm,
    CantSubOffset(usize, usize),
    EndOfProgram(usize),
    TracerError(TraceError),
    MainScopeError(ExecScopeError),
    RunNotFinished,
    InvalidArgCount(usize, usize),
    CouldntParsePrime(String),
    ErrorMessageAttribute(String, Box<VirtualMachineError>),
    Hint(usize, Box<HintError>),
    Unexpected,
    OutOfBoundsBuiltinSegmentAccess,
    OutOfBoundsProgramSegmentAccess,
    NegBuiltinBase,
    InvalidMemoryValueTemporaryAddress(Relocatable),
}

impl std::fmt::Display for VirtualMachineError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            VirtualMachineError::InvalidInstructionEncoding => "Instruction should be an int".fmt(f),
            VirtualMachineError::InvalidOp1Reg(v) => format!("Invalid op1_register value: {v}").fmt(f),
            VirtualMachineError::ImmShouldBe1 => "In immediate mode, off2 should be 1".fmt(f),
            VirtualMachineError::UnknownOp0 => "op0 must be known in double dereference".fmt(f),
            VirtualMachineError::InvalidApUpdate(v) => format!("Invalid ap_update value: {v}").fmt(f),
            VirtualMachineError::InvalidPcUpdate(v) => format!("Invalid pc_update value: {v}").fmt(f),
            VirtualMachineError::UnconstrainedResAdd => {
                "Res.UNCONSTRAINED cannot be used with ApUpdate.ADD".fmt(f)
            }
            VirtualMachineError::UnconstrainedResJump => {
                "Res.UNCONSTRAINED cannot be used with PcUpdate.JUMP".fmt(f)
            }
            VirtualMachineError::UnconstrainedResJumpRel => {
                "Res.UNCONSTRAINED cannot be used with PcUpdate.JUMP_REL".fmt(f)
            }
            VirtualMachineError::UnconstrainedResAssertEq => {
                "Res.UNCONSTRAINED cannot be used with Opcode.ASSERT_EQ".fmt(f)
            }
            VirtualMachineError::FailedToComputeOperands(operand, address) => {
                format!("Couldn't compute operand {operand} at address {address}").fmt(f)
            }
            VirtualMachineError::DiffAssertValues(v1, v2) => {
                format!("An ASSERT_EQ instruction failed: {v1} != {v2}.").fmt(f)
            }
            VirtualMachineError::CantWriteReturnPc(v1, v2) => format!("Call failed to write return-pc (inconsistent op0): {v1} != {v2}. Did you forget to increment ap?").fmt(f),
            VirtualMachineError::CantWriteReturnFp(v1, v2) => format!("Call failed to write return-fp (inconsistent dst): {v1} != {v2}. Did you forget to increment ap?").fmt(f),
            VirtualMachineError::NoDst => "Couldn't get or load dst".fmt(f),
            VirtualMachineError::PureValue => "Pure Value Error".fmt(f),
            VirtualMachineError::InvalidRes(v) => format!("Invalid res value: {v}").fmt(f),
            VirtualMachineError::InvalidOpcode(v) => format!("Invalid opcode value: {v}").fmt(f),
            VirtualMachineError::RelocatableAdd => "Cannot add two relocatable values".fmt(f),
            VirtualMachineError::OffsetExceeded(offset) => format!("Offset {offset} exceeds maximum offset value").fmt(f),
            VirtualMachineError::NotImplemented => "This is not implemented".fmt(f),
            VirtualMachineError::DiffIndexSub => "Can only subtract two relocatable values of the same segment".fmt(f),
            VirtualMachineError::InconsistentAutoDeduction(builtin, expected, actual) => format!("Inconsistent auto-deduction for builtin {builtin}, expected {expected}, got {actual:?}").fmt(f),
            VirtualMachineError::RunnerError(e) => e.fmt(f),
            VirtualMachineError::InvalidHintEncoding(pc) => format!("Invalid hint encoding at pc: {pc}").fmt(f),
            VirtualMachineError::MemoryError(e) => e.fmt(f),
            VirtualMachineError::NoRangeCheckBuiltin => "Expected range_check builtin to be present".fmt(f),
            VirtualMachineError::NoSignatureBuiltin => "Expected ecdsa builtin to be present".fmt(f),
            VirtualMachineError::MemoryGet(address) => format!("Failed to retrieve value from address {address}").fmt(f),
            VirtualMachineError::ExpectedInteger(address) => format!("Expected integer at address {address}").fmt(f),
            VirtualMachineError::ExpectedRelocatable(address) => format!("Expected relocatable at address {address}").fmt(f),
            VirtualMachineError::ValueNotPositive(v) => format!("Value: {v} should be positive").fmt(f),
            VirtualMachineError::OutOfValidRange(v, upper_range) => format!("Div out of range: 0 < {v} <= {upper_range}").fmt(f),
            VirtualMachineError::DiffTypeComparison(v1, v2) => format!("Failed to compare {v1} and {v2}, cant compare a relocatable to an integer value").fmt(f),
            VirtualMachineError::DiffIndexComp(v1, v2) => format!("Failed to compare {v1} and  {v2}, cant compare two relocatable values of different segment indexes").fmt(f),
            VirtualMachineError::BigintToUsizeFail => "Couldn't convert BigInt to usize".fmt(f),
            VirtualMachineError::BigintToU64Fail => "Couldn't convert BigInt to u64".fmt(f),
            VirtualMachineError::BigintToU32Fail => "Couldn't convert BigInt to u32".fmt(f),
            VirtualMachineError::NoneInMemoryRange => "Couldn't convert usize to u32".fmt(f),
            VirtualMachineError::UsizeToU32Fail => "Couldn't convert usize to u32".fmt(f),
            VirtualMachineError::SqrtNegative(v) => format!("Can't calculate the square root of negative number: {v})").fmt(f),
            VirtualMachineError::SafeDivFail(dividend, divisor) => format!("{dividend} is not divisible by {divisor}").fmt(f),
            VirtualMachineError::SafeDivFailBigInt(dividend, divisor) => format!("{dividend} is not divisible by {divisor}").fmt(f),
            VirtualMachineError::SafeDivFailBigUint(dividend, divisor) => format!("{dividend} is not divisible by {divisor}").fmt(f),
            VirtualMachineError::SafeDivFailU32(dividend, divisor) => format!("{dividend} is not divisible by {divisor}").fmt(f),
            VirtualMachineError::SafeDivFailUsize(dividend, divisor) => format!("{dividend} is not divisible by {divisor}").fmt(f),
            VirtualMachineError::DividedByZero => "Attempted to divide by zero".fmt(f),
            VirtualMachineError::FailedToGetSqrt(v) => format!("Failed to calculate the square root of: {v})").fmt(f),
            VirtualMachineError::ExpectedIntAtRange(v) => format!("Expected integer, found: {v:?}").fmt(f),
            VirtualMachineError::SliceToArrayError => "Could not convert slice to array".fmt(f),
            VirtualMachineError::CompileHintFail(hint) => format!("Failed to compile hint: {hint}").fmt(f),
            VirtualMachineError::NoImm => "op1_addr is Op1Addr.IMM, but no immediate was given".fmt(f),
            VirtualMachineError::CantSubOffset(v, offset) => format!("Cant substract {v} from offset {offset}, offsets cant be negative").fmt(f),
            VirtualMachineError::EndOfProgram(remaining_steps) => format!("Execution reached the end of the program. Requested remaining steps: {remaining_steps}.").fmt(f),
            VirtualMachineError::TracerError(e) => e.fmt(f),
            VirtualMachineError::MainScopeError(e) => e.fmt(f),
            VirtualMachineError::RunNotFinished => "Current run is not finished".fmt(f),
            VirtualMachineError::InvalidArgCount(expected, actual) => format!("Invalid argument count, expected {expected} but got {actual}").fmt(f),
            VirtualMachineError::CouldntParsePrime(v) => format!("Couldn't parse prime: {v}").fmt(f),
            VirtualMachineError::ErrorMessageAttribute(message, error) => format!("{message}, {error}").fmt(f),
            VirtualMachineError::Hint(_, error) => format!("Got an exception while executing a hint: {error}").fmt(f),
            VirtualMachineError::Unexpected => "Unexpected Failure".fmt(f),
            VirtualMachineError::OutOfBoundsBuiltinSegmentAccess => "Out of bounds access to builtin segment".fmt(f),
            VirtualMachineError::OutOfBoundsProgramSegmentAccess => "Out of bounds access to program segment".fmt(f),
            VirtualMachineError::NegBuiltinBase => "Negative builtin base".fmt(f),
            VirtualMachineError::InvalidMemoryValueTemporaryAddress(address) => format!("Security Error: Invalid Memory Value: temporary address not relocated: {address}").fmt(f),
        }
    }
}

impl From<RunnerError> for VirtualMachineError {
    fn from(error: RunnerError) -> Self {
        Self::RunnerError(error)
    }
}

impl From<MemoryError> for VirtualMachineError {
    fn from(error: MemoryError) -> Self {
        Self::MemoryError(error)
    }
}

impl From<TraceError> for VirtualMachineError {
    fn from(error: TraceError) -> Self {
        Self::TracerError(error)
    }
}

impl From<ExecScopeError> for VirtualMachineError {
    fn from(error: ExecScopeError) -> Self {
        Self::MainScopeError(error)
    }
}

#[cfg(feature = "std")]
impl std::error::Error for VirtualMachineError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VirtualMachineError::RunnerError(e) => Some(e),
            VirtualMachineError::MemoryError(e) => Some(e),
            VirtualMachineError::TracerError(e) => Some(e),
            VirtualMachineError::MainScopeError(e) => Some(e),
            _ => None,
        }
    }
}
