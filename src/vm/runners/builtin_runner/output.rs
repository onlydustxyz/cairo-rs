use std::prelude::v1::*;

use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::MemoryError;
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::vm_core::VirtualMachine;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;

#[derive(Debug, Clone)]
pub struct OutputBuiltinRunner {
    base: isize,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) included: bool,
}

impl OutputBuiltinRunner {
    pub fn new(included: bool) -> OutputBuiltinRunner {
        OutputBuiltinRunner {
            base: 0,
            stop_ptr: None,
            included,
        }
    }

    pub fn initialize_segments(
        &mut self,
        segments: &mut MemorySegmentManager,
        memory: &mut Memory,
    ) {
        self.base = segments.add(memory).segment_index
    }

    pub fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        if self.included {
            vec![MaybeRelocatable::from((self.base, 0))]
        } else {
            vec![]
        }
    }

    pub fn base(&self) -> isize {
        self.base
    }

    pub fn add_validation_rule(&self, _memory: &mut Memory) -> Result<(), RunnerError> {
        Ok(())
    }

    pub fn deduce_memory_cell(
        &self,
        _address: &Relocatable,
        _memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        Ok(None)
    }

    pub fn get_allocated_memory_units(&self, _vm: &VirtualMachine) -> Result<usize, MemoryError> {
        Ok(0)
    }

    pub fn get_memory_segment_addresses(&self) -> (isize, Option<usize>) {
        (self.base, self.stop_ptr)
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        let base = self.base();
        segments
            .get_segment_used_size(
                base.try_into()
                    .map_err(|_| MemoryError::AddressInTemporarySegment(base))?,
            )
            .ok_or(MemoryError::MissingSegmentUsedSizes)
    }

    pub fn get_used_cells_and_allocated_size(
        &self,
        vm: &VirtualMachine,
    ) -> Result<(usize, usize), MemoryError> {
        let used = self.get_used_cells(&vm.segments)?;
        Ok((used, used))
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
        memory: &Memory,
        pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        if self.included {
            if let Ok(stop_pointer) = memory
                .get_relocatable(&(pointer.sub_usize(1)).map_err(|_| RunnerError::FinalStack)?)
            {
                if self.base() != stop_pointer.segment_index {
                    return Err(RunnerError::InvalidStopPointer("output".to_string()));
                }
                let stop_ptr = stop_pointer.offset;
                let used = self
                    .get_used_cells(segments)
                    .map_err(|_| RunnerError::FinalStack)?;
                if stop_ptr != used {
                    return Err(RunnerError::InvalidStopPointer("output".to_string()));
                }

                self.stop_ptr = Some(stop_ptr);
                Ok(pointer.sub_usize(1).map_err(|_| RunnerError::FinalStack)?)
            } else {
                Err(RunnerError::FinalStack)
            }
        } else {
            let stop_ptr = self.base() as usize;
            self.stop_ptr = Some(stop_ptr);
            Ok(pointer)
        }
    }
}

impl Default for OutputBuiltinRunner {
    fn default() -> Self {
        Self::new(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        utils::test_utils::*,
        vm::{
            errors::memory_errors::MemoryError, runners::builtin_runner::BuiltinRunner,
            vm_core::VirtualMachine,
        },
    };

    #[test]
    fn get_used_instances() {
        let builtin = OutputBuiltinRunner::new(true);

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    fn final_stack() {
        let mut builtin = OutputBuiltinRunner::new(true);

        let mut vm = vm!();

        vm.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin
                .final_stack(&vm.segments, &vm.memory, pointer)
                .unwrap(),
            Relocatable::from((2, 1))
        );
    }

    #[test]
    fn final_stack_error_stop_pointer() {
        let mut builtin = OutputBuiltinRunner::new(true);

        let mut vm = vm!();

        vm.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![999]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, &vm.memory, pointer),
            Err(RunnerError::InvalidStopPointer("output".to_string()))
        );
    }

    #[test]
    fn final_stack_error_when_notincluded() {
        let mut builtin = OutputBuiltinRunner::new(false);

        let mut vm = vm!();

        vm.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin
                .final_stack(&vm.segments, &vm.memory, pointer)
                .unwrap(),
            Relocatable::from((2, 2))
        );
    }

    #[test]
    fn final_stack_error_non_relocatable() {
        let mut builtin = OutputBuiltinRunner::new(true);

        let mut vm = vm!();

        vm.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, &vm.memory, pointer),
            Err(RunnerError::FinalStack)
        );
    }

    #[test]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner = OutputBuiltinRunner::new(true).into();

        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);

        assert_eq!(
            builtin.get_used_cells_and_allocated_size(&vm),
            Ok((0_usize, 0))
        );
    }

    #[test]
    fn get_allocated_memory_units() {
        let builtin = OutputBuiltinRunner::new(true);

        let vm = vm!();

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(0));
    }

    #[test]
    fn initialize_segments_for_output() {
        let mut builtin = OutputBuiltinRunner::new(true);
        let mut segments = MemorySegmentManager::new();
        let mut memory = Memory::new();
        builtin.initialize_segments(&mut segments, &mut memory);
        assert_eq!(builtin.base, 0);
    }

    #[test]
    fn get_initial_stack_for_output_with_base() {
        let mut builtin = OutputBuiltinRunner::new(true);
        builtin.base = 1;
        let initial_stack = builtin.initial_stack();
        assert_eq!(
            initial_stack[0].clone(),
            MaybeRelocatable::RelocatableValue((builtin.base(), 0).into())
        );
        assert_eq!(initial_stack.len(), 1);
    }

    #[test]
    fn get_memory_segment_addresses() {
        let builtin = OutputBuiltinRunner::new(true);

        assert_eq!(builtin.get_memory_segment_addresses(), (0, None),);
    }

    #[test]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    fn get_memory_accesses_empty() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    fn get_memory_accesses() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
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
    fn get_used_cells_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    fn get_used_cells_empty() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    fn get_used_cells() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    fn test_get_used_instances_missing_segments() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let memory_segment_manager = MemorySegmentManager::new();

        assert_eq!(
            builtin.get_used_instances(&memory_segment_manager),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    fn test_get_used_instances_valid() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let mut memory_segment_manager = MemorySegmentManager::new();
        memory_segment_manager.segment_used_sizes = Some(vec![0]);

        assert_eq!(builtin.get_used_instances(&memory_segment_manager), Ok(0));
    }

    #[test]
    fn test_deduce_memory_cell_output_builtin() {
        let builtin = BuiltinRunner::Output(OutputBuiltinRunner::new(true));
        let mut vm = vm!();

        vm.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(builtin.deduce_memory_cell(&pointer, &vm.memory), Ok(None));
    }

    #[test]
    fn test_add_validation_rule() {
        let builtin = OutputBuiltinRunner::new(true);
        let mut vm = vm!();

        vm.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        assert_eq!(builtin.add_validation_rule(&mut vm.memory), Ok(()));
    }
}
