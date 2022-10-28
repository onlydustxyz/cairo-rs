use std::any::Any;

use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::runners::builtin_runner::BuiltinRunner;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;

pub struct OutputBuiltinRunner {
    base: isize,
    _stop_ptr: Option<Relocatable>,
}

impl OutputBuiltinRunner {
    pub fn new() -> OutputBuiltinRunner {
        OutputBuiltinRunner {
            base: 0,
            _stop_ptr: None,
        }
    }
}

impl BuiltinRunner for OutputBuiltinRunner {
    fn initialize_segments(&mut self, segments: &mut MemorySegmentManager, memory: &mut Memory) {
        self.base = segments.add(memory).segment_index
    }

    fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        vec![MaybeRelocatable::from((self.base, 0))]
    }

    fn base(&self) -> isize {
        self.base
    }

    fn add_validation_rule(&self, _memory: &mut Memory) -> Result<(), RunnerError> {
        Ok(())
    }

    fn deduce_memory_cell(
        &mut self,
        _address: &Relocatable,
        _memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        Ok(None)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Default for OutputBuiltinRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        utils::test_utils::vm,
        vm::{errors::memory_errors::MemoryError, vm_core::VirtualMachine},
    };
    use num_bigint::{BigInt, Sign};

    #[test]
    fn initialize_segments_for_output() {
        let mut builtin = OutputBuiltinRunner::new();
        let mut segments = MemorySegmentManager::new();
        let mut memory = Memory::new();
        builtin.initialize_segments(&mut segments, &mut memory);
        assert_eq!(builtin.base, 0);
    }

    #[test]
    fn get_initial_stack_for_output_with_base() {
        let mut builtin = OutputBuiltinRunner::new();
        builtin.base = 1;
        let initial_stack = builtin.initial_stack();
        assert_eq!(
            initial_stack[0].clone(),
            MaybeRelocatable::RelocatableValue((builtin.base(), 0).into())
        );
        assert_eq!(initial_stack.len(), 1);
    }

    #[test]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin = OutputBuiltinRunner::new();
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    fn get_memory_accesses_empty() {
        let builtin = OutputBuiltinRunner::new();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    fn get_memory_accesses() {
        let builtin = OutputBuiltinRunner::new();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Ok(vec![
                (builtin.base, 0).into(),
                (builtin.base, 1).into(),
                (builtin.base, 2).into(),
                (builtin.base, 3).into(),
            ]),
        );
    }
}
