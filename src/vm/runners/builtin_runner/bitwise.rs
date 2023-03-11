use crate::stdlib::vec::Vec;
use crate::{
    math_utils::safe_div_usize,
    types::{
        instance_definitions::bitwise_instance_def::{
            BitwiseInstanceDef, CELLS_PER_BITWISE, INPUT_CELLS_PER_BITWISE,
        },
        relocatable::{MaybeRelocatable, Relocatable},
    },
    vm::{
        errors::{
            memory_errors::{InsufficientAllocatedCellsError, MemoryError},
            runner_errors::RunnerError,
        },
        vm_core::VirtualMachine,
        vm_memory::{memory::Memory, memory_segments::MemorySegmentManager},
    },
};
use num_integer::div_ceil;

use super::BITWISE_BUILTIN_NAME;

#[derive(Debug, Clone)]
pub struct BitwiseBuiltinRunner {
    ratio: u32,
    pub base: usize,
    pub(crate) cells_per_instance: u32,
    pub(crate) n_input_cells: u32,
    bitwise_builtin: BitwiseInstanceDef,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) included: bool,
    instances_per_component: u32,
}

impl BitwiseBuiltinRunner {
    pub(crate) fn new(instance_def: &BitwiseInstanceDef, included: bool) -> Self {
        BitwiseBuiltinRunner {
            base: 0,
            ratio: instance_def.ratio,
            cells_per_instance: CELLS_PER_BITWISE,
            n_input_cells: INPUT_CELLS_PER_BITWISE,
            bitwise_builtin: instance_def.clone(),
            stop_ptr: None,
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
        let index = address.offset % self.cells_per_instance as usize;
        if index <= 1 {
            return Ok(None);
        }
        let x_addr = Relocatable::from((address.segment_index, address.offset - index));
        let y_addr = (x_addr + 1_usize)?;

        let num_x = memory.get(&x_addr);
        let num_y = memory.get(&y_addr);
        if let (Some(MaybeRelocatable::Int(ref num_x)), Some(MaybeRelocatable::Int(ref num_y))) = (
            num_x.as_ref().map(|x| x.as_ref()),
            num_y.as_ref().map(|x| x.as_ref()),
        ) {
            if num_x.bits() > self.bitwise_builtin.total_n_bits as u64 {
                return Err(RunnerError::IntegerBiggerThanPowerOfTwo(
                    x_addr.into(),
                    self.bitwise_builtin.total_n_bits,
                    num_x.clone(),
                ));
            };
            if num_y.bits() > self.bitwise_builtin.total_n_bits as u64 {
                return Err(RunnerError::IntegerBiggerThanPowerOfTwo(
                    y_addr.into(),
                    self.bitwise_builtin.total_n_bits,
                    num_y.clone(),
                ));
            };
            let res = match index {
                2 => Some(MaybeRelocatable::from(num_x & num_y)),
                3 => Some(MaybeRelocatable::from(num_x ^ num_y)),
                4 => Some(MaybeRelocatable::from(num_x | num_y)),
                _ => None,
            };
            return Ok(res);
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
            .get_segment_used_size(self.base)
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
                InsufficientAllocatedCellsError::MinStepNotReached(min_step, BITWISE_BUILTIN_NAME)
                    .into(),
            )
        } else {
            let used = self.get_used_cells(&vm.segments)?;
            let size = self.cells_per_instance as usize
                * safe_div_usize(vm.current_step, ratio).map_err(|_| {
                    InsufficientAllocatedCellsError::CurrentStepNotDivisibleByBuiltinRatio(
                        BITWISE_BUILTIN_NAME,
                        vm.current_step,
                        ratio,
                    )
                })?;
            if used > size {
                return Err(InsufficientAllocatedCellsError::BuiltinCells(
                    BITWISE_BUILTIN_NAME,
                    used,
                    size,
                )
                .into());
            }
            Ok((used, size))
        }
    }

    pub fn get_used_diluted_check_units(&self, diluted_spacing: u32, diluted_n_bits: u32) -> usize {
        let total_n_bits = self.bitwise_builtin.total_n_bits;
        let mut partition = Vec::with_capacity(total_n_bits as usize);
        for i in (0..total_n_bits).step_by((diluted_spacing * diluted_n_bits) as usize) {
            for j in 0..diluted_spacing {
                if i + j < total_n_bits {
                    partition.push(i + j)
                };
            }
        }
        let partition_lengh = partition.len();
        let num_trimmed = partition
            .into_iter()
            .filter(|elem| elem + diluted_spacing * (diluted_n_bits - 1) + 1 > total_n_bits)
            .count();
        4 * partition_lengh + num_trimmed
    }

