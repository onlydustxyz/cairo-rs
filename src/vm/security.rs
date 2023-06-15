use crate::stdlib::prelude::*;

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
///   - (Only if `verify_builtins` is set to true) All accesses to the builtin segments must be within the range defined by
///     the builtins themselves.
///   - There must not be accesses to the program segment outside the program
///     data range. This check will use the `program_segment_size` instead of the program data length if available.
///   - All addresses in memory must be real (not temporary)
///
/// Note: Each builtin is responsible for checking its own segments' data.
pub fn verify_secure_runner(
    runner: &CairoRunner,
    verify_builtins: bool,
    program_segment_size: Option<usize>,
    vm: &mut VirtualMachine,
) -> Result<(), VirtualMachineError> {
    let builtins_segment_info = match verify_builtins {
        true => runner.get_builtin_segments_info(vm)?,
        false => Vec::new(),
    };
    // Check builtin segment out of bounds.
    for (index, stop_ptr) in builtins_segment_info {
        let current_size = vm
            .segments
            .memory
            .data
            .get(index)
            .map(|segment| segment.len());
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
    let program_segment_size =
        program_segment_size.unwrap_or(runner.program.shared_program_data.data.len());
    let program_length = vm
        .segments
        .memory
        .data
        .get(program_segment_index)
        .map(|segment| segment.len());
    // + 1 here accounts for maximum segment offset being segment.len() -1
    if program_length >= Some(program_segment_size + 1) {
        return Err(VirtualMachineError::OutOfBoundsProgramSegmentAccess);
    }
    // Check that the addresses in memory are valid
    // This means that every temporary address has been properly relocated to a real address
    // Asumption: If temporary memory is empty, this means no temporary memory addresses were generated and all addresses in memory are real
    if !vm.segments.memory.temp_data.is_empty() {
        for value in vm.segments.memory.data.iter().flatten() {
            match value.as_ref().map(|x| x.get_value()) {
                Some(MaybeRelocatable::RelocatableValue(addr)) if addr.segment_index < 0 => {
                    return Err(VirtualMachineError::InvalidMemoryValueTemporaryAddress(
                        *addr,
                    ))
                }
                _ => {}
            }
        }
    }
    for builtin in vm.builtin_runners.iter() {
        builtin.run_security_checks(vm)?;
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::serde::deserialize_program::BuiltinName;
    use crate::stdlib::collections::HashMap;

    use crate::types::relocatable::Relocatable;

    use crate::{relocatable, types::program::Program, utils::test_utils::*};
    use assert_matches::assert_matches;
    use felt::Felt252;
    use num_traits::Zero;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_without_program_base() {
        let program = program!();

        let runner = cairo_runner!(program);
        let mut vm = vm!();

        assert_matches!(
            verify_secure_runner(&runner, true, None, &mut vm),
            Err(VirtualMachineError::RunnerError(RunnerError::NoProgBase))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_empty_memory() {
        let program = program!(main = Some(0),);

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();
        vm.segments.compute_effective_sizes();
        assert_matches!(verify_secure_runner(&runner, true, None, &mut vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_program_access_out_of_bounds() {
        let program = program!(main = Some(0),);

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();

        vm.segments = segments![((0, 0), 100)];
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_matches!(
            verify_secure_runner(&runner, true, None, &mut vm),
            Err(VirtualMachineError::OutOfBoundsProgramSegmentAccess)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_program_with_program_size() {
        let program = program!(main = Some(0),);

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();

        vm.segments = segments![((0, 0), 100)];
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_matches!(
            verify_secure_runner(&runner, true, Some(1), &mut vm),
            Ok(())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_builtin_access_out_of_bounds() {
        let program = program!(main = Some(0), builtins = vec![BuiltinName::range_check],);

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();
        runner.initialize(&mut vm).unwrap();
        vm.builtin_runners[0].set_stop_ptr(0);
        vm.segments.memory = memory![((2, 0), 1)];
        vm.segments.segment_used_sizes = Some(vec![0, 0, 0, 0]);

        assert_matches!(
            verify_secure_runner(&runner, true, None, &mut vm),
            Err(VirtualMachineError::OutOfBoundsBuiltinSegmentAccess)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_builtin_access_correct() {
        let program = program!(main = Some(0), builtins = vec![BuiltinName::range_check],);

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();
        runner.initialize(&mut vm).unwrap();
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        runner
            .end_run(false, false, &mut vm, &mut hint_processor)
            .unwrap();
        vm.builtin_runners[0].set_stop_ptr(1);

        vm.segments.memory = memory![((2, 0), 1)];
        vm.segments.segment_used_sizes = Some(vec![0, 0, 1, 0]);

        assert_matches!(verify_secure_runner(&runner, true, None, &mut vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_success() {
        let program = program!(
            data = vec![
                Felt252::zero().into(),
                Felt252::zero().into(),
                Felt252::zero().into(),
                Felt252::zero().into(),
            ],
            main = Some(0),
        );

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();
        vm.segments.memory = memory![
            ((0, 0), (1, 0)),
            ((0, 1), (2, 1)),
            ((0, 2), (3, 2)),
            ((0, 3), (4, 3))
        ];
        vm.segments.segment_used_sizes = Some(vec![5, 1, 2, 3, 4]);

        assert_matches!(verify_secure_runner(&runner, true, None, &mut vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_temporary_memory_properly_relocated() {
        let program = program!(
            data = vec![
                Felt252::zero().into(),
                Felt252::zero().into(),
                Felt252::zero().into(),
                Felt252::zero().into(),
            ],
            main = Some(0),
        );

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();
        vm.segments.memory = memory![
            ((0, 1), (1, 0)),
            ((0, 2), (2, 1)),
            ((0, 3), (3, 2)),
            ((-1, 0), (1, 2))
        ];
        vm.segments.segment_used_sizes = Some(vec![5, 1, 2, 3, 4]);

        assert_matches!(verify_secure_runner(&runner, true, None, &mut vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn verify_secure_runner_temporary_memory_not_fully_relocated() {
        let program = program!(
            data = vec![
                Felt252::zero().into(),
                Felt252::zero().into(),
                Felt252::zero().into(),
                Felt252::zero().into(),
            ],
            main = Some(0),
        );

        let mut runner = cairo_runner!(program);
        let mut vm = vm!();

        runner.initialize(&mut vm).unwrap();
        vm.segments.memory = memory![
            ((0, 0), (1, 0)),
            ((0, 1), (2, 1)),
            ((0, 2), (-3, 2)),
            ((0, 3), (4, 3)),
            ((-1, 0), (1, 2))
        ];
        vm.segments.segment_used_sizes = Some(vec![5, 1, 2, 3, 4]);

        assert_matches!(
            verify_secure_runner(&runner, true, None, &mut vm),
            Err(VirtualMachineError::InvalidMemoryValueTemporaryAddress(
                x
            )) if x == relocatable!(-3, 2)
        );
    }
}
