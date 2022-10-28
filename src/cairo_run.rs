use crate::hint_processor::hint_processor_definition::HintProcessor;
use crate::types::program::Program;
use crate::vm::errors::{cairo_run_errors::CairoRunError, runner_errors::RunnerError};
use crate::vm::runners::cairo_runner::CairoRunner;
use crate::vm::trace::trace_entry::RelocatedTraceEntry;
use crate::vm::vm_core::VirtualMachine;
use num_bigint::BigInt;
use std::fs::File;
use std::io::{self, BufWriter, Error, ErrorKind, Write};
use std::path::Path;

pub fn cairo_run<'a>(
    path: &'a Path,
    entrypoint: &'a str,
    trace_enabled: bool,
    print_output: bool,
    hint_executor: &dyn HintProcessor,
) -> Result<CairoRunner, CairoRunError> {
    let program = match Program::new(path, entrypoint) {
        Ok(program) => program,
        Err(error) => return Err(CairoRunError::Program(error)),
    };

    let mut cairo_runner = CairoRunner::new(&program)?;
    let mut vm = VirtualMachine::new(program.prime, trace_enabled);
    let end = cairo_runner.initialize(&mut vm)?;

    cairo_runner
        .run_until_pc(end, &mut vm, hint_executor)
        .map_err(CairoRunError::VirtualMachine)?;

    vm.verify_auto_deductions()
        .map_err(CairoRunError::VirtualMachine)?;

    cairo_runner
        .relocate(&mut vm)
        .map_err(CairoRunError::Trace)?;

    if print_output {
        write_output(&mut cairo_runner, &mut vm)?;
    }

    Ok(cairo_runner)
}

pub fn write_output(
    cairo_runner: &mut CairoRunner,
    vm: &mut VirtualMachine,
) -> Result<(), CairoRunError> {
    let mut buffer = BufWriter::new(io::stdout());
    writeln!(&mut buffer, "Program Output: ")
        .map_err(|_| CairoRunError::Runner(RunnerError::WriteFail))?;
    cairo_runner
        .write_output(vm, &mut buffer)
        .map_err(CairoRunError::Runner)?;
    buffer
        .flush()
        .map_err(|_| CairoRunError::Runner(RunnerError::WriteFail))
}

/// Writes a trace as a binary file. Bincode encodes to little endian by default and each trace
/// entry is composed of 3 usize values that are padded to always reach 64 bit size.
pub fn write_binary_trace(
    relocated_trace: &[RelocatedTraceEntry],
    trace_file: &Path,
) -> io::Result<()> {
    let file = File::create(trace_file)?;
    let mut buffer = BufWriter::new(file);

    for (i, entry) in relocated_trace.iter().enumerate() {
        bincode::serialize_into(&mut buffer, entry).map_err(|e| {
            Error::new(
                ErrorKind::Other,
                format!("Failed to dump trace at position {i}, serialize error: {e}"),
            )
        })?;
    }

    buffer.flush()
}

/*
   Writes a binary memory file with the relocated memory as input.
   The memory pairs (address, value) are encoded and concatenated in the file
   given by the path `memory_file`.

   * address -> 8-byte encoded
   * value -> 32-byte encoded
*/
pub fn write_binary_memory(
    relocated_memory: &[Option<BigInt>],
    memory_file: &Path,
) -> io::Result<()> {
    let file = File::create(memory_file)?;
    let mut buffer = BufWriter::new(file);

    // initialize bytes vector that will be dumped to file
    let mut memory_bytes: Vec<u8> = Vec::new();

    for (i, memory_cell) in relocated_memory.iter().enumerate() {
        match memory_cell {
            None => continue,
            Some(unwrapped_memory_cell) => {
                encode_relocated_memory(&mut memory_bytes, i, unwrapped_memory_cell);
            }
        }
    }

    buffer.write_all(&memory_bytes)?;
    buffer.flush()
}

