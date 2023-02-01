use std::prelude::v1::*;

use num_traits::ToPrimitive;

use super::{
    errors::{runner_errors::RunnerError, vm_errors::VirtualMachineError},
    runners::cairo_runner::CairoRunner,
    vm_core::VirtualMachine,
};
use crate::types::relocatable::MaybeRelocatable;

/// Verify that the completed run in a runner is safe to be relocated and be
/// used by other Cairo programs.
///
/// Checks include:
///   - All accesses to the builtin segments must be within the range defined by
///     the builtins themselves.
///   - There must not be accesses to the program segment outside the program
///     data range.
///   - All addresses in memory must be real (not temporary)
///
/// Note: Each builtin is responsible for checking its own segments' data.
pub fn verify_secure_runner(
    runner: &CairoRunner,
    verify_builtins: bool,
    vm: &mut VirtualMachine,
) -> Result<(), VirtualMachineError> {
    let builtins_segment_info = match verify_builtins {
        true => runner.get_builtin_segments_info(vm)?,
        false => Vec::new(),
    };
    // Check builtin segment out of bounds.
    for (index, stop_ptr) in builtins_segment_info {
        let current_size = vm.memory.data.get(index).map(|segment| segment.len());
        // + 1 here accounts for maximum segment offset being segment.len() -1
        if current_size >= Some(stop_ptr + 1) {
            return Err(VirtualMachineError::OutOfBoundsBuiltinSegmentAccess);
        }
    }
    // Check out of bounds for program segment.
    let program_segment_index = runner
        .program_base
        .and_then(|rel| rel.segment_index.to_usize())
        .ok_or(RunnerError::NoProgBase)?;
    let program_segment_size = vm
        .memory
        .data
        .get(program_segment_index)
        .map(|segment| segment.len());
    // + 1 here accounts for maximum segment offset being segment.len() -1
    if program_segment_size >= Some(runner.program.data.len() + 1) {
        return Err(VirtualMachineError::OutOfBoundsProgramSegmentAccess);
    }
    // Check that the addresses in memory are valid
    // This means that every temporary address has been properly relocated to a real address
    // Asumption: If temporary memory is empty, this means no temporary memory addresses were generated and all addresses in memory are real
    if !vm.memory.temp_data.is_empty() {
        for value in vm.memory.data.iter().flatten() {
            match value {
                Some(MaybeRelocatable::RelocatableValue(addr)) if addr.segment_index < 0 => {
                    return Err(VirtualMachineError::InvalidMemoryValueTemporaryAddress(
                        *addr,
                    ))
                }
                _ => {}
            }
        }
    }
    for (_, builtin) in vm.builtin_runners.iter() {
        builtin.run_security_checks(vm)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::types::relocatable::MaybeRelocatable;
    use crate::types::relocatable::Relocatable;
    use crate::vm::errors::memory_errors::MemoryError;
    use crate::vm::vm_memory::memory::Memory;
    use crate::{relocatable, types::program::Program, utils::test_utils::*};
    use felt::Felt;
    use num_traits::Zero;

    #[test]
    fn verify_secure_runner_without_program_base() {
        let program = program!();

        let runner = cairo_runner!(program);
        let mut vm = vm!();

        assert_eq!(
            verify_secure_runner(&runner, true, &mut vm),
            Err(RunnerError::NoProgBase.into()),
        );
    }

    #[test]
    fn verify_secure_runner_empty_memory() {
        let program = program!(main = Some(0),);

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();
        vm.segments.compute_effective_sizes(&vm.memory);
        assert_eq!(verify_secure_runner(&runner, true, &mut vm), Ok(()));
    }

    #[test]
    fn verify_secure_runner_program_access_out_of_bounds() {
        let program = program!(main = Some(0),);

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();

        vm.memory = memory![((0, 0), 100)];
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(
            verify_secure_runner(&runner, true, &mut vm),
            Err(VirtualMachineError::OutOfBoundsProgramSegmentAccess)
        );
    }

    #[test]
    fn verify_secure_runner_builtin_access_out_of_bounds() {
        let program = program!(main = Some(0), builtins = vec!["range_check".to_string()],);

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();
        runner.initialize(&mut vm).unwrap();
        vm.builtin_runners[0].1.set_stop_ptr(0);

        vm.memory.data = vec![vec![], vec![], vec![Some(mayberelocatable!(1))]];
        vm.segments.segment_used_sizes = Some(vec![0, 0, 0, 0]);

        assert_eq!(
            verify_secure_runner(&runner, true, &mut vm),
            Err(VirtualMachineError::OutOfBoundsBuiltinSegmentAccess)
        );
    }

    #[test]
    fn verify_secure_runner_builtin_access_correct() {
        let program = program!(main = Some(0), builtins = vec!["range_check".to_string()],);

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();
        runner.initialize(&mut vm).unwrap();
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        runner
            .end_run(false, false, &mut vm, &mut hint_processor)
            .unwrap();
        vm.builtin_runners[0].1.set_stop_ptr(1);

        vm.memory.data = vec![vec![], vec![], vec![Some(mayberelocatable!(1))]];
        vm.segments.segment_used_sizes = Some(vec![0, 0, 1, 0]);

        assert_eq!(verify_secure_runner(&runner, true, &mut vm), Ok(()));
    }

    #[test]
    fn verify_secure_runner_success() {
        let program = program!(
            data = vec![
                Felt::zero().into(),
                Felt::zero().into(),
                Felt::zero().into(),
                Felt::zero().into(),
            ],
            main = Some(0),
        );

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();

        vm.memory.data = vec![vec![
            Some(relocatable!(1, 0).into()),
            Some(relocatable!(2, 1).into()),
            Some(relocatable!(3, 2).into()),
            Some(relocatable!(4, 3).into()),
        ]];
        vm.segments.segment_used_sizes = Some(vec![5, 1, 2, 3, 4]);

        assert_eq!(verify_secure_runner(&runner, true, &mut vm), Ok(()));
    }

    #[test]
    fn verify_secure_runner_temporary_memory_properly_relocated() {
        let program = program!(
            data = vec![
                Felt::zero().into(),
                Felt::zero().into(),
                Felt::zero().into(),
                Felt::zero().into(),
            ],
            main = Some(0),
        );

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();

        vm.memory.data = vec![vec![
            Some(relocatable!(1, 0).into()),
            Some(relocatable!(2, 1).into()),
            Some(relocatable!(3, 2).into()),
            Some(relocatable!(4, 3).into()),
        ]];
        vm.memory.temp_data = vec![vec![Some(relocatable!(1, 2).into())]];
        vm.segments.segment_used_sizes = Some(vec![5, 1, 2, 3, 4]);

        assert_eq!(verify_secure_runner(&runner, true, &mut vm), Ok(()));
    }

    #[test]
    fn verify_secure_runner_temporary_memory_not_fully_relocated() {
        let program = program!(
            data = vec![
                Felt::zero().into(),
                Felt::zero().into(),
                Felt::zero().into(),
                Felt::zero().into(),
            ],
            main = Some(0),
        );

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();

        vm.memory.data = vec![vec![
            Some(relocatable!(1, 0).into()),
            Some(relocatable!(2, 1).into()),
            Some(relocatable!(-3, 2).into()),
            Some(relocatable!(4, 3).into()),
        ]];
        vm.memory.temp_data = vec![vec![Some(relocatable!(1, 2).into())]];
        vm.segments.segment_used_sizes = Some(vec![5, 1, 2, 3, 4]);

        assert_eq!(
            verify_secure_runner(&runner, true, &mut vm),
            Err(VirtualMachineError::InvalidMemoryValueTemporaryAddress(
                relocatable!(-3, 2)
            ))
        );
    }
}
