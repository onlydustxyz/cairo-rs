use crate::math_utils::safe_div_usize;
use crate::stdlib::prelude::*;
use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::{self, InsufficientAllocatedCellsError, MemoryError};
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
mod poseidon;
mod range_check;
mod segment_arena;
mod signature;

pub use self::keccak::KeccakBuiltinRunner;
pub use self::poseidon::PoseidonBuiltinRunner;
pub use self::segment_arena::SegmentArenaBuiltinRunner;
pub use bitwise::BitwiseBuiltinRunner;
pub use ec_op::EcOpBuiltinRunner;
pub use hash::HashBuiltinRunner;
use num_integer::div_floor;
pub use output::OutputBuiltinRunner;
pub use range_check::RangeCheckBuiltinRunner;
pub use signature::SignatureBuiltinRunner;

pub const OUTPUT_BUILTIN_NAME: &str = "output_builtin";
pub const HASH_BUILTIN_NAME: &str = "pedersen_builtin";
pub const RANGE_CHECK_BUILTIN_NAME: &str = "range_check_builtin";
pub const SIGNATURE_BUILTIN_NAME: &str = "ecdsa_builtin";
pub const BITWISE_BUILTIN_NAME: &str = "bitwise_builtin";
pub const EC_OP_BUILTIN_NAME: &str = "ec_op_builtin";
pub const KECCAK_BUILTIN_NAME: &str = "keccak_builtin";
pub const POSEIDON_BUILTIN_NAME: &str = "poseidon_builtin";
pub const SEGMENT_ARENA_BUILTIN_NAME: &str = "segment_arena_builtin";

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
    Poseidon(PoseidonBuiltinRunner),
    SegmentArena(SegmentArenaBuiltinRunner),
}

impl BuiltinRunner {
    ///Creates the necessary segments for the builtin in the MemorySegmentManager and stores the first address on the builtin's base
    pub fn initialize_segments(&mut self, segments: &mut MemorySegmentManager) {
        match *self {
            BuiltinRunner::Bitwise(ref mut bitwise) => bitwise.initialize_segments(segments),
            BuiltinRunner::EcOp(ref mut ec) => ec.initialize_segments(segments),
            BuiltinRunner::Hash(ref mut hash) => hash.initialize_segments(segments),
            BuiltinRunner::Output(ref mut output) => output.initialize_segments(segments),
            BuiltinRunner::RangeCheck(ref mut range_check) => {
                range_check.initialize_segments(segments)
            }
            BuiltinRunner::Keccak(ref mut keccak) => keccak.initialize_segments(segments),
            BuiltinRunner::Signature(ref mut signature) => signature.initialize_segments(segments),
            BuiltinRunner::Poseidon(ref mut poseidon) => poseidon.initialize_segments(segments),
            BuiltinRunner::SegmentArena(ref mut segment_arena) => {
                segment_arena.initialize_segments(segments)
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
            BuiltinRunner::Poseidon(ref poseidon) => poseidon.initial_stack(),
            BuiltinRunner::SegmentArena(ref segment_arena) => segment_arena.initial_stack(),
        }
    }

    ///Returns the builtin's final stack
    pub fn final_stack(
        &mut self,
        segments: &MemorySegmentManager,
        stack_pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        match self {
            BuiltinRunner::Bitwise(ref mut bitwise) => bitwise.final_stack(segments, stack_pointer),
            BuiltinRunner::EcOp(ref mut ec) => ec.final_stack(segments, stack_pointer),
            BuiltinRunner::Hash(ref mut hash) => hash.final_stack(segments, stack_pointer),
            BuiltinRunner::Output(ref mut output) => output.final_stack(segments, stack_pointer),
            BuiltinRunner::RangeCheck(ref mut range_check) => {
                range_check.final_stack(segments, stack_pointer)
            }
            BuiltinRunner::Keccak(ref mut keccak) => keccak.final_stack(segments, stack_pointer),
            BuiltinRunner::Signature(ref mut signature) => {
                signature.final_stack(segments, stack_pointer)
            }
            BuiltinRunner::Poseidon(ref mut poseidon) => {
                poseidon.final_stack(segments, stack_pointer)
            }
            BuiltinRunner::SegmentArena(ref mut segment_arena) => {
                segment_arena.final_stack(segments, stack_pointer)
            }
        }
    }

    ///Returns the builtin's allocated memory units
    pub fn get_allocated_memory_units(
        &self,
        vm: &VirtualMachine,
    ) -> Result<usize, memory_errors::MemoryError> {
        match *self {
            BuiltinRunner::Output(_) | BuiltinRunner::SegmentArena(_) => Ok(0),
            _ => {
                match self.ratio() {
                    None => {
                        // Dynamic layout has the exact number of instances it needs (up to a power of 2).
                        let instances: usize =
                            self.get_used_cells(&vm.segments)? / self.cells_per_instance() as usize;
                        let components = (instances / self.instances_per_component() as usize)
                            .next_power_of_two();
                        Ok(self.cells_per_instance() as usize
                            * self.instances_per_component() as usize
                            * components)
                    }
                    Some(ratio) => {
                        let min_step = (ratio * self.instances_per_component()) as usize;
                        if vm.current_step < min_step {
                            return Err(InsufficientAllocatedCellsError::MinStepNotReached(
                                min_step,
                                self.name(),
                            )
                            .into());
                        };
                        let value = safe_div_usize(vm.current_step, ratio as usize)
                            .map_err(|_| MemoryError::ErrorCalculatingMemoryUnits)?;
                        Ok(self.cells_per_instance() as usize * value)
                    }
                }
            }
        }
    }

    ///Returns the builtin's base
    pub fn base(&self) -> usize {
        match *self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.base(),
            BuiltinRunner::EcOp(ref ec) => ec.base(),
            BuiltinRunner::Hash(ref hash) => hash.base(),
            BuiltinRunner::Output(ref output) => output.base(),
            BuiltinRunner::RangeCheck(ref range_check) => range_check.base(),
            BuiltinRunner::Keccak(ref keccak) => keccak.base(),
            BuiltinRunner::Signature(ref signature) => signature.base(),
            BuiltinRunner::Poseidon(ref poseidon) => poseidon.base(),
            //Warning, returns only the segment index, base offset will be 3
            BuiltinRunner::SegmentArena(ref segment_arena) => segment_arena.base(),
        }
    }

