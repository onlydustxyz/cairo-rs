#[cfg(feature = "cairo-1-hints")]
use crate::vm::errors::cairo_run_errors::CairoRunError;
#[cfg(feature = "cairo-1-hints")]
use crate::vm::runners::cairo_runner::RunResources;
#[cfg(feature = "cairo-1-hints")]
use crate::{
    hint_processor::cairo_1_hint_processor::hint_processor::Cairo1HintProcessor,
    serde::deserialize_program::BuiltinName,
    types::relocatable::MaybeRelocatable,
    vm::{
        runners::cairo_runner::{CairoArg, CairoRunner},
        vm_core::VirtualMachine,
    },
};
#[cfg(feature = "cairo-1-hints")]
use cairo_lang_casm_contract_class::CasmContractClass;
#[cfg(feature = "cairo-1-hints")]
use felt::Felt252;

use crate::stdlib::prelude::*;

use crate::{
    cairo_run::{cairo_run, CairoRunConfig},
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
    vm::trace::trace_entry::TraceEntry,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{string::String, vec::Vec};

mod bitwise_test;

#[cfg(feature = "cairo-1-hints")]
mod cairo_1_run_from_entrypoint_tests;
mod cairo_run_test;
mod pedersen_test;
mod struct_test;

#[cfg(feature = "skip_next_instruction_hint")]
mod skip_instruction_test;

//For simple programs that should just succeed and have no special needs.
//Checks memory holes == 0
pub(self) fn run_program_simple(data: &[u8]) {
    run_program(data, Some("all_cairo"), None, None, Some(0))
}

//For simple programs that should just succeed and have no special needs.
//Checks memory holes
pub(self) fn run_program_simple_with_memory_holes(data: &[u8], holes: usize) {
    run_program(data, Some("all_cairo"), None, None, Some(holes))
}

//For simple programs that should just succeed but using small layout.
pub(self) fn run_program_small(data: &[u8]) {
    run_program(data, Some("small"), None, None, None)
}

pub(self) fn run_program_with_trace(data: &[u8], trace: &[(usize, usize, usize)]) {
    run_program(data, Some("all_cairo"), Some(trace), None, None)
}

pub(self) fn run_program_with_error(data: &[u8], error: &str) {
    run_program(data, Some("all_cairo"), None, Some(error), None)
}

pub(self) fn run_program(
    data: &[u8],
    layout: Option<&str>,
    trace: Option<&[(usize, usize, usize)]>,
    error: Option<&str>,
    memory_holes: Option<usize>,
) {
    let mut hint_executor = BuiltinHintProcessor::new_empty();
    let cairo_run_config = CairoRunConfig {
        layout: layout.unwrap_or("all_cairo"),
        relocate_mem: true,
        trace_enabled: true,
        ..Default::default()
    };
    let res = cairo_run(data, &cairo_run_config, &mut hint_executor);
    if let Some(error) = error {
        assert!(res.is_err());
        assert!(res.err().unwrap().to_string().contains(error));
        return;
    }
    let (runner, vm) = res.expect("Execution failed");
    if let Some(trace) = trace {
        let expected_trace: Vec<_> = trace
            .iter()
            .copied()
            .map(|(pc, ap, fp)| TraceEntry { pc, ap, fp })
            .collect();
        let trace = vm.get_relocated_trace().unwrap();
        assert_eq!(trace.len(), expected_trace.len());
        for (entry, expected) in trace.iter().zip(expected_trace.iter()) {
            assert_eq!(entry, expected);
        }
    }
    if let Some(holes) = memory_holes {
        assert_eq!(runner.get_memory_holes(&vm).unwrap(), holes);
    }
}

#[cfg(feature = "cairo-1-hints")]
// Runs a contract entrypoint with given arguments and checks its return values
// Doesn't use a syscall_handler
pub(self) fn run_cairo_1_entrypoint(
    program_content: &[u8],
    entrypoint_offset: usize,
    args: &[MaybeRelocatable],
    expected_retdata: &[Felt252],
) {
    let contract_class: CasmContractClass = serde_json::from_slice(program_content).unwrap();
    let mut hint_processor = Cairo1HintProcessor::new(&contract_class.hints);

    let mut runner = CairoRunner::new(
        &(contract_class.clone().try_into().unwrap()),
        "all_cairo",
        false,
    )
    .unwrap();
    let mut vm = VirtualMachine::new(false);

    let program_builtins = get_casm_contract_builtins(&contract_class, entrypoint_offset);
    runner
        .initialize_function_runner_cairo_1(&mut vm, &program_builtins)
        .unwrap();

    // Implicit Args
    let syscall_segment = MaybeRelocatable::from(vm.add_memory_segment());

    let builtins: Vec<&'static str> = runner
        .get_program_builtins()
        .iter()
        .map(|b| b.name())
        .collect();

    let builtin_segment: Vec<MaybeRelocatable> = vm
        .get_builtin_runners()
        .iter()
        .filter(|b| builtins.contains(&b.name()))
        .flat_map(|b| b.initial_stack())
        .collect();

    let initial_gas = MaybeRelocatable::from(usize::MAX);

    let mut implicit_args = builtin_segment;
    implicit_args.extend([initial_gas]);
    implicit_args.extend([syscall_segment]);

    // Other args

    // Load builtin costs
    let builtin_costs: Vec<MaybeRelocatable> =
        vec![0.into(), 0.into(), 0.into(), 0.into(), 0.into()];
    let builtin_costs_ptr = vm.add_memory_segment();
    vm.load_data(builtin_costs_ptr, &builtin_costs).unwrap();

    // Load extra data
    let core_program_end_ptr =
        (runner.program_base.unwrap() + runner.program.shared_program_data.data.len()).unwrap();
    let program_extra_data: Vec<MaybeRelocatable> =
        vec![0x208B7FFF7FFF7FFE.into(), builtin_costs_ptr.into()];
    vm.load_data(core_program_end_ptr, &program_extra_data)
        .unwrap();

    // Load calldata
    let calldata_start = vm.add_memory_segment();
    let calldata_end = vm.load_data(calldata_start, &args.to_vec()).unwrap();

    // Create entrypoint_args

    let mut entrypoint_args: Vec<CairoArg> = implicit_args
        .iter()
        .map(|m| CairoArg::from(m.clone()))
        .collect();
    entrypoint_args.extend([
        MaybeRelocatable::from(calldata_start).into(),
        MaybeRelocatable::from(calldata_end).into(),
    ]);
    let entrypoint_args: Vec<&CairoArg> = entrypoint_args.iter().collect();

    // Run contract entrypoint

    runner
        .run_from_entrypoint(
            entrypoint_offset,
            &entrypoint_args,
            &mut RunResources::default(),
            true,
            Some(runner.program.shared_program_data.data.len() + program_extra_data.len()),
            &mut vm,
            &mut hint_processor,
        )
        .unwrap();

    // Check return values
    let return_values = vm.get_return_values(5).unwrap();
    let retdata_start = return_values[3].get_relocatable().unwrap();
    let retdata_end = return_values[4].get_relocatable().unwrap();
    let retdata: Vec<Felt252> = vm
        .get_integer_range(retdata_start, (retdata_end - retdata_start).unwrap())
        .unwrap()
        .iter()
        .map(|c| c.clone().into_owned())
        .collect();
    assert_eq!(expected_retdata, &retdata);
}

#[cfg(feature = "cairo-1-hints")]
/// Equals to fn run_cairo_1_entrypoint
/// But with run_resources as an input
pub(self) fn run_cairo_1_entrypoint_with_run_resources(
    program_content: &[u8],
    entrypoint_offset: usize,
    run_resources: &mut RunResources,
    args: &[MaybeRelocatable],
) -> Result<Vec<Felt252>, CairoRunError> {
    let contract_class: CasmContractClass = serde_json::from_slice(program_content).unwrap();
    let mut hint_processor = Cairo1HintProcessor::new(&contract_class.hints);

    let mut runner = CairoRunner::new(
        &(contract_class.clone().try_into().unwrap()),
        "all_cairo",
        false,
    )
    .unwrap();
    let mut vm = VirtualMachine::new(false);

    let program_builtins = get_casm_contract_builtins(&contract_class, entrypoint_offset);
    runner
        .initialize_function_runner_cairo_1(&mut vm, &program_builtins)
        .unwrap();

    // Implicit Args
    let syscall_segment = MaybeRelocatable::from(vm.add_memory_segment());

    let builtins: Vec<&'static str> = runner
        .get_program_builtins()
        .iter()
        .map(|b| b.name())
        .collect();

    let builtin_segment: Vec<MaybeRelocatable> = vm
        .get_builtin_runners()
        .iter()
        .filter(|b| builtins.contains(&b.name()))
        .flat_map(|b| b.initial_stack())
        .collect();

    let initial_gas = MaybeRelocatable::from(usize::MAX);

    let mut implicit_args = builtin_segment;
    implicit_args.extend([initial_gas]);
    implicit_args.extend([syscall_segment]);

    // Other args

    // Load builtin costs
    let builtin_costs: Vec<MaybeRelocatable> =
        vec![0.into(), 0.into(), 0.into(), 0.into(), 0.into()];
    let builtin_costs_ptr = vm.add_memory_segment();
    vm.load_data(builtin_costs_ptr, &builtin_costs).unwrap();

    // Load extra data
    let core_program_end_ptr =
        (runner.program_base.unwrap() + runner.program.shared_program_data.data.len()).unwrap();
    let program_extra_data: Vec<MaybeRelocatable> =
        vec![0x208B7FFF7FFF7FFE.into(), builtin_costs_ptr.into()];
    vm.load_data(core_program_end_ptr, &program_extra_data)
        .unwrap();

    // Load calldata
    let calldata_start = vm.add_memory_segment();
    let calldata_end = vm.load_data(calldata_start, &args.to_vec()).unwrap();

    // Create entrypoint_args

    let mut entrypoint_args: Vec<CairoArg> = implicit_args
        .iter()
        .map(|m| CairoArg::from(m.clone()))
        .collect();
    entrypoint_args.extend([
        MaybeRelocatable::from(calldata_start).into(),
        MaybeRelocatable::from(calldata_end).into(),
    ]);
    let entrypoint_args: Vec<&CairoArg> = entrypoint_args.iter().collect();

    // Run contract entrypoint

    runner.run_from_entrypoint(
        entrypoint_offset,
        &entrypoint_args,
        run_resources,
        true,
        Some(runner.program.shared_program_data.data.len() + program_extra_data.len()),
        &mut vm,
        &mut hint_processor,
    )?;

    // Check return values
    let return_values = vm.get_return_values(5).unwrap();
    let retdata_start = return_values[3].get_relocatable().unwrap();
    let retdata_end = return_values[4].get_relocatable().unwrap();
    let retdata: Vec<Felt252> = vm
        .get_integer_range(retdata_start, (retdata_end - retdata_start).unwrap())
        .unwrap()
        .iter()
        .map(|c| c.clone().into_owned())
        .collect();
    Ok(retdata)
}

#[cfg(feature = "cairo-1-hints")]
fn get_casm_contract_builtins(
    contract_class: &CasmContractClass,
    entrypoint_offset: usize,
) -> Vec<BuiltinName> {
    contract_class
        .entry_points_by_type
        .external
        .iter()
        .find(|e| e.offset == entrypoint_offset)
        .unwrap()
        .builtins
        .iter()
        .map(|n| format!("{}_builtin", n))
        .map(|s| match &*s {
            crate::vm::runners::builtin_runner::OUTPUT_BUILTIN_NAME => BuiltinName::output,
            crate::vm::runners::builtin_runner::RANGE_CHECK_BUILTIN_NAME => {
                BuiltinName::range_check
            }
            crate::vm::runners::builtin_runner::HASH_BUILTIN_NAME => BuiltinName::pedersen,
            crate::vm::runners::builtin_runner::SIGNATURE_BUILTIN_NAME => BuiltinName::ecdsa,
            crate::vm::runners::builtin_runner::KECCAK_BUILTIN_NAME => BuiltinName::keccak,
            crate::vm::runners::builtin_runner::BITWISE_BUILTIN_NAME => BuiltinName::bitwise,
            crate::vm::runners::builtin_runner::EC_OP_BUILTIN_NAME => BuiltinName::ec_op,
            crate::vm::runners::builtin_runner::POSEIDON_BUILTIN_NAME => BuiltinName::poseidon,
            crate::vm::runners::builtin_runner::SEGMENT_ARENA_BUILTIN_NAME => {
                BuiltinName::segment_arena
            }
            _ => panic!("Invalid builtin {}", s),
        })
        .collect()
}
