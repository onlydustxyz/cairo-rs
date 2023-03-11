use crate::stdlib::{cell::RefCell, prelude::*};

use crate::math_utils::safe_div_usize;
use crate::types::instance_definitions::pedersen_instance_def::{
    CELLS_PER_HASH, INPUT_CELLS_PER_HASH,
};
use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::{InsufficientAllocatedCellsError, MemoryError};
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::vm_core::VirtualMachine;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;
use felt::Felt;
use num_integer::{div_ceil, Integer};
use starknet_crypto::{pedersen_hash, FieldElement};

use super::EC_OP_BUILTIN_NAME;

#[derive(Debug, Clone)]
pub struct HashBuiltinRunner {
    pub base: usize,
    ratio: u32,
    pub(crate) cells_per_instance: u32,
    pub(crate) n_input_cells: u32,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) included: bool,
    instances_per_component: u32,
    // This act as a cache to optimize calls to deduce_memory_cell
    // Therefore need interior mutability
    pub(self) verified_addresses: RefCell<Vec<Relocatable>>,
}

impl HashBuiltinRunner {
    pub fn new(ratio: u32, included: bool) -> Self {
        HashBuiltinRunner {
            base: 0,
            ratio,
            cells_per_instance: CELLS_PER_HASH,
            n_input_cells: INPUT_CELLS_PER_HASH,
            stop_ptr: None,
            verified_addresses: RefCell::new(Vec::new()),
            included,
            instances_per_component: 1,
        }
    }

    pub fn initialize_segments(&mut self, segments: &mut MemorySegmentManager) {
        self.base = segments.add().segment_index as usize // segments.add() always returns a positive index
    }

