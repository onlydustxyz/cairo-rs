use std::path::Path;

use cairo_vm::{
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
    types::program::Program,
    vm::trace::trace_entry::RelocatedTraceEntry,
    vm::{runners::cairo_runner::CairoRunner, vm_core::VirtualMachine},
};

#[test]
fn pedersen_integration_test() {
    let program = load_program("cairo_programs/pedersen_test.json", Some("main"));
    let mut hint_processor = BuiltinHintProcessor::new_empty();
    let mut cairo_runner = CairoRunner::new(&program, "all", false).unwrap();
    let mut vm = VirtualMachine::new(true);
    let end = cairo_runner.initialize(&mut vm).unwrap();
    assert_eq!(
        cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
        Ok(())
    );
    assert!(cairo_runner.relocate(&mut vm) == Ok(()), "Execution failed");

    let python_vm_relocated_trace: Vec<RelocatedTraceEntry> = vec![
        RelocatedTraceEntry {
            pc: 7,
            ap: 25,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 8,
            ap: 26,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 10,
            ap: 27,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 12,
            ap: 28,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 1,
            ap: 30,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 2,
            ap: 30,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 3,
            ap: 30,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 5,
            ap: 31,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 6,
            ap: 32,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 14,
            ap: 32,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 15,
            ap: 32,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 17,
            ap: 33,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 18,
            ap: 34,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 19,
            ap: 35,
            fp: 25,
        },
    ];
    assert_eq!(
        cairo_runner.relocated_trace,
        Some(python_vm_relocated_trace)
    );
}

fn load_program(path: &str, entrypoint: Option<&str>) -> Program {
    #[cfg(feature = "std")]
    let program = Program::from_file(Path::new(path), entrypoint)
        .expect("Call to `Program::from_file()` failed.");

    #[cfg(not(feature = "std"))]
    let program = { get_program_from_file(&format!("../{path}"), entrypoint) };

    program
}
