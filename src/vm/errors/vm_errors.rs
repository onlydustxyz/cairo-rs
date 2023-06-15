use crate::stdlib::prelude::*;

#[cfg(feature = "std")]
use thiserror::Error;
#[cfg(not(feature = "std"))]
use thiserror_no_std::Error;

use crate::{
    types::{
        errors::math_errors::MathError,
        relocatable::{MaybeRelocatable, Relocatable},
    },
    vm::errors::{
        exec_scope_errors::ExecScopeError, hint_errors::HintError, memory_errors::MemoryError,
        runner_errors::RunnerError, trace_errors::TraceError,
    },
};
use felt::Felt252;

#[derive(Debug, Error)]
pub enum VirtualMachineError {
    #[error("Instruction should be an int")]
    InvalidInstructionEncoding,
    #[error("Invalid op1_register value: {0}")]
    InvalidOp1Reg(i64),
    #[error("In immediate mode, off2 should be 1")]
    ImmShouldBe1,
    #[error("op0 must be known in double dereference")]
    UnknownOp0,
    #[error("Invalid ap_update value: {0}")]
    InvalidApUpdate(i64),
    #[error("Invalid pc_update value: {0}")]
    InvalidPcUpdate(i64),
    #[error("Res.UNCONSTRAINED cannot be used with ApUpdate.ADD")]
    UnconstrainedResAdd,
    #[error("Res.UNCONSTRAINED cannot be used with PcUpdate.JUMP")]
    UnconstrainedResJump,
    #[error("Res.UNCONSTRAINED cannot be used with PcUpdate.JUMP_REL")]
    UnconstrainedResJumpRel,
    #[error("Res.UNCONSTRAINED cannot be used with Opcode.ASSERT_EQ")]
    UnconstrainedResAssertEq,
    #[error("An integer value as Res cannot be used with PcUpdate.JUMP_REL")]
    JumpRelNotInt,
    #[error(
        "Failed to compute Res.MUL: Could not complete computation of non pure values {0} * {1}"
    )]
    ComputeResRelocatableMul(MaybeRelocatable, MaybeRelocatable),
    #[error("Couldn't compute operand {0}. Unknown value for memory cell {1}")]
    FailedToComputeOperands(String, Relocatable),
    #[error("An ASSERT_EQ instruction failed: {0} != {1}.")]
    DiffAssertValues(MaybeRelocatable, MaybeRelocatable),
    #[error("Call failed to write return-pc (inconsistent op0): {0} != {1}. Did you forget to increment ap?")]
    CantWriteReturnPc(MaybeRelocatable, MaybeRelocatable),
    #[error("Call failed to write return-fp (inconsistent dst): {0} != {1}. Did you forget to increment ap?")]
    CantWriteReturnFp(MaybeRelocatable, MaybeRelocatable),
    #[error("Couldn't get or load dst")]
    NoDst,
    #[error("Invalid res value: {0}")]
    InvalidRes(i64),
    #[error("Invalid opcode value: {0}")]
    InvalidOpcode(i64),
    #[error("This is not implemented")]
    NotImplemented,
    #[error("Inconsistent auto-deduction for builtin {0}, expected {1}, got {2:?}")]
    InconsistentAutoDeduction(&'static str, MaybeRelocatable, Option<MaybeRelocatable>),
    #[error(transparent)]
    RunnerError(#[from] RunnerError),
    #[error("Invalid hint encoding at pc: {0}")]
    InvalidHintEncoding(MaybeRelocatable),
    #[error(transparent)]
    Memory(#[from] MemoryError),
    #[error("Expected range_check builtin to be present")]
    NoRangeCheckBuiltin,
    #[error("Expected ecdsa builtin to be present")]
    NoSignatureBuiltin,
    #[error("Div out of range: 0 < {0} <= {1}")]
    OutOfValidRange(Felt252, Felt252),
    #[error("Failed to compare {0} and {1}, cant compare a relocatable to an integer value")]
    DiffTypeComparison(MaybeRelocatable, MaybeRelocatable),
    #[error("Failed to compare {0} and  {1}, cant compare two relocatable values of different segment indexes")]
    DiffIndexComp(Relocatable, Relocatable),
    #[error("Couldn't convert usize to u32")]
    NoneInMemoryRange,
    #[error("Expected integer, found: {0:?}")]
    ExpectedIntAtRange(Option<MaybeRelocatable>),
    #[error("Could not convert slice to array")]
    SliceToArrayError,
    #[error("Failed to compile hint: {0}")]
    CompileHintFail(String),
    #[error("op1_addr is Op1Addr.IMM, but no immediate was given")]
    NoImm,
    #[error("Execution reached the end of the program. Requested remaining steps: {0}.")]
    EndOfProgram(usize),
    #[error(transparent)]
    TracerError(#[from] TraceError),
    #[error(transparent)]
    MainScopeError(#[from] ExecScopeError),
    #[error("Current run is not finished")]
    RunNotFinished,
    #[error("Invalid argument count, expected {0} but got {1}")]
    InvalidArgCount(usize, usize),
    #[error("Couldn't parse prime: {0}")]
    CouldntParsePrime(String),
    #[error("Got an exception while executing a hint: {1}")]
    Hint(usize, Box<HintError>),
    #[error("Unexpected Failure")]
    Unexpected,
    #[error("Out of bounds access to builtin segment")]
    OutOfBoundsBuiltinSegmentAccess,
    #[error("Out of bounds access to program segment")]
    OutOfBoundsProgramSegmentAccess,
    #[error("Security Error: Invalid Memory Value: temporary address not relocated: {0}")]
    InvalidMemoryValueTemporaryAddress(Relocatable),
    #[error("accessed_addresses is None.")]
    MissingAccessedAddresses,
    #[error(transparent)]
    Math(#[from] MathError),
    #[error("Failed to write the output builtin content")]
    FailedToWriteOutput,
    #[error(transparent)]
    Other(anyhow::Error),
}