    pub fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        if self.included {
            vec![MaybeRelocatable::from((self.base as isize, 0))]
        } else {
            vec![]
        }
    }

    pub fn base(&self) -> usize {
        self.base
    }

    pub fn ratio(&self) -> u32 {
        self.ratio
    }

    pub fn add_validation_rule(&self, _memory: &mut Memory) {}

    pub fn deduce_memory_cell(
        &self,
        address: Relocatable,
        memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        if address
            .offset
            .mod_floor(&(self.cells_per_instance as usize))
            != 2
            || self.verified_addresses.borrow().contains(&address)
        {
            return Ok(None);
        };

        let num_a = memory.get(&MaybeRelocatable::RelocatableValue(Relocatable {
            segment_index: address.segment_index,
            offset: address.offset - 1,
        }));
        let num_b = memory.get(&MaybeRelocatable::RelocatableValue(Relocatable {
            segment_index: address.segment_index,
            offset: address.offset - 2,
        }));
        if let (Some(MaybeRelocatable::Int(num_a)), Some(MaybeRelocatable::Int(num_b))) = (
            num_a.as_ref().map(|x| x.as_ref()),
            num_b.as_ref().map(|x| x.as_ref()),
        ) {
            self.verified_addresses.borrow_mut().push(address);

            //Convert MaybeRelocatable to FieldElement
            let a_string = num_a.to_str_radix(10);
            let b_string = num_b.to_str_radix(10);
            let (y, x) = match (
                FieldElement::from_dec_str(&a_string),
                FieldElement::from_dec_str(&b_string),
            ) {
                (Ok(field_element_a), Ok(field_element_b)) => (field_element_a, field_element_b),
                _ => return Err(RunnerError::FailedStringConversion),
            };
            //Compute pedersen Hash
            let fe_result = pedersen_hash(&x, &y);
            //Convert result from FieldElement to MaybeRelocatable
            let r_byte_slice = fe_result.to_bytes_be();
            let result = Felt::from_bytes_be(&r_byte_slice);
            return Ok(Some(MaybeRelocatable::from(result)));
        }
        Ok(None)
    }

    pub fn get_allocated_memory_units(&self, vm: &VirtualMachine) -> Result<usize, MemoryError> {
        let value = safe_div_usize(vm.current_step, self.ratio as usize)
            .map_err(|_| MemoryError::ErrorCalculatingMemoryUnits)?;
        Ok(self.cells_per_instance as usize * value)
    }

    pub fn get_memory_segment_addresses(&self) -> (usize, Option<usize>) {
        (self.base, self.stop_ptr)
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        segments
            .get_segment_used_size(self.base())
            .ok_or(MemoryError::MissingSegmentUsedSizes)
    }

    pub fn get_used_cells_and_allocated_size(
        &self,
        vm: &VirtualMachine,
    ) -> Result<(usize, usize), MemoryError> {
        let ratio = self.ratio as usize;
        let min_step = ratio * self.instances_per_component as usize;
        if vm.current_step < min_step {
            Err(
                InsufficientAllocatedCellsError::MinStepNotReached(min_step, EC_OP_BUILTIN_NAME)
                    .into(),
            )
        } else {
            let used = self.get_used_cells(&vm.segments)?;
            let size = self.cells_per_instance as usize
                * safe_div_usize(vm.current_step, ratio).map_err(|_| {
                    InsufficientAllocatedCellsError::CurrentStepNotDivisibleByBuiltinRatio(
                        EC_OP_BUILTIN_NAME,
                        vm.current_step,
                        ratio,
                    )
                })?;
            if used > size {
                return Err(InsufficientAllocatedCellsError::BuiltinCells(
                    EC_OP_BUILTIN_NAME,
                    used,
                    size,
                )
                .into());
            }
            Ok((used, size))
        }
    }

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        let used_cells = self.get_used_cells(segments)?;
        Ok(div_ceil(used_cells, self.cells_per_instance as usize))
    }

    pub fn final_stack(
        &mut self,
        segments: &MemorySegmentManager,
        pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        if self.included {
            let stop_pointer_addr =
                (pointer - 1).map_err(|_| RunnerError::NoStopPointer(EC_OP_BUILTIN_NAME))?;
            let stop_pointer = segments
                .memory
                .get_relocatable(stop_pointer_addr)
                .map_err(|_| RunnerError::NoStopPointer(EC_OP_BUILTIN_NAME))?;
            if self.base as isize != stop_pointer.segment_index {
                return Err(RunnerError::InvalidStopPointerIndex(
                    EC_OP_BUILTIN_NAME,
                    stop_pointer,
                    self.base,
                ));
            }
            let stop_ptr = stop_pointer.offset;
            let num_instances = self.get_used_instances(segments)?;
            let used = num_instances * self.cells_per_instance as usize;
            if stop_ptr != used {
                return Err(RunnerError::InvalidStopPointer(
                    EC_OP_BUILTIN_NAME,
                    Relocatable::from((self.base as isize, used)),
                    Relocatable::from((self.base as isize, stop_ptr)),
                ));
            }
            self.stop_ptr = Some(stop_ptr);
            Ok(stop_pointer_addr)
        } else {
            let stop_ptr = self.base;
            self.stop_ptr = Some(stop_ptr);
            Ok(pointer)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::relocatable;
    use crate::serde::deserialize_program::BuiltinName;
    use crate::stdlib::collections::HashMap;
    use crate::types::program::Program;
    use crate::utils::test_utils::*;
    use crate::vm::runners::cairo_runner::CairoRunner;
    use crate::vm::vm_memory::memory::Memory;
    use crate::vm::{
        errors::memory_errors::MemoryError, runners::builtin_runner::BuiltinRunner,
        vm_core::VirtualMachine,
    };
    use felt::felt_str;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin = HashBuiltinRunner::new(10, true);

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack() {
        let mut builtin = HashBuiltinRunner::new(10, true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer).unwrap(),
            Relocatable::from((2, 1))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_stop_pointer() {
        let mut builtin = HashBuiltinRunner::new(10, true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![999]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(
                EC_OP_BUILTIN_NAME,
                relocatable!(0, 999),
                relocatable!(0, 0)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_when_not_included() {
        let mut builtin = HashBuiltinRunner::new(10, false);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer).unwrap(),
            Relocatable::from((2, 2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_non_relocatable() {
        let mut builtin = HashBuiltinRunner::new(10, true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::NoStopPointer(EC_OP_BUILTIN_NAME))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner = HashBuiltinRunner::new(10, true).into();

        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);

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

        assert_eq!(builtin.get_used_cells_and_allocated_size(&vm), Ok((0, 3)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units() {
        let builtin = HashBuiltinRunner::new(10, true);

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

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(3));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_pedersen_for_preset_memory_valid() {
        let memory = memory![((0, 3), 32), ((0, 4), 72), ((0, 5), 0)];
        let builtin = HashBuiltinRunner::new(8, true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 5)), &memory);
        assert_eq!(
            result,
            Ok(Some(MaybeRelocatable::from(felt_str!(
                "3270867057177188607814717243084834301278723532952411121381966378910183338911"
            ))))
        );
        assert_eq!(
            builtin.verified_addresses.into_inner(),
            vec![Relocatable::from((0, 5))]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_pedersen_for_preset_memory_incorrect_offset() {
        let memory = memory![((0, 4), 32), ((0, 5), 72), ((0, 6), 0)];
        let builtin = HashBuiltinRunner::new(8, true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 6)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_pedersen_for_preset_memory_no_values_to_hash() {
        let memory = memory![((0, 4), 72), ((0, 5), 0)];
        let builtin = HashBuiltinRunner::new(8, true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_pedersen_for_preset_memory_already_computed() {
        let memory = memory![((0, 3), 32), ((0, 4), 72), ((0, 5), 0)];
        let mut builtin = HashBuiltinRunner::new(8, true);
        builtin.verified_addresses = RefCell::new(vec![Relocatable::from((0, 5))]);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_segment_addresses() {
        let builtin = HashBuiltinRunner::new(256, true);

        assert_eq!(builtin.get_memory_segment_addresses(), (0, None),);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(256, true));
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_empty() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(256, true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(256, true));
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
    fn get_used_cells_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(256, true));
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_empty() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(256, true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells() {
        let builtin = BuiltinRunner::Hash(HashBuiltinRunner::new(256, true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }
}
