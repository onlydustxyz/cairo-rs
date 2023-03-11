use crate::stdlib::{
    cmp::{max, min},
    ops::Shl,
    prelude::*,
};

use crate::{
    math_utils::safe_div_usize,
    types::{
        instance_definitions::range_check_instance_def::CELLS_PER_RANGE_CHECK,
        relocatable::{MaybeRelocatable, Relocatable},
    },
    vm::{
        errors::{
            memory_errors::{InsufficientAllocatedCellsError, MemoryError},
            runner_errors::RunnerError,
        },
        vm_core::VirtualMachine,
        vm_memory::{
            memory::{Memory, ValidationRule},
            memory_segments::MemorySegmentManager,
        },
    },
};
use felt::Felt;
use num_integer::Integer;
use num_traits::{One, ToPrimitive, Zero};

use super::RANGE_CHECK_BUILTIN_NAME;

#[derive(Debug, Clone)]
pub struct RangeCheckBuiltinRunner {
    ratio: u32,
    base: usize,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) cells_per_instance: u32,
    pub(crate) n_input_cells: u32,
    inner_rc_bound: usize,
    pub _bound: Option<Felt>,
    pub(crate) included: bool,
    n_parts: u32,
    instances_per_component: u32,
}

impl RangeCheckBuiltinRunner {
    pub fn new(ratio: u32, n_parts: u32, included: bool) -> RangeCheckBuiltinRunner {
        let inner_rc_bound = 1_usize << 16;

        let bound = Felt::one().shl(16 * n_parts);
        let _bound = if n_parts != 0 && bound.is_zero() {
            None
        } else {
            Some(Felt::new(bound))
        };

        RangeCheckBuiltinRunner {
            ratio,
            base: 0,
            stop_ptr: None,
            cells_per_instance: CELLS_PER_RANGE_CHECK,
            n_input_cells: CELLS_PER_RANGE_CHECK,
            inner_rc_bound,
            _bound,
            included,
            n_parts,
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

    pub fn add_validation_rule(&self, memory: &mut Memory) {
        let rule: ValidationRule = ValidationRule(Box::new(
            |memory: &Memory, address: Relocatable| -> Result<Vec<Relocatable>, MemoryError> {
                let num = memory
                    .get_integer(address)
                    .map_err(|_| MemoryError::RangeCheckFoundNonInt(address))?;
                if &Felt::zero() <= num.as_ref() && num.as_ref() < &Felt::one().shl(128_usize) {
                    Ok(vec![address.to_owned()])
                } else {
                    Err(MemoryError::RangeCheckNumOutOfBounds(
                        num.into_owned(),
                        Felt::one().shl(128_usize),
                    ))
                }
            },
        ));
        memory.add_validation_rule(self.base, rule);
    }

    pub fn deduce_memory_cell(
        &self,
        _address: Relocatable,
        _memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
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
            Err(InsufficientAllocatedCellsError::MinStepNotReached(
                min_step,
                RANGE_CHECK_BUILTIN_NAME,
            )
            .into())
        } else {
            let used = self.get_used_cells(&vm.segments)?;
            let size = self.cells_per_instance as usize
                * safe_div_usize(vm.current_step, ratio).map_err(|_| {
                    InsufficientAllocatedCellsError::CurrentStepNotDivisibleByBuiltinRatio(
                        RANGE_CHECK_BUILTIN_NAME,
                        vm.current_step,
                        ratio,
                    )
                })?;
            if used > size {
                return Err(InsufficientAllocatedCellsError::BuiltinCells(
                    RANGE_CHECK_BUILTIN_NAME,
                    used,
                    size,
                )
                .into());
            }
            Ok((used, size))
        }
    }

    pub fn get_range_check_usage(&self, memory: &Memory) -> Option<(usize, usize)> {
        let mut rc_bounds: Option<(usize, usize)> = None;
        let range_check_segment = memory.data.get(self.base)?;
        let inner_rc_bound = Felt::new(self.inner_rc_bound);
        for value in range_check_segment {
            //Split val into n_parts parts.
            for _ in 0..self.n_parts {
                let part_val = value
                    .as_ref()?
                    .get_value()
                    .get_int_ref()?
                    .mod_floor(&inner_rc_bound)
                    .to_usize()?;
                rc_bounds = Some(match rc_bounds {
                    None => (part_val, part_val),
                    Some((rc_min, rc_max)) => {
                        let rc_min = min(rc_min, part_val);
                        let rc_max = max(rc_max, part_val);

                        (rc_min, rc_max)
                    }
                });
            }
        }
        rc_bounds
    }

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        self.get_used_cells(segments)
    }

