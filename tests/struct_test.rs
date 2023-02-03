use cairo_vm::{
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
    vm::vm_core::VirtualMachine,
};
use std::path::Path;

use cairo_vm::{
    types::program::Program,
    vm::{runners::cairo_runner::CairoRunner, trace::trace_entry::RelocatedTraceEntry},
};

#[test]
fn struct_integration_test() {
    let program = load_program("cairo_programs/struct.json", Some("main"));

    let mut hint_processor = BuiltinHintProcessor::new_empty();
    let mut cairo_runner = CairoRunner::new(&program, "all", false).unwrap();
    let mut vm = VirtualMachine::new(true);
    let end = cairo_runner.initialize(&mut vm).unwrap();

    assert!(
        cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor) == Ok(()),
        "Execution failed"
    );
    assert!(cairo_runner.relocate(&mut vm) == Ok(()), "Execution failed");
    let relocated_entry = RelocatedTraceEntry {
        pc: 1,
        ap: 4,
        fp: 4,
    };

    assert_eq!(cairo_runner.relocated_trace, Some(vec![relocated_entry]));
}

fn load_program(path: &str, entrypoint: Option<&str>) -> Program {
    #[cfg(feature = "std")]
    let program = Program::from_file(Path::new(path), entrypoint)
        .expect("Call to `Program::from_file()` failed.");

    #[cfg(not(feature = "std"))]
    let program = { get_program_from_file(&format!("../{path}"), entrypoint) };

    program
}
