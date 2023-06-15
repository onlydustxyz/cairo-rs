use crate::stdlib::prelude::*;

use crate::{
    cairo_run::{cairo_run, CairoRunConfig},
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
    vm::trace::trace_entry::TraceEntry,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

mod bitwise_test;
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
