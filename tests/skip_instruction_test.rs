#[cfg(feature = "skip_next_instruction_hint")]
use cairo_vm::{
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
    utils::load_program,
    vm::{runners::cairo_runner::CairoRunner, vm_core::VirtualMachine},
};

#[cfg(feature = "skip_next_instruction_hint")]
#[cfg(feature = "skip_next_instruction_hint")]
#[test]
fn skip_next_instruction_test() {
    let program = load_program(
        #[cfg(feature = "std")]
        "cairo_programs/noretrocompat/test_skip_next_instruction.noretrocompat.json",
        #[cfg(not(feature = "std"))]
        include_str!(
            "../cairo_programs/noretrocompat/test_skip_next_instruction.noretrocompat.json"
        ),
        Some("main"),
    );

    let mut hint_processor = BuiltinHintProcessor::new_empty();

    let mut cairo_runner = CairoRunner::new(&program, "all", false).unwrap();
    let mut vm = VirtualMachine::new(false);
    let end = cairo_runner.initialize(&mut vm).unwrap();
    assert_eq!(
        cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
        Ok(())
    );
}

fn load_program(path: &str, entrypoint: Option<&str>) -> Program {
    #[cfg(feature = "std")]
    let program = Program::from_file(Path::new(path), entrypoint)
        .expect("Call to `Program::from_file()` failed.");

    #[cfg(not(feature = "std"))]
    let program = {
        use serde::deserialize_program::{
            deserialize_program_json, parse_program_json, ProgramJson,
        };
        get_program_from_file(&format!("../{path}"), entrypoint)
    };

    program
}
