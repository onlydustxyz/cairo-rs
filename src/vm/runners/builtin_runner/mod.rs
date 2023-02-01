use std::prelude::v1::*;

use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::{self, MemoryError};
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::errors::vm_errors::VirtualMachineError;
use crate::vm::vm_core::VirtualMachine;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;

mod bitwise;
mod ec_op;
mod hash;
mod keccak;
mod output;
mod range_check;
mod signature;

pub use self::keccak::KeccakBuiltinRunner;
pub use bitwise::BitwiseBuiltinRunner;
pub use ec_op::EcOpBuiltinRunner;
pub use hash::HashBuiltinRunner;
use num_integer::div_floor;
use num_traits::ToPrimitive;
pub use output::OutputBuiltinRunner;
pub use range_check::RangeCheckBuiltinRunner;
pub use signature::SignatureBuiltinRunner;

/* NB: this enum is no accident: we may need (and cairo-rs-py *does* need)
 * structs containing this to be `Send`. The only two ways to achieve that
 * are either storing a `dyn Trait` inside an `Arc<Mutex<&dyn Trait>>` or
 * making the type itself `Send`. We opted for not complicating the user nor
 * moving the guarantees to runtime by using an `enum` rather than a `Trait`.
 * This works under the assumption that we don't expect downstream users to
 * extend Cairo by adding new builtin runners.
 */
#[derive(Debug, Clone)]
pub enum BuiltinRunner {
    Bitwise(BitwiseBuiltinRunner),
    EcOp(EcOpBuiltinRunner),
    Hash(HashBuiltinRunner),
    Output(OutputBuiltinRunner),
    RangeCheck(RangeCheckBuiltinRunner),
    Keccak(KeccakBuiltinRunner),
    Signature(SignatureBuiltinRunner),
}

impl BuiltinRunner {
    ///Creates the necessary segments for the builtin in the MemorySegmentManager and stores the first address on the builtin's base
    pub fn initialize_segments(
        &mut self,
        segments: &mut MemorySegmentManager,
        memory: &mut Memory,
    ) {
        match *self {
            BuiltinRunner::Bitwise(ref mut bitwise) => {
                bitwise.initialize_segments(segments, memory)
            }
            BuiltinRunner::EcOp(ref mut ec) => ec.initialize_segments(segments, memory),
            BuiltinRunner::Hash(ref mut hash) => hash.initialize_segments(segments, memory),
            BuiltinRunner::Output(ref mut output) => output.initialize_segments(segments, memory),
            BuiltinRunner::RangeCheck(ref mut range_check) => {
                range_check.initialize_segments(segments, memory)
            }
            BuiltinRunner::Keccak(ref mut keccak) => keccak.initialize_segments(segments, memory),
            BuiltinRunner::Signature(ref mut signature) => {
                signature.initialize_segments(segments, memory)
            }
        }
    }