    pub fn ratio(&self) -> Option<u32> {
        match self {
            BuiltinRunner::Bitwise(bitwise) => bitwise.ratio(),
            BuiltinRunner::EcOp(ec) => ec.ratio(),
            BuiltinRunner::Hash(hash) => hash.ratio(),
            BuiltinRunner::Output(_) | BuiltinRunner::SegmentArena(_) => None,
            BuiltinRunner::RangeCheck(range_check) => range_check.ratio(),
            BuiltinRunner::Keccak(keccak) => keccak.ratio(),
            BuiltinRunner::Signature(ref signature) => signature.ratio(),
            BuiltinRunner::Poseidon(poseidon) => poseidon.ratio(),
        }
    }

    pub fn add_validation_rule(&self, memory: &mut Memory) {
        match *self {
            BuiltinRunner::Bitwise(ref bitwise) => bitwise.add_validation_rule(memory),
            BuiltinRunner::EcOp(ref ec) => ec.add_validation_rule(memory),
            BuiltinRunner::Hash(ref hash) => hash.add_validation_rule(memory),
            BuiltinRunner::Output(ref output) => output.add_validation_rule(memory),
            BuiltinRunner::RangeCheck(ref range_check) => range_check.add_validation_rule(memory),
            BuiltinRunner::Keccak(ref keccak) => keccak.add_validation_rule(memory),
            BuiltinRunner::Signature(ref signature) => signature.add_validation_rule(memory),
            BuiltinRunner::Poseidon(ref poseidon) => poseidon.add_validation_rule(memory),
            BuiltinRunner::SegmentArena(ref segment_arena) => {
                segment_arena.add_validation_rule(memory)
            }
        }
    }