    pub fn final_stack(
        &mut self,
        segments: &MemorySegmentManager,
        pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        if self.included {
            let stop_pointer_addr =
                (pointer - 1).map_err(|_| RunnerError::NoStopPointer(RANGE_CHECK_BUILTIN_NAME))?;
            let stop_pointer = segments
                .memory
                .get_relocatable(stop_pointer_addr)
                .map_err(|_| RunnerError::NoStopPointer(RANGE_CHECK_BUILTIN_NAME))?;
            if self.base as isize != stop_pointer.segment_index {
                return Err(RunnerError::InvalidStopPointerIndex(
                    RANGE_CHECK_BUILTIN_NAME,
                    stop_pointer,
                    self.base,
                ));
            }
            let stop_ptr = stop_pointer.offset;
            let num_instances = self.get_used_instances(segments)?;
            let used = num_instances * self.cells_per_instance as usize;
            if stop_ptr != used {
                return Err(RunnerError::InvalidStopPointer(
                    RANGE_CHECK_BUILTIN_NAME,
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

    /// Returns the number of range check units used by the builtin.
    pub fn get_used_perm_range_check_units(
        &self,
        vm: &VirtualMachine,
    ) -> Result<usize, MemoryError> {
        let (used_cells, _) = self.get_used_cells_and_allocated_size(vm)?;
        Ok(used_cells * self.n_parts as usize)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relocatable;
    use crate::serde::deserialize_program::BuiltinName;
    use crate::stdlib::collections::HashMap;
    use crate::vm::vm_memory::memory::Memory;
    use crate::{
        hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
        types::program::Program,
        utils::test_utils::*,
        vm::{
            runners::{builtin_runner::BuiltinRunner, cairo_runner::CairoRunner},
            vm_core::VirtualMachine,
        },
    };

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin = RangeCheckBuiltinRunner::new(10, 12, true);

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack() {
        let mut builtin = RangeCheckBuiltinRunner::new(10, 12, true);

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
        let mut builtin = RangeCheckBuiltinRunner::new(10, 12, true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![998]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(
                RANGE_CHECK_BUILTIN_NAME,
                relocatable!(0, 998),
                relocatable!(0, 0)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_when_notincluded() {
        let mut builtin = RangeCheckBuiltinRunner::new(10, 12, false);

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
        let mut builtin = RangeCheckBuiltinRunner::new(10, 12, true);

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
            Err(RunnerError::NoStopPointer(RANGE_CHECK_BUILTIN_NAME))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner = RangeCheckBuiltinRunner::new(10, 12, true).into();

        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);

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

        assert_eq!(builtin.get_used_cells_and_allocated_size(&vm), Ok((0, 1)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units() {
        let builtin = RangeCheckBuiltinRunner::new(10, 12, true);

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
    fn initialize_segments_for_range_check() {
        let mut builtin = RangeCheckBuiltinRunner::new(8, 8, true);
        let mut segments = MemorySegmentManager::new();
        builtin.initialize_segments(&mut segments);
        assert_eq!(builtin.base, 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_initial_stack_for_range_check_with_base() {
        let mut builtin = RangeCheckBuiltinRunner::new(8, 8, true);
        builtin.base = 1;
        let initial_stack = builtin.initial_stack();
        assert_eq!(
            initial_stack[0].clone(),
            MaybeRelocatable::RelocatableValue((builtin.base() as isize, 0).into())
        );
        assert_eq!(initial_stack.len(), 1);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_segment_addresses() {
        let builtin = RangeCheckBuiltinRunner::new(8, 8, true);

        assert_eq!(builtin.get_memory_segment_addresses(), (0, None),);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(256, 8, true));
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_empty() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(256, 8, true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(256, 8, true));
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
    fn test_base() {
        let builtin = RangeCheckBuiltinRunner::new(8, 8, true);
        assert_eq!(builtin.base(), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_ratio() {
        let builtin = RangeCheckBuiltinRunner::new(8, 8, true);
        assert_eq!(builtin.ratio(), 8);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(256, 8, true));
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_empty() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(256, 8, true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells() {
        let builtin = BuiltinRunner::RangeCheck(RangeCheckBuiltinRunner::new(256, 8, true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_usage_succesful_a() {
        let builtin = RangeCheckBuiltinRunner::new(8, 8, true);
        let memory = memory![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        assert_eq!(builtin.get_range_check_usage(&memory), Some((1, 4)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_usage_succesful_b() {
        let builtin = RangeCheckBuiltinRunner::new(8, 8, true);
        let memory = memory![
            ((0, 0), 1465218365),
            ((0, 1), 2134570341),
            ((0, 2), 31349610736_i64),
            ((0, 3), 413468326585859_i64)
        ];
        assert_eq!(builtin.get_range_check_usage(&memory), Some((6384, 62821)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_usage_succesful_c() {
        let builtin = RangeCheckBuiltinRunner::new(8, 8, true);
        let memory = memory![
            ((0, 0), 634834751465218365_i64),
            ((0, 1), 42876922134570341_i64),
            ((0, 2), 23469831349610736_i64),
            ((0, 3), 23468413468326585859_i128),
            ((0, 4), 75346043276073460326_i128),
            ((0, 5), 87234598724867609478353436890268_i128)
        ];
        assert_eq!(builtin.get_range_check_usage(&memory), Some((10480, 42341)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_empty_memory() {
        let builtin = RangeCheckBuiltinRunner::new(8, 8, true);
        let memory = Memory::new();
        assert_eq!(builtin.get_range_check_usage(&memory), None);
    }

    /// Test that the method get_used_perm_range_check_units works as intended.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_perm_range_check_units() {
        let builtin_runner = RangeCheckBuiltinRunner::new(8, 8, true);
        let mut vm = vm!();

        vm.current_step = 8;
        vm.segments.segment_used_sizes = Some(vec![1]);
        assert_eq!(builtin_runner.get_used_perm_range_check_units(&vm), Ok(8));
    }
}