    pub fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        match *self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.initial_stack(),
            BuiltinRunner::EcOp(ref ec) => ec.initial_stack(),
            BuiltinRunner::Hash(ref hash) => hash.initial_stack(),
            BuiltinRunner::Output(ref output) => output.initial_stack(),
            BuiltinRunner::RangeCheck(ref range_check) => range_check.initial_stack(),
            BuiltinRunner::Keccak(ref keccak) => keccak.initial_stack(),
            BuiltinRunner::Signature(ref signature) => signature.initial_stack(),
        }
    }

    pub fn final_stack(
        &mut self,
        segments: &MemorySegmentManager,
        memory: &Memory,
        stack_pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        match *self {
            BuiltinRunner::Bitwise(ref mut bitwise) => {
                bitwise.final_stack(segments, memory, stack_pointer)
            }
            BuiltinRunner::EcOp(ref mut ec) => ec.final_stack(segments, memory, stack_pointer),
            BuiltinRunner::Hash(ref mut hash) => hash.final_stack(segments, memory, stack_pointer),
            BuiltinRunner::Output(ref mut output) => {
                output.final_stack(segments, memory, stack_pointer)
            }
            BuiltinRunner::RangeCheck(ref mut range_check) => {
                range_check.final_stack(segments, memory, stack_pointer)
            }
            BuiltinRunner::Keccak(ref mut keccak) => {
                keccak.final_stack(segments, memory, stack_pointer)
            }
            BuiltinRunner::Signature(ref mut signature) => {
                signature.final_stack(segments, memory, stack_pointer)
            }
        }
    }

    ///Returns the builtin's allocated memory units
    pub fn get_allocated_memory_units(
        &self,
        vm: &VirtualMachine,
    ) -> Result<usize, memory_errors::MemoryError> {
        match *self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.get_allocated_memory_units(vm),
            BuiltinRunner::EcOp(ref ec) => ec.get_allocated_memory_units(vm),
            BuiltinRunner::Hash(ref hash) => hash.get_allocated_memory_units(vm),
            BuiltinRunner::Output(ref output) => output.get_allocated_memory_units(vm),
            BuiltinRunner::RangeCheck(ref range_check) => {
                range_check.get_allocated_memory_units(vm)
            }
            BuiltinRunner::Keccak(ref keccak) => keccak.get_allocated_memory_units(vm),
            BuiltinRunner::Signature(ref signature) => signature.get_allocated_memory_units(vm),
        }
    }

    ///Returns the builtin's base
    pub fn base(&self) -> isize {
        match *self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.base(),
            BuiltinRunner::EcOp(ref ec) => ec.base(),
            BuiltinRunner::Hash(ref hash) => hash.base(),
            BuiltinRunner::Output(ref output) => output.base(),
            BuiltinRunner::RangeCheck(ref range_check) => range_check.base(),
            BuiltinRunner::Keccak(ref keccak) => keccak.base(),
            BuiltinRunner::Signature(ref signature) => signature.base(),
        }
    }

    pub fn ratio(&self) -> Option<u32> {
        match self {
            BuiltinRunner::Bitwise(bitwise) => Some(bitwise.ratio()),
            BuiltinRunner::EcOp(ec) => Some(ec.ratio()),
            BuiltinRunner::Hash(hash) => Some(hash.ratio()),
            BuiltinRunner::Output(_) => None,
            BuiltinRunner::RangeCheck(range_check) => Some(range_check.ratio()),
            BuiltinRunner::Keccak(keccak) => Some(keccak.ratio()),
            BuiltinRunner::Signature(ref signature) => Some(signature.ratio()),
        }
    }

    pub fn add_validation_rule(&self, memory: &mut Memory) -> Result<(), RunnerError> {
        match *self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.add_validation_rule(memory),
            BuiltinRunner::EcOp(ref ec) => ec.add_validation_rule(memory),
            BuiltinRunner::Hash(ref hash) => hash.add_validation_rule(memory),
            BuiltinRunner::Output(ref output) => output.add_validation_rule(memory),
            BuiltinRunner::RangeCheck(ref range_check) => range_check.add_validation_rule(memory),
            BuiltinRunner::Keccak(ref keccak) => keccak.add_validation_rule(memory),
            BuiltinRunner::Signature(ref signature) => signature.add_validation_rule(memory),
        }
    }

    pub fn deduce_memory_cell(
        &self,
        address: &Relocatable,
        memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        match *self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.deduce_memory_cell(address, memory),
            BuiltinRunner::EcOp(ref ec) => ec.deduce_memory_cell(address, memory),
            BuiltinRunner::Hash(ref hash) => hash.deduce_memory_cell(address, memory),
            BuiltinRunner::Output(ref output) => output.deduce_memory_cell(address, memory),
            BuiltinRunner::RangeCheck(ref range_check) => {
                range_check.deduce_memory_cell(address, memory)
            }
            BuiltinRunner::Keccak(ref keccak) => keccak.deduce_memory_cell(address, memory),
            BuiltinRunner::Signature(ref signature) => {
                signature.deduce_memory_cell(address, memory)
            }
        }
    }

    pub fn get_memory_accesses(
        &self,
        vm: &VirtualMachine,
    ) -> Result<Vec<Relocatable>, MemoryError> {
        let base = self.base();
        let segment_size = vm
            .segments
            .get_segment_size(
                base.try_into()
                    .map_err(|_| MemoryError::AddressInTemporarySegment(base))?,
            )
            .ok_or(MemoryError::MissingSegmentUsedSizes)?;

        Ok((0..segment_size).map(|i| (base, i).into()).collect())
    }

    pub fn get_memory_segment_addresses(&self) -> (isize, Option<usize>) {
        match self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.get_memory_segment_addresses(),
            BuiltinRunner::EcOp(ref ec) => ec.get_memory_segment_addresses(),
            BuiltinRunner::Hash(ref hash) => hash.get_memory_segment_addresses(),
            BuiltinRunner::Output(ref output) => output.get_memory_segment_addresses(),
            BuiltinRunner::RangeCheck(ref range_check) => {
                range_check.get_memory_segment_addresses()
            }
            BuiltinRunner::Keccak(ref keccak) => keccak.get_memory_segment_addresses(),
            BuiltinRunner::Signature(ref signature) => signature.get_memory_segment_addresses(),
        }
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        match self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.get_used_cells(segments),
            BuiltinRunner::EcOp(ref ec) => ec.get_used_cells(segments),
            BuiltinRunner::Hash(ref hash) => hash.get_used_cells(segments),
            BuiltinRunner::Output(ref output) => output.get_used_cells(segments),
            BuiltinRunner::RangeCheck(ref range_check) => range_check.get_used_cells(segments),
            BuiltinRunner::Keccak(ref keccak) => keccak.get_used_cells(segments),
            BuiltinRunner::Signature(ref signature) => signature.get_used_cells(segments),
        }
    }

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        match self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.get_used_instances(segments),
            BuiltinRunner::EcOp(ref ec) => ec.get_used_instances(segments),
            BuiltinRunner::Hash(ref hash) => hash.get_used_instances(segments),
            BuiltinRunner::Output(ref output) => output.get_used_instances(segments),
            BuiltinRunner::RangeCheck(ref range_check) => range_check.get_used_instances(segments),
            BuiltinRunner::Keccak(ref keccak) => keccak.get_used_instances(segments),
            BuiltinRunner::Signature(ref signature) => signature.get_used_instances(segments),
        }
    }

    pub fn get_range_check_usage(&self, memory: &Memory) -> Option<(usize, usize)> {
        match self {
            BuiltinRunner::RangeCheck(ref range_check) => range_check.get_range_check_usage(memory),
            _ => None,
        }
    }

    /// Returns the number of range check units used by the builtin.
    pub fn get_used_perm_range_check_units(
        &self,
        vm: &VirtualMachine,
    ) -> Result<usize, MemoryError> {
        match self {
            BuiltinRunner::RangeCheck(range_check) => {
                range_check.get_used_perm_range_check_units(vm)
            }
            _ => Ok(0),
        }
    }

    pub fn get_used_diluted_check_units(&self, diluted_spacing: u32, diluted_n_bits: u32) -> usize {
        match self {
            BuiltinRunner::Bitwise(ref bitwise) => {
                bitwise.get_used_diluted_check_units(diluted_spacing, diluted_n_bits)
            }
            BuiltinRunner::Keccak(ref keccak) => {
                keccak.get_used_diluted_check_units(diluted_n_bits)
            }
            _ => 0,
        }
    }

    fn cells_per_instance(&self) -> u32 {
        match self {
            BuiltinRunner::Bitwise(builtin) => builtin.cells_per_instance,
            BuiltinRunner::EcOp(builtin) => builtin.cells_per_instance,
            BuiltinRunner::Hash(builtin) => builtin.cells_per_instance,
            BuiltinRunner::RangeCheck(builtin) => builtin.cells_per_instance,
            BuiltinRunner::Output(_) => 0,
            BuiltinRunner::Keccak(builtin) => builtin.cells_per_instance,
            BuiltinRunner::Signature(builtin) => builtin.cells_per_instance,
        }
    }

    fn n_input_cells(&self) -> u32 {
        match self {
            BuiltinRunner::Bitwise(builtin) => builtin.n_input_cells,
            BuiltinRunner::EcOp(builtin) => builtin.n_input_cells,
            BuiltinRunner::Hash(builtin) => builtin.n_input_cells,
            BuiltinRunner::RangeCheck(builtin) => builtin.n_input_cells,
            BuiltinRunner::Output(_) => 0,
            BuiltinRunner::Keccak(builtin) => builtin.n_input_cells,
            BuiltinRunner::Signature(builtin) => builtin.n_input_cells,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BuiltinRunner::Bitwise(_) => "bitwise",
            BuiltinRunner::EcOp(_) => "ec_op",
            BuiltinRunner::Hash(_) => "hash",
            BuiltinRunner::RangeCheck(_) => "range_check",
            BuiltinRunner::Output(_) => "output",
            BuiltinRunner::Keccak(_) => "keccak",
            BuiltinRunner::Signature(_) => "ecdsa",
        }
    }

    pub fn run_security_checks(&self, vm: &VirtualMachine) -> Result<(), VirtualMachineError> {
        if let BuiltinRunner::Output(_) = self {
            return Ok(());
        }
        let cells_per_instance = self.cells_per_instance() as usize;
        let n_input_cells = self.n_input_cells() as usize;
        let builtin_segment_index = self
            .base()
            .to_usize()
            .ok_or(VirtualMachineError::NegBuiltinBase)?;
        // If the builtin's segment is empty, there are no security checks to run
        let builtin_segment = match vm.memory.data.get(builtin_segment_index) {
            Some(segment) if !segment.is_empty() => segment,
            _ => return Ok(()),
        };
        // The builtin segment's size - 1 is the maximum offset within the segment's addresses
        // Assumption: The last element is not a None value
        // It is safe to asume this for normal program execution
        // If there are trailing None values at the end, the following security checks will fail
        let offset_max = builtin_segment.len().saturating_sub(1);
        // offset_len is the amount of non-None values in the segment
        let offset_len = builtin_segment.iter().filter(|x| x.is_some()).count();
        let n = match offset_len {
            0 => 0,
            _ => div_floor(offset_max, cells_per_instance) + 1,
        };
        // Verify that n is not too large to make sure the expected_offsets set that is constructed
        // below is not too large.
        if n > div_floor(offset_len, n_input_cells) {
            return Err(MemoryError::MissingMemoryCells(self.name()).into());
        }
        // Check that the two inputs (x and y) of each instance are set.
        let mut missing_offsets = Vec::with_capacity(n);
        // Check for missing expected offsets (either their address is no present, or their value is None)
        for i in 0..n {
            for j in 0..n_input_cells {
                let offset = cells_per_instance * i + j;
                if let None | Some(None) = builtin_segment.get(offset) {
                    missing_offsets.push(offset)
                }
            }
        }
        if !missing_offsets.is_empty() {
            return Err(
                MemoryError::MissingMemoryCellsWithOffsets(self.name(), missing_offsets).into(),
            );
        }
        // Verify auto deduction rules for the unasigned output cells
        // Assigned output cells are checked as part of the call to verify_auto_deductions().
        for i in 0..n {
            for j in n_input_cells..cells_per_instance {
                let offset = cells_per_instance * i + j;
                if let None | Some(None) = builtin_segment.get(offset) {
                    vm.verify_auto_deductions_for_addr(
                        &Relocatable::from((builtin_segment_index as isize, offset)),
                        self,
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn get_used_cells_and_allocated_size(
        &self,
        vm: &VirtualMachine,
    ) -> Result<(usize, usize), MemoryError> {
        match self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.get_used_cells_and_allocated_size(vm),
            BuiltinRunner::EcOp(ref ec) => ec.get_used_cells_and_allocated_size(vm),
            BuiltinRunner::Hash(ref hash) => hash.get_used_cells_and_allocated_size(vm),
            BuiltinRunner::Output(ref output) => output.get_used_cells_and_allocated_size(vm),
            BuiltinRunner::RangeCheck(ref range_check) => {
                range_check.get_used_cells_and_allocated_size(vm)
            }
            BuiltinRunner::Keccak(ref keccak) => keccak.get_used_cells_and_allocated_size(vm),
            BuiltinRunner::Signature(ref signature) => {
                signature.get_used_cells_and_allocated_size(vm)
            }
        }
    }

    pub fn set_stop_ptr(&mut self, stop_ptr: usize) {
        match self {
            BuiltinRunner::Bitwise(ref mut bitwise) => bitwise.stop_ptr = Some(stop_ptr),
            BuiltinRunner::EcOp(ref mut ec) => ec.stop_ptr = Some(stop_ptr),
            BuiltinRunner::Hash(ref mut hash) => hash.stop_ptr = Some(stop_ptr),
            BuiltinRunner::Output(ref mut output) => output.stop_ptr = Some(stop_ptr),
            BuiltinRunner::RangeCheck(ref mut range_check) => range_check.stop_ptr = Some(stop_ptr),
            BuiltinRunner::Keccak(ref mut keccak) => keccak.stop_ptr = Some(stop_ptr),
            BuiltinRunner::Signature(ref mut signature) => signature.stop_ptr = Some(stop_ptr),
        }
    }
}

impl From<KeccakBuiltinRunner> for BuiltinRunner {
    fn from(runner: KeccakBuiltinRunner) -> Self {
        BuiltinRunner::Keccak(runner)
    }
}

impl From<BitwiseBuiltinRunner> for BuiltinRunner {
    fn from(runner: BitwiseBuiltinRunner) -> Self {
        BuiltinRunner::Bitwise(runner)
    }
}

impl From<EcOpBuiltinRunner> for BuiltinRunner {
    fn from(runner: EcOpBuiltinRunner) -> Self {
        BuiltinRunner::EcOp(runner)
    }
}

impl From<HashBuiltinRunner> for BuiltinRunner {
    fn from(runner: HashBuiltinRunner) -> Self {
        BuiltinRunner::Hash(runner)
    }
}

impl From<OutputBuiltinRunner> for BuiltinRunner {
    fn from(runner: OutputBuiltinRunner) -> Self {
        BuiltinRunner::Output(runner)
    }
}

impl From<RangeCheckBuiltinRunner> for BuiltinRunner {
    fn from(runner: RangeCheckBuiltinRunner) -> Self {
        BuiltinRunner::RangeCheck(runner)
    }
}

impl From<SignatureBuiltinRunner> for BuiltinRunner {
    fn from(runner: SignatureBuiltinRunner) -> Self {
        BuiltinRunner::Signature(runner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::relocatable;
    use crate::types::instance_definitions::ecdsa_instance_def::EcdsaInstanceDef;
    use crate::types::instance_definitions::keccak_instance_def::KeccakInstanceDef;
    use crate::types::program::Program;
    use crate::vm::runners::cairo_runner::CairoRunner;
    use crate::{
        types::instance_definitions::{
            bitwise_instance_def::BitwiseInstanceDef, ec_op_instance_def::EcOpInstanceDef,
        },
        utils::test_utils::*,
        vm::vm_core::VirtualMachine,
    };

    #[test]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    fn get_memory_accesses_empty() {
        let builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    fn get_memory_accesses() {
        let builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Ok(vec![
                (builtin.base(), 0).into(),
                (builtin.base(), 1).into(),
                (builtin.base(), 2).into(),
                (builtin.base(), 3).into(),
            ]),
        );
    }

    #[test]
    fn get_n_input_cells_bitwise() {
        let bitwise = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), true);
        let builtin: BuiltinRunner = bitwise.clone().into();
        assert_eq!(bitwise.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    fn get_n_input_cells_hash() {
        let hash = HashBuiltinRunner::new(10, true);
        let builtin: BuiltinRunner = hash.clone().into();
        assert_eq!(hash.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    fn get_n_input_cells_range_check() {
        let range_check = RangeCheckBuiltinRunner::new(10, 10, true);
        let builtin: BuiltinRunner = range_check.clone().into();
        assert_eq!(range_check.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    fn get_n_input_cells_ec_op() {
        let ec_op = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);
        let builtin: BuiltinRunner = ec_op.clone().into();
        assert_eq!(ec_op.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    fn get_n_input_cells_ecdsa() {
        let signature = SignatureBuiltinRunner::new(&EcdsaInstanceDef::new(10), true);
        let builtin: BuiltinRunner = signature.clone().into();
        assert_eq!(signature.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    fn get_n_input_cells_output() {
        let output = OutputBuiltinRunner::new(true);
        let builtin: BuiltinRunner = output.into();
        assert_eq!(0, builtin.n_input_cells())
    }

    #[test]
    fn get_cells_per_instance_bitwise() {
        let bitwise = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), true);
        let builtin: BuiltinRunner = bitwise.clone().into();
        assert_eq!(bitwise.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    fn get_cells_per_instance_hash() {
        let hash = HashBuiltinRunner::new(10, true);
        let builtin: BuiltinRunner = hash.clone().into();
        assert_eq!(hash.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    fn get_cells_per_instance_range_check() {
        let range_check = RangeCheckBuiltinRunner::new(10, 10, true);
        let builtin: BuiltinRunner = range_check.clone().into();
        assert_eq!(range_check.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    fn get_cells_per_instance_ec_op() {
        let ec_op = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);
        let builtin: BuiltinRunner = ec_op.clone().into();
        assert_eq!(ec_op.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    fn get_cells_per_instance_ecdsa() {
        let signature = SignatureBuiltinRunner::new(&EcdsaInstanceDef::new(10), true);
        let builtin: BuiltinRunner = signature.clone().into();
        assert_eq!(signature.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    fn get_cells_per_instance_output() {
        let output = OutputBuiltinRunner::new(true);
        let builtin: BuiltinRunner = output.into();
        assert_eq!(0, builtin.cells_per_instance())
    }

    #[test]
    fn get_name_bitwise() {
        let bitwise = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), true);
        let builtin: BuiltinRunner = bitwise.into();
        assert_eq!("bitwise", builtin.name())
    }

    #[test]
    fn get_name_hash() {
        let hash = HashBuiltinRunner::new(10, true);
        let builtin: BuiltinRunner = hash.into();
        assert_eq!("hash", builtin.name())
    }

    #[test]
    fn get_name_range_check() {
        let range_check = RangeCheckBuiltinRunner::new(10, 10, true);
        let builtin: BuiltinRunner = range_check.into();
        assert_eq!("range_check", builtin.name())
    }

    #[test]
    fn get_name_ec_op() {
        let ec_op = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);
        let builtin: BuiltinRunner = ec_op.into();
        assert_eq!("ec_op", builtin.name())
    }

    #[test]
    fn get_name_ecdsa() {
        let signature = SignatureBuiltinRunner::new(&EcdsaInstanceDef::new(10), true);
        let builtin: BuiltinRunner = signature.into();
        assert_eq!("ecdsa", builtin.name())
    }

    #[test]
    fn get_name_output() {
        let output = OutputBuiltinRunner::new(true);
        let builtin: BuiltinRunner = output.into();
        assert_eq!("output", builtin.name())
    }

    #[test]
    fn get_allocated_memory_units_bitwise_with_items() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::new(10),
            true,
        ));

        let mut vm = vm!();

        let program = program!(
            builtins = vec![String::from("bitwise")],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut cairo_runner = cairo_runner!(program);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(&mut vm).unwrap();

        cairo_runner
            .run_until_pc(address, &mut vm, &mut hint_processor)
            .unwrap();

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(5));
    }

    #[test]
    fn get_allocated_memory_units_ec_op_with_items() {
        let builtin = BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::new(10), true));

        let mut vm = vm!();

        let program = program!(
            builtins = vec![String::from("ec_op")],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut cairo_runner = cairo_runner!(program);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(&mut vm).unwrap();

        cairo_runner
            .run_until_pc(address, &mut vm, &mut hint_processor)
            .unwrap();

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(7));
    }

    #[test]
    fn get_allocated_memory_units_hash_with_items() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(10, true));

        let mut vm = vm!();

        let program = program!(
            builtins = vec![String::from("pedersen")],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut cairo_runner = cairo_runner!(program);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(&mut vm).unwrap();

        cairo_runner
            .run_until_pc(address, &mut vm, &mut hint_processor)
            .unwrap();

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(3));
    }

    #[test]
    fn get_allocated_memory_units_range_check_with_items() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(10, 12, true));

        let mut vm = vm!();

        let program = program!(
            builtins = vec![String::from("range_check")],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut cairo_runner = cairo_runner!(program);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(&mut vm).unwrap();

        cairo_runner
            .run_until_pc(address, &mut vm, &mut hint_processor)
            .unwrap();

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(1));
    }

    #[test]
    fn get_allocated_memory_units_keccak_with_items() {
        let builtin = BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
            &KeccakInstanceDef::new(10, vec![200; 8]),
            true,
        ));

        let mut vm = vm!();

        let program = program!(
            builtins = vec![String::from("keccak")],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut cairo_runner = cairo_runner!(program, "recursive");

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(&mut vm).unwrap();

        cairo_runner
            .run_until_pc(address, &mut vm, &mut hint_processor)
            .unwrap();

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(16));
    }

    #[test]
    fn get_allocated_memory_units_output() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let vm = vm!();

        // In this case, the function always return Ok(0)
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(0));
    }

    #[test]
    fn get_allocated_memory_units_range_check() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(8, 8, true));
        let vm = vm!();
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(0));
    }

    #[test]
    fn get_allocated_memory_units_hash() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(1, true));
        let vm = vm!();
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(0));
    }

    #[test]
    fn get_allocated_memory_units_bitwise() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let vm = vm!();
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(0));
    }

    #[test]
    fn get_allocated_memory_units_ec_op() {
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
        let vm = vm!();
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(0));
    }

    #[test]
    fn get_allocated_memory_units_keccak() {
        let builtin = BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
            &KeccakInstanceDef::default(),
            true,
        ));
        let vm = vm!();
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(0));
    }

    #[test]
    fn get_range_check_usage_range_check() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(8, 8, true));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), Some((1, 4)));
    }

    #[test]
    fn get_range_check_usage_output() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), None);
    }

    #[test]
    fn get_range_check_usage_hash() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(256, true));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), None);
    }

    #[test]
    fn get_range_check_usage_ec_op() {
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), None);
    }

    #[test]
    fn get_range_check_usage_bitwise() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), None);
    }

    #[test]
    fn get_used_diluted_check_units_bitwise() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 1255);
    }

    #[test]
    fn get_used_diluted_check_units_keccak_zero_case() {
        let builtin = BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
            &KeccakInstanceDef::default(),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    fn get_used_diluted_check_units_keccak_non_zero_case() {
        let builtin = BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
            &KeccakInstanceDef::default(),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(0, 8), 32768);
    }

    #[test]
    fn get_used_diluted_check_units_ec_op() {
        let builtin = BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::new(10), true));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    fn get_used_diluted_check_units_hash() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(16, true));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    fn get_used_diluted_check_units_range_check() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(8, 8, true));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    fn get_used_diluted_check_units_output() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    fn get_memory_segment_addresses_test() {
        let bitwise_builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        assert_eq!(bitwise_builtin.get_memory_segment_addresses(), (0, None),);
        let ec_op_builtin: BuiltinRunner =
            EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true).into();
        assert_eq!(ec_op_builtin.get_memory_segment_addresses(), (0, None),);
        let hash_builtin: BuiltinRunner = HashBuiltinRunner::new(8, true).into();
        assert_eq!(hash_builtin.get_memory_segment_addresses(), (0, None),);
        let output_builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();
        assert_eq!(output_builtin.get_memory_segment_addresses(), (0, None),);
        let range_check_builtin: BuiltinRunner =
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(8, 8, true));
        assert_eq!(
            range_check_builtin.get_memory_segment_addresses(),
            (0, None),
        );
    }

    #[test]
    fn run_security_checks_for_output() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let vm = vm!();

        assert_eq!(builtin.run_security_checks(&vm), Ok(()));
    }

    #[test]
    fn run_security_checks_empty_memory() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let vm = vm!();
        // Unused builtin shouldn't fail security checks
        assert_eq!(builtin.run_security_checks(&vm), Ok(()),);
    }

    #[test]
    fn run_security_checks_temporary_segment() {
        let builtin = BuiltinRunner::Bitwise({
            let mut builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);
            builtin.base = -1;
            builtin
        });
        let vm = vm!();

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::NegBuiltinBase),
        );
    }

    #[test]
    fn run_security_checks_empty_offsets() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.memory.data = vec![vec![]];

        assert_eq!(builtin.run_security_checks(&vm), Ok(()));
    }

    #[test]
    fn run_security_checks_bitwise_missing_memory_cells_with_offsets() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.memory.data = vec![vec![
            None,
            mayberelocatable!(0, 1).into(),
            mayberelocatable!(0, 2).into(),
            mayberelocatable!(0, 3).into(),
            mayberelocatable!(0, 4).into(),
        ]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCellsWithOffsets("bitwise", vec![0],).into()),
        );
    }

    #[test]
    fn run_security_checks_bitwise_missing_memory_cells() {
        let mut bitwise_builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);

        bitwise_builtin.cells_per_instance = 2;
        bitwise_builtin.n_input_cells = 5;

        let builtin: BuiltinRunner = bitwise_builtin.into();

        let mut vm = vm!();

        vm.memory.data = vec![vec![
            mayberelocatable!(0, 0).into(),
            mayberelocatable!(0, 1).into(),
            mayberelocatable!(0, 2).into(),
            mayberelocatable!(0, 3).into(),
            mayberelocatable!(0, 4).into(),
            mayberelocatable!(0, 5).into(),
        ]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCells("bitwise").into()),
        );
    }

    #[test]
    fn run_security_checks_hash_missing_memory_cells_with_offsets() {
        let builtin: BuiltinRunner = HashBuiltinRunner::new(8, true).into();
        let mut vm = vm!();

        vm.memory.data = vec![vec![
            None,
            mayberelocatable!(0, 1).into(),
            mayberelocatable!(0, 2).into(),
            mayberelocatable!(0, 3).into(),
            mayberelocatable!(0, 4).into(),
            mayberelocatable!(0, 5).into(),
        ]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCellsWithOffsets("hash", vec![0],).into()),
        );
    }

    #[test]
    fn run_security_checks_hash_missing_memory_cells() {
        let hash_builtin = HashBuiltinRunner::new(8, true);

        let builtin: BuiltinRunner = hash_builtin.into();

        let mut vm = vm!();

        vm.memory.data = vec![vec![mayberelocatable!(0, 0).into()]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCells("hash").into()),
        );
    }

    #[test]
    fn run_security_checks_range_check_missing_memory_cells_with_offsets() {
        let range_check_builtin = RangeCheckBuiltinRunner::new(8, 8, true);
        let builtin: BuiltinRunner = range_check_builtin.into();
        let mut vm = vm!();

        vm.memory.data = vec![vec![
            None,
            mayberelocatable!(100).into(),
            mayberelocatable!(2).into(),
            mayberelocatable!(3).into(),
            None,
            mayberelocatable!(5).into(),
            mayberelocatable!(17).into(),
            mayberelocatable!(22).into(),
        ]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCells("range_check").into()),
        );
    }

    #[test]
    fn run_security_checks_range_check_missing_memory_cells() {
        let builtin: BuiltinRunner =
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(8, 8, true));
        let mut vm = vm!();

        vm.memory.data = vec![vec![None, mayberelocatable!(0).into()]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCells("range_check").into()),
        );
    }

    #[test]
    fn run_security_checks_range_check_empty() {
        let range_check_builtin = RangeCheckBuiltinRunner::new(8, 8, true);

        let builtin: BuiltinRunner = range_check_builtin.into();

        let mut vm = vm!();

        vm.memory.data = vec![vec![None, None, None]];

        assert_eq!(builtin.run_security_checks(&vm), Ok(()),);
    }

    #[test]
    fn run_security_checks_validate_auto_deductions() {
        let builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();

        let mut vm = vm!();
        vm.memory.validated_addresses.insert(relocatable!(0, 2));

        vm.memory.data = vec![vec![
            mayberelocatable!(0, 0).into(),
            mayberelocatable!(0, 1).into(),
            mayberelocatable!(0, 2).into(),
            mayberelocatable!(0, 3).into(),
            mayberelocatable!(0, 4).into(),
        ]];

        assert_eq!(builtin.run_security_checks(&vm), Ok(()));
    }

    #[test]
    fn run_security_ec_op_check_memory_empty() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let builtin: BuiltinRunner = ec_op_builtin.into();

        let mut vm = vm!();
        // The values stored in memory are not relevant for this test
        vm.memory.data = vec![vec![]];

        assert_eq!(builtin.run_security_checks(&vm), Ok(()),);
    }

    #[test]
    fn run_security_ec_op_check_memory_1_element() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let builtin: BuiltinRunner = ec_op_builtin.into();

        let mut vm = vm!();
        // The values stored in memory are not relevant for this test
        vm.memory.data = vec![vec![mayberelocatable!(0).into()]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCells("ec_op").into()),
        );
    }

    #[test]
    fn run_security_ec_op_check_memory_3_elements() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let builtin: BuiltinRunner = ec_op_builtin.into();

        let mut vm = vm!();
        // The values stored in memory are not relevant for this test
        vm.memory.data = vec![vec![
            mayberelocatable!(0).into(),
            mayberelocatable!(0).into(),
            mayberelocatable!(0).into(),
        ]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCells("ec_op").into()),
        );
    }

    #[test]
    fn run_security_ec_op_missing_memory_cells_with_offsets() {
        let builtin: BuiltinRunner =
            EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.memory.data = vec![vec![
            None,
            mayberelocatable!(0, 1).into(),
            mayberelocatable!(0, 2).into(),
            mayberelocatable!(0, 3).into(),
            mayberelocatable!(0, 4).into(),
            mayberelocatable!(0, 5).into(),
            mayberelocatable!(0, 6).into(),
        ]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCellsWithOffsets("ec_op", vec![0],).into()),
        );
    }

    #[test]
    fn run_security_ec_op_check_memory_gap() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let builtin: BuiltinRunner = ec_op_builtin.into();

        let mut vm = vm!();
        // The values stored in memory are not relevant for this test
        vm.memory.data = vec![vec![
            mayberelocatable!(0).into(),
            mayberelocatable!(1).into(),
            mayberelocatable!(2).into(),
            mayberelocatable!(3).into(),
            mayberelocatable!(4).into(),
            mayberelocatable!(5).into(),
            mayberelocatable!(6).into(),
            None,
            mayberelocatable!(8).into(),
            mayberelocatable!(9).into(),
            mayberelocatable!(10).into(),
            mayberelocatable!(11).into(),
        ]];

        assert_eq!(
            builtin.run_security_checks(&vm),
            Err(MemoryError::MissingMemoryCellsWithOffsets("ec_op", vec![7]).into()),
        );
    }

    /// Test that get_used_perm_range_check_units() returns zero when the
    /// builtin is a BitwiseBuiltinRunner.
    #[test]
    fn get_used_perm_range_check_units_bitwise() {
        let builtin_runner: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.current_step = 8;
        vm.segments.segment_used_sizes = Some(vec![5]);
        assert_eq!(builtin_runner.get_used_perm_range_check_units(&vm), Ok(0));
    }

    /// Test that get_used_perm_range_check_units() returns zero when the
    /// builtin is an EcOpBuiltinRunner.
    #[test]
    fn get_used_perm_range_check_units_ec_op() {
        let builtin_runner: BuiltinRunner =
            EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.current_step = 8;
        vm.segments.segment_used_sizes = Some(vec![5]);
        assert_eq!(builtin_runner.get_used_perm_range_check_units(&vm), Ok(0));
    }

    /// Test that get_used_perm_range_check_units() returns zero when the
    /// builtin is a HashBuiltinRunner.
    #[test]
    fn get_used_perm_range_check_units_hash() {
        let builtin_runner: BuiltinRunner = HashBuiltinRunner::new(8, true).into();
        let mut vm = vm!();

        vm.current_step = 8;
        vm.segments.segment_used_sizes = Some(vec![5]);
        assert_eq!(builtin_runner.get_used_perm_range_check_units(&vm), Ok(0));
    }

    /// Test that get_used_perm_range_check_units() returns zero when the
    /// builtin is an OutputBuiltinRunner.
    #[test]
    fn get_used_perm_range_check_units_output() {
        let builtin_runner: BuiltinRunner = OutputBuiltinRunner::new(true).into();
        let mut vm = vm!();

        vm.current_step = 8;
        vm.segments.segment_used_sizes = Some(vec![5]);
        assert_eq!(builtin_runner.get_used_perm_range_check_units(&vm), Ok(0));
    }

    /// Test that get_used_perm_range_check_units() calls the corresponding
    /// method when the builtin is a RangeCheckBuiltinRunner.
    #[test]
    fn get_used_perm_range_check_units_range_check() {
        let builtin_runner: BuiltinRunner = RangeCheckBuiltinRunner::new(8, 8, true).into();
        let mut vm = vm!();

        vm.current_step = 8;
        vm.segments.segment_used_sizes = Some(vec![1]);
        assert_eq!(builtin_runner.get_used_perm_range_check_units(&vm), Ok(8));
    }

    #[test]
    fn get_ratio_tests() {
        let bitwise_builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        assert_eq!(bitwise_builtin.ratio(), (Some(256)),);
        let ec_op_builtin: BuiltinRunner =
            EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true).into();
        assert_eq!(ec_op_builtin.ratio(), (Some(256)),);
        let hash_builtin: BuiltinRunner = HashBuiltinRunner::new(8, true).into();
        assert_eq!(hash_builtin.ratio(), (Some(8)),);
        let output_builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();
        assert_eq!(output_builtin.ratio(), None,);
        let range_check_builtin: BuiltinRunner =
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(8, 8, true));
        assert_eq!(range_check_builtin.ratio(), (Some(8)),);
        let keccak_builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true).into();
        assert_eq!(keccak_builtin.ratio(), (Some(2048)),);
    }

    #[test]
    fn bitwise_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let bitwise_builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        assert_eq!(bitwise_builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    fn ec_op_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let ec_op_builtin: BuiltinRunner =
            EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true).into();
        assert_eq!(ec_op_builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    fn hash_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let hash_builtin: BuiltinRunner = HashBuiltinRunner::new(8, true).into();
        assert_eq!(hash_builtin.get_used_instances(&vm.segments), Ok(2));
    }

    #[test]
    fn output_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let output_builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();
        assert_eq!(output_builtin.get_used_instances(&vm.segments), Ok(4));
    }
    #[test]
    fn range_check_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let range_check_builtin: BuiltinRunner =
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(8, 8, true));
        assert_eq!(range_check_builtin.get_used_instances(&vm.segments), Ok(4));
    }

    #[test]
    fn runners_final_stack() {
        let mut builtins = vec![
            BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
                &BitwiseInstanceDef::default(),
                false,
            )),
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), false)),
            BuiltinRunner::Hash(HashBuiltinRunner::new(1, false)),
            BuiltinRunner::Output(OutputBuiltinRunner::new(false)),
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(8, 8, false)),
            BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
                &KeccakInstanceDef::default(),
                false,
            )),
            BuiltinRunner::Signature(SignatureBuiltinRunner::new(
                &EcdsaInstanceDef::default(),
                false,
            )),
        ];
        let vm = vm!();

        for br in builtins.iter_mut() {
            assert_eq!(
                br.final_stack(&vm.segments, &vm.memory, vm.get_ap()),
                Ok(vm.get_ap())
            );
        }
    }

    #[test]
    fn runners_set_stop_ptr() {
        let builtins = vec![
            BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
                &BitwiseInstanceDef::default(),
                false,
            )),
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), false)),
            BuiltinRunner::Hash(HashBuiltinRunner::new(1, false)),
            BuiltinRunner::Output(OutputBuiltinRunner::new(false)),
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(8, 8, false)),
            BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
                &KeccakInstanceDef::default(),
                false,
            )),
            BuiltinRunner::Signature(SignatureBuiltinRunner::new(
                &EcdsaInstanceDef::default(),
                false,
            )),
        ];

        let ptr = 3;

        for mut br in builtins {
            br.set_stop_ptr(ptr);
            let (_, stop_ptr) = br.get_memory_segment_addresses();
            assert_eq!(stop_ptr, Some(ptr));
        }
    }
}