    pub fn deduce_memory_cell(
        &self,
        address: Relocatable,
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
            BuiltinRunner::Poseidon(ref poseidon) => poseidon.deduce_memory_cell(address, memory),
            BuiltinRunner::SegmentArena(ref segment_arena) => {
                segment_arena.deduce_memory_cell(address, memory)
            }
        }
    }

    pub fn get_memory_accesses(
        &self,
        vm: &VirtualMachine,
    ) -> Result<Vec<Relocatable>, MemoryError> {
        if let BuiltinRunner::SegmentArena(_) = self {
            return Ok(vec![]);
        }
        let base = self.base();
        let segment_size = vm
            .segments
            .get_segment_size(base)
            .ok_or(MemoryError::MissingSegmentUsedSizes)?;

        Ok((0..segment_size)
            .map(|i| (base as isize, i).into())
            .collect())
    }

    pub fn get_memory_segment_addresses(&self) -> (usize, Option<usize>) {
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
            BuiltinRunner::Poseidon(ref poseidon) => poseidon.get_memory_segment_addresses(),
            BuiltinRunner::SegmentArena(ref segment_arena) => {
                segment_arena.get_memory_segment_addresses()
            }
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
            BuiltinRunner::Poseidon(ref poseidon) => poseidon.get_used_cells(segments),
            BuiltinRunner::SegmentArena(ref segment_arena) => {
                segment_arena.get_used_cells(segments)
            }
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
            BuiltinRunner::Poseidon(ref poseidon) => poseidon.get_used_instances(segments),
            BuiltinRunner::SegmentArena(ref segment_arena) => {
                segment_arena.get_used_instances(segments)
            }
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
                let (used_cells, _) = self.get_used_cells_and_allocated_size(vm)?;
                Ok(used_cells * range_check.n_parts as usize)
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
            BuiltinRunner::Poseidon(builtin) => builtin.cells_per_instance,
            BuiltinRunner::SegmentArena(builtin) => builtin.cells_per_instance,
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
            BuiltinRunner::Poseidon(builtin) => builtin.n_input_cells,
            BuiltinRunner::SegmentArena(builtin) => builtin.n_input_cells_per_instance,
        }
    }

    fn instances_per_component(&self) -> u32 {
        match self {
            BuiltinRunner::Bitwise(builtin) => builtin.instances_per_component,
            BuiltinRunner::EcOp(builtin) => builtin.instances_per_component,
            BuiltinRunner::Hash(builtin) => builtin.instances_per_component,
            BuiltinRunner::RangeCheck(builtin) => builtin.instances_per_component,
            BuiltinRunner::Output(_) | BuiltinRunner::SegmentArena(_) => 1,
            BuiltinRunner::Keccak(builtin) => builtin.instances_per_component,
            BuiltinRunner::Signature(builtin) => builtin.instances_per_component,
            BuiltinRunner::Poseidon(builtin) => builtin.instances_per_component,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BuiltinRunner::Bitwise(_) => BITWISE_BUILTIN_NAME,
            BuiltinRunner::EcOp(_) => EC_OP_BUILTIN_NAME,
            BuiltinRunner::Hash(_) => HASH_BUILTIN_NAME,
            BuiltinRunner::RangeCheck(_) => RANGE_CHECK_BUILTIN_NAME,
            BuiltinRunner::Output(_) => OUTPUT_BUILTIN_NAME,
            BuiltinRunner::Keccak(_) => KECCAK_BUILTIN_NAME,
            BuiltinRunner::Signature(_) => SIGNATURE_BUILTIN_NAME,
            BuiltinRunner::Poseidon(_) => POSEIDON_BUILTIN_NAME,
            BuiltinRunner::SegmentArena(_) => SEGMENT_ARENA_BUILTIN_NAME,
        }
    }

    pub fn run_security_checks(&self, vm: &VirtualMachine) -> Result<(), VirtualMachineError> {
        if let BuiltinRunner::Output(_) | BuiltinRunner::SegmentArena(_) = self {
            return Ok(());
        }
        let cells_per_instance = self.cells_per_instance() as usize;
        let n_input_cells = self.n_input_cells() as usize;
        let builtin_segment_index = self.base();
        // If the builtin's segment is empty, there are no security checks to run
        let builtin_segment = match vm.segments.memory.data.get(builtin_segment_index) {
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
                        Relocatable::from((builtin_segment_index as isize, offset)),
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
            BuiltinRunner::Output(_) | BuiltinRunner::SegmentArena(_) => {
                let used = self.get_used_cells(&vm.segments)?;
                Ok((used, used))
            }
            _ => {
                let used = self.get_used_cells(&vm.segments)?;
                let size = self.get_allocated_memory_units(vm)?;
                if used > size {
                    return Err(InsufficientAllocatedCellsError::BuiltinCells(
                        self.name(),
                        used,
                        size,
                    )
                    .into());
                }
                Ok((used, size))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn set_stop_ptr(&mut self, stop_ptr: usize) {
        match self {
            BuiltinRunner::Bitwise(ref mut bitwise) => bitwise.stop_ptr = Some(stop_ptr),
            BuiltinRunner::EcOp(ref mut ec) => ec.stop_ptr = Some(stop_ptr),
            BuiltinRunner::Hash(ref mut hash) => hash.stop_ptr = Some(stop_ptr),
            BuiltinRunner::Output(ref mut output) => output.stop_ptr = Some(stop_ptr),
            BuiltinRunner::RangeCheck(ref mut range_check) => range_check.stop_ptr = Some(stop_ptr),
            BuiltinRunner::Keccak(ref mut keccak) => keccak.stop_ptr = Some(stop_ptr),
            BuiltinRunner::Signature(ref mut signature) => signature.stop_ptr = Some(stop_ptr),
            BuiltinRunner::Poseidon(ref mut poseidon) => poseidon.stop_ptr = Some(stop_ptr),
            BuiltinRunner::SegmentArena(ref mut segment_arena) => {
                segment_arena.stop_ptr = Some(stop_ptr)
            }
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

impl From<PoseidonBuiltinRunner> for BuiltinRunner {
    fn from(runner: PoseidonBuiltinRunner) -> Self {
        BuiltinRunner::Poseidon(runner)
    }
}

impl From<SegmentArenaBuiltinRunner> for BuiltinRunner {
    fn from(runner: SegmentArenaBuiltinRunner) -> Self {
        BuiltinRunner::SegmentArena(runner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::relocatable;
    use crate::serde::deserialize_program::BuiltinName;
    use crate::types::instance_definitions::ecdsa_instance_def::EcdsaInstanceDef;
    use crate::types::instance_definitions::keccak_instance_def::KeccakInstanceDef;
    use crate::types::program::Program;
    use crate::vm::errors::memory_errors::InsufficientAllocatedCellsError;
    use crate::vm::runners::cairo_runner::CairoRunner;
    use crate::{
        types::instance_definitions::{
            bitwise_instance_def::BitwiseInstanceDef, ec_op_instance_def::EcOpInstanceDef,
        },
        utils::test_utils::*,
        vm::vm_core::VirtualMachine,
    };
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_empty() {
        let builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses() {
        let builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Ok(vec![
                (builtin.base() as isize, 0).into(),
                (builtin.base() as isize, 1).into(),
                (builtin.base() as isize, 2).into(),
                (builtin.base() as isize, 3).into(),
            ]),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_n_input_cells_bitwise() {
        let bitwise = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(Some(10)), true);
        let builtin: BuiltinRunner = bitwise.clone().into();
        assert_eq!(bitwise.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_n_input_cells_hash() {
        let hash = HashBuiltinRunner::new(Some(10), true);
        let builtin: BuiltinRunner = hash.clone().into();
        assert_eq!(hash.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_n_input_cells_range_check() {
        let range_check = RangeCheckBuiltinRunner::new(Some(10), 10, true);
        let builtin: BuiltinRunner = range_check.clone().into();
        assert_eq!(range_check.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_n_input_cells_ec_op() {
        let ec_op = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);
        let builtin: BuiltinRunner = ec_op.clone().into();
        assert_eq!(ec_op.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_n_input_cells_ecdsa() {
        let signature = SignatureBuiltinRunner::new(&EcdsaInstanceDef::new(Some(10)), true);
        let builtin: BuiltinRunner = signature.clone().into();
        assert_eq!(signature.n_input_cells, builtin.n_input_cells())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_n_input_cells_output() {
        let output = OutputBuiltinRunner::new(true);
        let builtin: BuiltinRunner = output.into();
        assert_eq!(0, builtin.n_input_cells())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_instance_bitwise() {
        let bitwise = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(Some(10)), true);
        let builtin: BuiltinRunner = bitwise.clone().into();
        assert_eq!(bitwise.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_instance_hash() {
        let hash = HashBuiltinRunner::new(Some(10), true);
        let builtin: BuiltinRunner = hash.clone().into();
        assert_eq!(hash.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_instance_range_check() {
        let range_check = RangeCheckBuiltinRunner::new(Some(10), 10, true);
        let builtin: BuiltinRunner = range_check.clone().into();
        assert_eq!(range_check.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_instance_ec_op() {
        let ec_op = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);
        let builtin: BuiltinRunner = ec_op.clone().into();
        assert_eq!(ec_op.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_instance_ecdsa() {
        let signature = SignatureBuiltinRunner::new(&EcdsaInstanceDef::new(Some(10)), true);
        let builtin: BuiltinRunner = signature.clone().into();
        assert_eq!(signature.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_instance_output() {
        let output = OutputBuiltinRunner::new(true);
        let builtin: BuiltinRunner = output.into();
        assert_eq!(0, builtin.cells_per_instance())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_instance_keccak() {
        let keccak = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let builtin: BuiltinRunner = keccak.clone().into();
        assert_eq!(keccak.cells_per_instance, builtin.cells_per_instance())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_name_bitwise() {
        let bitwise = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(Some(10)), true);
        let builtin: BuiltinRunner = bitwise.into();
        assert_eq!(BITWISE_BUILTIN_NAME, builtin.name())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_name_hash() {
        let hash = HashBuiltinRunner::new(Some(10), true);
        let builtin: BuiltinRunner = hash.into();
        assert_eq!(HASH_BUILTIN_NAME, builtin.name())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_name_range_check() {
        let range_check = RangeCheckBuiltinRunner::new(Some(10), 10, true);
        let builtin: BuiltinRunner = range_check.into();
        assert_eq!(RANGE_CHECK_BUILTIN_NAME, builtin.name())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_name_ec_op() {
        let ec_op = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);
        let builtin: BuiltinRunner = ec_op.into();
        assert_eq!(EC_OP_BUILTIN_NAME, builtin.name())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_name_ecdsa() {
        let signature = SignatureBuiltinRunner::new(&EcdsaInstanceDef::new(Some(10)), true);
        let builtin: BuiltinRunner = signature.into();
        assert_eq!(SIGNATURE_BUILTIN_NAME, builtin.name())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_name_output() {
        let output = OutputBuiltinRunner::new(true);
        let builtin: BuiltinRunner = output.into();
        assert_eq!(OUTPUT_BUILTIN_NAME, builtin.name())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_bitwise_with_items() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::new(Some(10)),
            true,
        ));

        let mut vm = vm!();

        let program = program!(
            builtins = vec![BuiltinName::bitwise],
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_ec_op_with_items() {
        let builtin = BuiltinRunner::EcOp(EcOpBuiltinRunner::new(
            &EcOpInstanceDef::new(Some(10)),
            true,
        ));

        let mut vm = vm!();

        let program = program!(
            builtins = vec![BuiltinName::ec_op],
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_hash_with_items() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(Some(10), true));

        let mut vm = vm!();

        let program = program!(
            builtins = vec![BuiltinName::pedersen],
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_range_check_with_items() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(10), 12, true));

        let mut vm = vm!();

        let program = program!(
            builtins = vec![BuiltinName::range_check],
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_keccak_with_items() {
        let builtin = BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
            &KeccakInstanceDef::new(Some(10), vec![200; 8]),
            true,
        ));

        let mut vm = vm!();
        vm.current_step = 160;
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(256));
    }

    #[test]
    fn get_allocated_memory_units_keccak_min_steps_not_reached() {
        let builtin = BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
            &KeccakInstanceDef::new(Some(10), vec![200; 8]),
            true,
        ));

        let mut vm = vm!();
        vm.current_step = 10;
        assert_eq!(
            builtin.get_allocated_memory_units(&vm),
            Err(MemoryError::InsufficientAllocatedCells(
                InsufficientAllocatedCellsError::MinStepNotReached(160, KECCAK_BUILTIN_NAME)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_output() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let vm = vm!();

        // In this case, the function always return Ok(0)
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_range_check() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(8), 8, true));
        let mut vm = vm!();
        vm.current_step = 8;
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_hash() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(Some(1), true));
        let mut vm = vm!();
        vm.current_step = 1;
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(3));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_bitwise() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();
        vm.current_step = 256;
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(5));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_ec_op() {
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
        let mut vm = vm!();
        vm.current_step = 256;
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(7));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units_keccak() {
        let builtin = BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
            &KeccakInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();
        vm.current_step = 32768;
        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(256));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_usage_range_check() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(8), 8, true));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), Some((0, 4)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_usage_output() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_usage_hash() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(Some(256), true));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_usage_ec_op() {
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_usage_bitwise() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_bitwise() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 1255);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_keccak_zero_case() {
        let builtin = BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
            &KeccakInstanceDef::default(),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_keccak_non_zero_case() {
        let builtin = BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
            &KeccakInstanceDef::default(),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(0, 8), 32768);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_ec_op() {
        let builtin = BuiltinRunner::EcOp(EcOpBuiltinRunner::new(
            &EcOpInstanceDef::new(Some(10)),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_hash() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(Some(1), true));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_range_check() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(8), 8, true));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_output() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        assert_eq!(builtin.get_used_diluted_check_units(270, 7), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_segment_addresses_test() {
        let bitwise_builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        assert_eq!(bitwise_builtin.get_memory_segment_addresses(), (0, None),);
        let ec_op_builtin: BuiltinRunner =
            EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true).into();
        assert_eq!(ec_op_builtin.get_memory_segment_addresses(), (0, None),);
        let hash_builtin: BuiltinRunner = HashBuiltinRunner::new(Some(8), true).into();
        assert_eq!(hash_builtin.get_memory_segment_addresses(), (0, None),);
        let output_builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();
        assert_eq!(output_builtin.get_memory_segment_addresses(), (0, None),);
        let range_check_builtin: BuiltinRunner =
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(8), 8, true));
        assert_eq!(
            range_check_builtin.get_memory_segment_addresses(),
            (0, None),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_for_output() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let vm = vm!();

        assert_matches!(builtin.run_security_checks(&vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_empty_memory() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let vm = vm!();
        // Unused builtin shouldn't fail security checks
        assert_matches!(builtin.run_security_checks(&vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_empty_offsets() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.segments.memory.data = vec![vec![]];

        assert_matches!(builtin.run_security_checks(&vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_bitwise_missing_memory_cells_with_offsets() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();
        vm.segments.memory = memory![
            ((0, 1), (0, 1)),
            ((0, 2), (0, 2)),
            ((0, 3), (0, 3)),
            ((0, 4), (0, 4))
        ];

        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCellsWithOffsets(BITWISE_BUILTIN_NAME, x)
            )) if x == vec![0]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_bitwise_missing_memory_cells() {
        let mut bitwise_builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);

        bitwise_builtin.cells_per_instance = 2;
        bitwise_builtin.n_input_cells = 5;

        let builtin: BuiltinRunner = bitwise_builtin.into();

        let mut vm = vm!();

        vm.segments.memory = memory![
            ((0, 0), (0, 1)),
            ((0, 1), (0, 2)),
            ((0, 2), (0, 3)),
            ((0, 3), (0, 4)),
            ((0, 4), (0, 5))
        ];

        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCells(BITWISE_BUILTIN_NAME)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_hash_missing_memory_cells_with_offsets() {
        let builtin: BuiltinRunner = HashBuiltinRunner::new(Some(8), true).into();
        let mut vm = vm!();

        vm.segments.memory = memory![
            ((0, 1), (0, 1)),
            ((0, 2), (0, 2)),
            ((0, 3), (0, 3)),
            ((0, 4), (0, 4)),
            ((0, 5), (0, 5))
        ];
        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCellsWithOffsets(HASH_BUILTIN_NAME, x)
            )) if x == vec![0]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_hash_missing_memory_cells() {
        let hash_builtin = HashBuiltinRunner::new(Some(8), true);

        let builtin: BuiltinRunner = hash_builtin.into();

        let mut vm = vm!();

        vm.segments.memory = memory![((0, 0), (0, 0))];

        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCells(HASH_BUILTIN_NAME)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_range_check_missing_memory_cells_with_offsets() {
        let range_check_builtin = RangeCheckBuiltinRunner::new(Some(8), 8, true);
        let builtin: BuiltinRunner = range_check_builtin.into();
        let mut vm = vm!();

        vm.segments.memory = memory![
            ((0, 1), 100),
            ((0, 2), 2),
            ((0, 3), 3),
            ((0, 5), 5),
            ((0, 6), 17),
            ((0, 7), 22)
        ];

        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCells(RANGE_CHECK_BUILTIN_NAME)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_range_check_missing_memory_cells() {
        let builtin: BuiltinRunner =
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(8), 8, true));
        let mut vm = vm!();

        vm.segments.memory = memory![((0, 1), 1)];

        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCells(RANGE_CHECK_BUILTIN_NAME)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_range_check_empty() {
        let range_check_builtin = RangeCheckBuiltinRunner::new(Some(8), 8, true);

        let builtin: BuiltinRunner = range_check_builtin.into();

        let mut vm = vm!();

        vm.segments.memory.data = vec![vec![None, None, None]];

        assert_matches!(builtin.run_security_checks(&vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_checks_validate_auto_deductions() {
        let builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();

        let mut vm = vm!();
        vm.segments
            .memory
            .validated_addresses
            .extend(&[relocatable!(0, 2)]);

        vm.segments.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((0, 2), (0, 2)),
            ((0, 3), (0, 3)),
            ((0, 4), (0, 4))
        ];

        assert_matches!(builtin.run_security_checks(&vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_ec_op_check_memory_empty() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let builtin: BuiltinRunner = ec_op_builtin.into();

        let mut vm = vm!();
        // The values stored in memory are not relevant for this test
        vm.segments.memory.data = vec![vec![]];

        assert_matches!(builtin.run_security_checks(&vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_ec_op_check_memory_1_element() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let builtin: BuiltinRunner = ec_op_builtin.into();

        let mut vm = vm!();
        // The values stored in memory are not relevant for this test
        vm.segments.memory = memory![((0, 0), 0)];
        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCells(EC_OP_BUILTIN_NAME)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_ec_op_check_memory_3_elements() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let builtin: BuiltinRunner = ec_op_builtin.into();

        let mut vm = vm!();
        // The values stored in memory are not relevant for this test
        vm.segments.memory = memory![((0, 0), 0), ((0, 1), 0), ((0, 2), 0)];

        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCells(EC_OP_BUILTIN_NAME)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_ec_op_missing_memory_cells_with_offsets() {
        let builtin: BuiltinRunner =
            EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true).into();
        let mut vm = vm!();
        vm.segments.memory = memory![
            ((0, 1), (0, 1)),
            ((0, 2), (0, 2)),
            ((0, 3), (0, 3)),
            ((0, 4), (0, 4)),
            ((0, 5), (0, 5)),
            ((0, 6), (0, 6))
        ];

        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCellsWithOffsets(EC_OP_BUILTIN_NAME, x)
            )) if x == vec![0]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_security_ec_op_check_memory_gap() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let builtin: BuiltinRunner = ec_op_builtin.into();

        let mut vm = vm!();
        // The values stored in memory are not relevant for this test
        vm.segments.memory = memory![
            ((0, 0), 0),
            ((0, 1), 1),
            ((0, 2), 2),
            ((0, 3), 3),
            ((0, 4), 4),
            ((0, 5), 5),
            ((0, 6), 6),
            ((0, 8), 8),
            ((0, 9), 9),
            ((0, 10), 10),
            ((0, 11), 11)
        ];

        assert_matches!(
            builtin.run_security_checks(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::MissingMemoryCellsWithOffsets(EC_OP_BUILTIN_NAME, x)
            )) if x == vec![7]
        );
    }

    /// Test that get_used_perm_range_check_units() returns zero when the
    /// builtin is a BitwiseBuiltinRunner.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_perm_range_check_units_hash() {
        let builtin_runner: BuiltinRunner = HashBuiltinRunner::new(Some(8), true).into();
        let mut vm = vm!();

        vm.current_step = 8;
        vm.segments.segment_used_sizes = Some(vec![5]);
        assert_eq!(builtin_runner.get_used_perm_range_check_units(&vm), Ok(0));
    }

    /// Test that get_used_perm_range_check_units() returns zero when the
    /// builtin is an OutputBuiltinRunner.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_perm_range_check_units_range_check() {
        let builtin_runner: BuiltinRunner = RangeCheckBuiltinRunner::new(Some(8), 8, true).into();
        let mut vm = vm!();

        vm.current_step = 8;
        vm.segments.segment_used_sizes = Some(vec![1]);
        assert_eq!(builtin_runner.get_used_perm_range_check_units(&vm), Ok(8));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_ratio_tests() {
        let bitwise_builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        assert_eq!(bitwise_builtin.ratio(), (Some(256)),);
        let ec_op_builtin: BuiltinRunner =
            EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true).into();
        assert_eq!(ec_op_builtin.ratio(), (Some(256)),);
        let hash_builtin: BuiltinRunner = HashBuiltinRunner::new(Some(8), true).into();
        assert_eq!(hash_builtin.ratio(), (Some(8)),);
        let output_builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();
        assert_eq!(output_builtin.ratio(), None,);
        let range_check_builtin: BuiltinRunner =
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(8), 8, true));
        assert_eq!(range_check_builtin.ratio(), (Some(8)),);
        let keccak_builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true).into();
        assert_eq!(keccak_builtin.ratio(), (Some(2048)),);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn bitwise_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let bitwise_builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into();
        assert_eq!(bitwise_builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn ec_op_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let ec_op_builtin: BuiltinRunner =
            EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true).into();
        assert_eq!(ec_op_builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn hash_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let hash_builtin: BuiltinRunner = HashBuiltinRunner::new(Some(8), true).into();
        assert_eq!(hash_builtin.get_used_instances(&vm.segments), Ok(2));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn output_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let output_builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();
        assert_eq!(output_builtin.get_used_instances(&vm.segments), Ok(4));
    }
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn range_check_get_used_instances_test() {
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        let range_check_builtin: BuiltinRunner =
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(8), 8, true));
        assert_eq!(range_check_builtin.get_used_instances(&vm.segments), Ok(4));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn runners_final_stack() {
        let mut builtins = vec![
            BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
                &BitwiseInstanceDef::default(),
                false,
            )),
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), false)),
            BuiltinRunner::Hash(HashBuiltinRunner::new(Some(1), false)),
            BuiltinRunner::Output(OutputBuiltinRunner::new(false)),
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(8), 8, false)),
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
            assert_eq!(br.final_stack(&vm.segments, vm.get_ap()), Ok(vm.get_ap()));
        }
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn runners_set_stop_ptr() {
        let builtins = vec![
            BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
                &BitwiseInstanceDef::default(),
                false,
            )),
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), false)),
            BuiltinRunner::Hash(HashBuiltinRunner::new(Some(1), false)),
            BuiltinRunner::Output(OutputBuiltinRunner::new(false)),
            BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(Some(8), 8, false)),
            BuiltinRunner::Keccak(KeccakBuiltinRunner::new(
                &KeccakInstanceDef::default(),
                false,
            )),
            BuiltinRunner::Signature(SignatureBuiltinRunner::new(
                &EcdsaInstanceDef::default(),
                false,
            )),
            BuiltinRunner::Poseidon(PoseidonBuiltinRunner::new(Some(32), false)),
            BuiltinRunner::SegmentArena(SegmentArenaBuiltinRunner::new(false)),
        ];

        let ptr = 3;

        for mut br in builtins {
            br.set_stop_ptr(ptr);
            let (_, stop_ptr) = br.get_memory_segment_addresses();
            assert_eq!(stop_ptr, Some(ptr));
        }
    }
}
