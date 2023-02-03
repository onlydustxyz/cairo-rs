use cairo_vm::{
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
    utils::load_program, vm::vm_core::VirtualMachine,
};

use cairo_vm::vm::{runners::cairo_runner::CairoRunner, trace::trace_entry::RelocatedTraceEntry};

#[test]
fn struct_integration_test() {
    let program = load_program(
        #[cfg(feature = "std")]
        "cairo_programs/struct.json",
        #[cfg(not(feature = "std"))]
        include_str!("../cairo_programs/struct.json"),
        Some("main"),
    );

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