// encodes a given memory cell.
fn encode_relocated_memory(memory_bytes: &mut Vec<u8>, addr: usize, memory_cell: &BigInt) {
    // append memory address to bytes vector using a 8 bytes representation
    let mut addr_bytes = (addr as u64).to_le_bytes().to_vec();
    memory_bytes.append(&mut addr_bytes);

    // append memory value at address using a 32 bytes representation
    let mut value_bytes = memory_cell.to_signed_bytes_le();
    value_bytes.resize(32, 0);
    memory_bytes.append(&mut value_bytes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_processor::hint_processor_definition::HintProcessor;
    use crate::{
        bigint,
        hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
        utils::test_utils::vm,
    };
    use num_bigint::Sign;
    use std::io::Read;

    fn run_test_program(
        program_path: &Path,
        hint_processor: &dyn HintProcessor,
    ) -> Result<(CairoRunner, VirtualMachine), CairoRunError> {
        let program = Program::new(program_path, "main").map_err(CairoRunError::Program)?;

        let mut cairo_runner = CairoRunner::new(&program).unwrap();
        let mut vm = vm!(true);
        let end = cairo_runner
            .initialize(&mut vm)
            .map_err(CairoRunError::Runner)?;

        assert!(cairo_runner
            .run_until_pc(end, &mut vm, hint_processor)
            .is_ok());

        Ok((cairo_runner, vm))
    }

    #[test]
    fn cairo_run_custom_entry_point() {
        let program_path = Path::new("cairo_programs/not_main.json");
        let program = Program::new(program_path, "not_main").unwrap();
        let mut vm = vm!();
        let hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = CairoRunner::new(&program).unwrap();

        let end = cairo_runner.initialize(&mut vm).unwrap();
        assert!(cairo_runner
            .run_until_pc(end, &mut vm, &hint_processor)
            .is_ok());
        assert!(cairo_runner.relocate(&mut vm).is_ok());
        // `main` returns without doing nothing, but `not_main` sets `[ap]` to `1`
        // Memory location was found empirically and simply hardcoded
        assert_eq!(cairo_runner.relocated_memory[2], Some(bigint!(123)));
    }

    fn compare_files(file_path_1: &Path, file_path_2: &Path) -> io::Result<()> {
        let mut file_1 = File::open(file_path_1)?;
        let mut file_2 = File::open(file_path_2)?;

        let mut buffer_1 = Vec::new();
        let mut buffer_2 = Vec::new();

        file_1.read_to_end(&mut buffer_1)?;
        file_2.read_to_end(&mut buffer_2)?;

        assert_eq!(&buffer_1.len(), &buffer_2.len());

        for (buf_byte_1, buf_byte_2) in buffer_1.iter().zip(buffer_2.iter()) {
            assert_eq!(buf_byte_1, buf_byte_2);
        }
        Ok(())
    }

    #[test]
    fn cairo_run_with_no_data_program() {
        // a compiled program with no `data` key.
        // it should fail when the program is loaded.
        let no_data_program_path = Path::new("cairo_programs/no_data_program.json");
        let hint_processor = BuiltinHintProcessor::new_empty();
        assert!(cairo_run(no_data_program_path, "main", false, false, &hint_processor).is_err());
    }

    #[test]
    fn cairo_run_with_no_main_program() {
        // a compiled program with no main scope
        // it should fail when trying to run initialize_main_entrypoint.
        let no_main_program_path = Path::new("cairo_programs/no_main_program.json");
        let hint_processor = BuiltinHintProcessor::new_empty();
        assert!(cairo_run(no_main_program_path, "main", false, false, &hint_processor).is_err());
    }

    #[test]
    fn cairo_run_with_invalid_memory() {
        // the program invalid_memory.json has an invalid memory cell and errors when trying to
        // decode the instruction.
        let hint_executor = BuiltinHintProcessor::new_empty();
        let invalid_memory = Path::new("cairo_programs/invalid_memory.json");
        assert!(cairo_run(invalid_memory, "main", false, false, &hint_executor).is_err());
    }

    #[test]
    fn write_output_program() {
        let program_path = Path::new("cairo_programs/bitwise_output.json");
        let hint_processor = BuiltinHintProcessor::new_empty();
        let (mut cairo_runner, mut vm) = run_test_program(program_path, &hint_processor)
            .expect("Couldn't initialize cairo runner");
        assert!(write_output(&mut cairo_runner, &mut vm).is_ok());
    }

    #[test]
    fn write_binary_trace_file() {
        let program_path = Path::new("cairo_programs/struct.json");
        let expected_trace_path = Path::new("cairo_programs/trace_memory/cairo_trace_struct");
        let cairo_rs_trace_path = Path::new("cairo_programs/trace_memory/struct_cairo_rs.trace");

        // run test program until the end
        let hint_processor = BuiltinHintProcessor::new_empty();
        let cairo_runner_result = run_test_program(program_path, &hint_processor);
        let (mut cairo_runner, mut vm) = cairo_runner_result.unwrap();

        // relocate memory so we can dump it to file
        assert!(cairo_runner.relocate(&mut vm).is_ok());
        assert!(vm.trace.is_some());
        assert!(cairo_runner.relocated_trace.is_some());

        // write cairo_rs vm trace file
        assert!(
            write_binary_trace(&cairo_runner.relocated_trace.unwrap(), cairo_rs_trace_path).is_ok()
        );

        // compare that the original cairo vm trace file and cairo_rs vm trace files are equal
        assert!(compare_files(cairo_rs_trace_path, expected_trace_path).is_ok());
    }

    #[test]
    fn write_binary_memory_file() {
        let program_path = Path::new("cairo_programs/struct.json");
        let expected_memory_path = Path::new("cairo_programs/trace_memory/cairo_memory_struct");
        let cairo_rs_memory_path = Path::new("cairo_programs/trace_memory/struct_cairo_rs.memory");

        // run test program until the end
        let hint_processor = BuiltinHintProcessor::new_empty();
        let cairo_runner_result = run_test_program(program_path, &hint_processor);
        let (mut cairo_runner, mut vm) = cairo_runner_result.unwrap();

        // relocate memory so we can dump it to file
        assert!(cairo_runner.relocate(&mut vm).is_ok());

        // write cairo_rs vm memory file
        assert!(write_binary_memory(&cairo_runner.relocated_memory, cairo_rs_memory_path).is_ok());

        // compare that the original cairo vm memory file and cairo_rs vm memory files are equal
        assert!(compare_files(cairo_rs_memory_path, expected_memory_path).is_ok());
    }

    #[test]
    fn run_with_no_trace() {
        let program_path = Path::new("cairo_programs/struct.json");
        let program = Program::new(program_path, "main").unwrap();
        let hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = CairoRunner::new(&program).unwrap();
        let mut vm = vm!();
        let end = cairo_runner.initialize(&mut vm).unwrap();
        assert!(cairo_runner
            .run_until_pc(end, &mut vm, &hint_processor)
            .is_ok());
        assert!(vm.trace.is_none());
    }
}