    pub fn final_stack(
        &mut self,
        segments: &MemorySegmentManager,
        pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        if self.included {
            let stop_pointer_addr =
                (pointer - 1).map_err(|_| RunnerError::NoStopPointer(BITWISE_BUILTIN_NAME))?;
            let stop_pointer = segments
                .memory
                .get_relocatable(stop_pointer_addr)
                .map_err(|_| RunnerError::NoStopPointer(BITWISE_BUILTIN_NAME))?;
            if self.base as isize != stop_pointer.segment_index {
                return Err(RunnerError::InvalidStopPointerIndex(
                    BITWISE_BUILTIN_NAME,
                    stop_pointer,
                    self.base,
                ));
            }
            let stop_ptr = stop_pointer.offset;
            let num_instances = self.get_used_instances(segments)?;
            let used = num_instances * self.cells_per_instance as usize;
            if stop_ptr != used {
                return Err(RunnerError::InvalidStopPointer(
                    BITWISE_BUILTIN_NAME,
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

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        let used_cells = self.get_used_cells(segments)?;
        Ok(div_ceil(used_cells, self.cells_per_instance as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relocatable;
    use crate::serde::deserialize_program::BuiltinName;
    use crate::stdlib::collections::HashMap;
    use crate::vm::errors::memory_errors::MemoryError;
    use crate::vm::runners::builtin_runner::BuiltinRunner;
    use crate::vm::vm_core::VirtualMachine;
    use crate::vm::vm_memory::memory::Memory;
    use crate::{
        hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
        types::program::Program, utils::test_utils::*, vm::runners::cairo_runner::CairoRunner,
    };
    use felt::Felt;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack() {
        let mut builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), true);

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
        let mut builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![995]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(
                BITWISE_BUILTIN_NAME,
                relocatable!(0, 995),
                relocatable!(0, 0)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_when_notincluded() {
        let mut builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), false);

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
        let mut builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), true);

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
            Err(RunnerError::NoStopPointer(BITWISE_BUILTIN_NAME))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner =
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), true).into();

        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);

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

        assert_eq!(builtin.get_used_cells_and_allocated_size(&vm), Ok((0, 5)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units() {
        let builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(10), true);

        let mut vm = vm!();

        let program = program!(
            builtins = vec![BuiltinName::pedersen, BuiltinName::bitwise],
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
    fn deduce_memory_cell_bitwise_for_preset_memory_valid_and() {
        let memory = memory![((0, 5), 10), ((0, 6), 12), ((0, 7), 0)];
        let builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 7)), &memory);
        assert_eq!(result, Ok(Some(MaybeRelocatable::from(Felt::new(8)))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_bitwise_for_preset_memory_valid_xor() {
        let memory = memory![((0, 5), 10), ((0, 6), 12), ((0, 8), 0)];
        let builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 8)), &memory);
        assert_eq!(result, Ok(Some(MaybeRelocatable::from(Felt::new(6)))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_bitwise_for_preset_memory_valid_or() {
        let memory = memory![((0, 5), 10), ((0, 6), 12), ((0, 9), 0)];
        let builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 9)), &memory);
        assert_eq!(result, Ok(Some(MaybeRelocatable::from(Felt::new(14)))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_bitwise_for_preset_memory_incorrect_offset() {
        let memory = memory![((0, 3), 10), ((0, 4), 12), ((0, 5), 0)];
        let builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_bitwise_for_preset_memory_no_values_to_operate() {
        let memory = memory![((0, 5), 12), ((0, 7), 0)];
        let builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_segment_addresses() {
        let builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);

        assert_eq!(builtin.get_memory_segment_addresses(), (0, None),);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_empty() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
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
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_empty() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_a() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(12, 2), 535);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_b() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(30, 56), 150);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_c() {
        let builtin = BuiltinRunner::Bitwise(BitwiseBuiltinRunner::new(
            &BitwiseInstanceDef::default(),
            true,
        ));
        assert_eq!(builtin.get_used_diluted_check_units(50, 25), 250);
    }
}
