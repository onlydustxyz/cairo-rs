use crate::{
    serde::deserialize_program::BuiltinName,
    stdlib::{
        any::Any,
        collections::{HashMap, HashSet},
        ops::{Add, Sub},
        prelude::*,
    },
};

use crate::{
    hint_processor::hint_processor_definition::{HintProcessor, HintReference},
    math_utils::safe_div_usize,
    serde::deserialize_program::OffsetValue,
    types::{
        errors::{math_errors::MathError, program_errors::ProgramError},
        exec_scope::ExecutionScopes,
        instance_definitions::{
            bitwise_instance_def::BitwiseInstanceDef, ec_op_instance_def::EcOpInstanceDef,
            ecdsa_instance_def::EcdsaInstanceDef,
        },
        instruction::Register,
        layout::CairoLayout,
        program::Program,
        relocatable::{relocate_address, relocate_value, MaybeRelocatable, Relocatable},
    },
    utils::is_subsequence,
    vm::{
        errors::{
            cairo_run_errors::CairoRunError,
            memory_errors::{InsufficientAllocatedCellsError, MemoryError},
            runner_errors::RunnerError,
            trace_errors::TraceError,
            vm_errors::VirtualMachineError,
            vm_exception::VmException,
        },
        security::verify_secure_runner,
        trace::get_perm_range_check_limits,
        {
            runners::builtin_runner::{
                BitwiseBuiltinRunner, BuiltinRunner, EcOpBuiltinRunner, HashBuiltinRunner,
                OutputBuiltinRunner, RangeCheckBuiltinRunner, SignatureBuiltinRunner,
            },
            trace::trace_entry::{relocate_trace_register, RelocatedTraceEntry},
            vm_core::VirtualMachine,
        },
    },
};
use felt::Felt;
use num_integer::div_rem;
use num_traits::Zero;

use super::builtin_runner::KeccakBuiltinRunner;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CairoArg {
    Single(MaybeRelocatable),
    Array(Vec<MaybeRelocatable>),
    Composed(Vec<CairoArg>),
}

impl From<MaybeRelocatable> for CairoArg {
    fn from(other: MaybeRelocatable) -> Self {
        CairoArg::Single(other)
    }
}

impl From<Vec<MaybeRelocatable>> for CairoArg {
    fn from(other: Vec<MaybeRelocatable>) -> Self {
        CairoArg::Array(other)
    }
}

#[derive(Debug)]
pub struct CairoRunner {
    pub(crate) program: Program,
    layout: CairoLayout,
    final_pc: Option<Relocatable>,
    pub(crate) program_base: Option<Relocatable>,
    execution_base: Option<Relocatable>,
    initial_ap: Option<Relocatable>,
    initial_fp: Option<Relocatable>,
    initial_pc: Option<Relocatable>,
    run_ended: bool,
    segments_finalized: bool,
    execution_public_memory: Option<Vec<usize>>,
    proof_mode: bool,
    pub original_steps: Option<usize>,
    pub relocated_memory: Vec<Option<Felt>>,
    pub relocated_trace: Option<Vec<RelocatedTraceEntry>>,
    pub exec_scopes: ExecutionScopes,
}

impl CairoRunner {
    pub fn new(
        program: &Program,
        layout: &str,
        proof_mode: bool,
    ) -> Result<CairoRunner, RunnerError> {
        let cairo_layout = match layout {
            "plain" => CairoLayout::plain_instance(),
            "small" => CairoLayout::small_instance(),
            "dex" => CairoLayout::dex_instance(),
            "perpetual_with_bitwise" => CairoLayout::perpetual_with_bitwise_instance(),
            "bitwise" => CairoLayout::bitwise_instance(),
            "all" => CairoLayout::all_instance(),
            name => return Err(RunnerError::InvalidLayoutName(name.to_string())),
        };
        Ok(CairoRunner {
            program: program.clone(),
            layout: cairo_layout,
            final_pc: None,
            program_base: None,
            execution_base: None,
            initial_ap: None,
            initial_fp: None,
            initial_pc: None,
            run_ended: false,
            segments_finalized: false,
            proof_mode,
            original_steps: None,
            relocated_memory: Vec::new(),
            relocated_trace: None,
            exec_scopes: ExecutionScopes::new(),
            execution_public_memory: if proof_mode { Some(Vec::new()) } else { None },
        })
    }

    pub fn initialize(&mut self, vm: &mut VirtualMachine) -> Result<Relocatable, RunnerError> {
        self.initialize_builtins(vm)?;
        self.initialize_segments(vm, None);
        let end = self.initialize_main_entrypoint(vm)?;
        self.initialize_vm(vm)?;
        Ok(end)
    }

    pub fn initialize_builtins(&self, vm: &mut VirtualMachine) -> Result<(), RunnerError> {
        let builtin_ordered_list = vec![
            BuiltinName::output,
            BuiltinName::pedersen,
            BuiltinName::range_check,
            BuiltinName::ecdsa,
            BuiltinName::bitwise,
            BuiltinName::ec_op,
            BuiltinName::keccak,
        ];
        if !is_subsequence(&self.program.builtins, &builtin_ordered_list) {
            return Err(RunnerError::DisorderedBuiltins);
        };
        let mut builtin_runners = Vec::<(BuiltinName, BuiltinRunner)>::new();

        if self.layout.builtins.output {
            let included = self.program.builtins.contains(&BuiltinName::output);
            if included || self.proof_mode {
                builtin_runners.push((
                    BuiltinName::output,
                    OutputBuiltinRunner::new(included).into(),
                ));
            }
        }

        if let Some(instance_def) = self.layout.builtins.pedersen.as_ref() {
            let included = self.program.builtins.contains(&BuiltinName::pedersen);
            if included || self.proof_mode {
                builtin_runners.push((
                    BuiltinName::pedersen,
                    HashBuiltinRunner::new(instance_def.ratio, included).into(),
                ));
            }
        }

        if let Some(instance_def) = self.layout.builtins.range_check.as_ref() {
            let included = self.program.builtins.contains(&BuiltinName::range_check);
            if included || self.proof_mode {
                builtin_runners.push((
                    BuiltinName::range_check,
                    RangeCheckBuiltinRunner::new(
                        instance_def.ratio,
                        instance_def.n_parts,
                        included,
                    )
                    .into(),
                ));
            }
        }

        if let Some(instance_def) = self.layout.builtins.ecdsa.as_ref() {
            let included = self.program.builtins.contains(&BuiltinName::ecdsa);
            if included || self.proof_mode {
                builtin_runners.push((
                    BuiltinName::ecdsa,
                    SignatureBuiltinRunner::new(instance_def, included).into(),
                ));
            }
        }

        if let Some(instance_def) = self.layout.builtins.bitwise.as_ref() {
            let included = self.program.builtins.contains(&BuiltinName::bitwise);
            if included || self.proof_mode {
                builtin_runners.push((
                    BuiltinName::bitwise,
                    BitwiseBuiltinRunner::new(instance_def, included).into(),
                ));
            }
        }

        if let Some(instance_def) = self.layout.builtins.ec_op.as_ref() {
            let included = self.program.builtins.contains(&BuiltinName::ec_op);
            if included || self.proof_mode {
                builtin_runners.push((
                    BuiltinName::ec_op,
                    EcOpBuiltinRunner::new(instance_def, included).into(),
                ));
            }
        }

        if let Some(instance_def) = self.layout.builtins.keccak.as_ref() {
            let included = self.program.builtins.contains(&BuiltinName::keccak);
            if included || self.proof_mode {
                builtin_runners.push((
                    BuiltinName::keccak,
                    KeccakBuiltinRunner::new(instance_def, included).into(),
                ));
            }
        }

        let inserted_builtins = builtin_runners
            .iter()
            .map(|x| &x.0)
            .collect::<HashSet<&BuiltinName>>();
        let program_builtins: HashSet<&BuiltinName> = self.program.builtins.iter().collect();
        // Get the builtins that belong to the program but weren't inserted (those who dont belong to the instance)
        if !program_builtins.is_subset(&inserted_builtins) {
            return Err(RunnerError::NoBuiltinForInstance(
                program_builtins
                    .difference(&inserted_builtins)
                    .map(|x| **x)
                    .collect(),
                self.layout._name.clone(),
            ));
        }
        drop(inserted_builtins);

        vm.builtin_runners = builtin_runners;
        Ok(())
    }

    // Initialize all the builtins. Values used are the original one from the CairoFunctionRunner
    // Values extracted from here: https://github.com/starkware-libs/cairo-lang/blob/4fb83010ab77aa7ead0c9df4b0c05e030bc70b87/src/starkware/cairo/common/cairo_function_runner.py#L28
    fn initialize_all_builtins(&self, vm: &mut VirtualMachine) -> Result<(), RunnerError> {
        let starknet_preset_builtins = vec![
            BuiltinName::pedersen,
            BuiltinName::range_check,
            BuiltinName::output,
            BuiltinName::ecdsa,
            BuiltinName::bitwise,
            BuiltinName::ec_op,
            BuiltinName::keccak,
        ];

        fn initialize_builtin(name: BuiltinName, vm: &mut VirtualMachine) {
            match name {
                BuiltinName::pedersen => vm
                    .builtin_runners
                    .push((name, HashBuiltinRunner::new(32, true).into())),
                BuiltinName::range_check => vm
                    .builtin_runners
                    .push((name, RangeCheckBuiltinRunner::new(1, 8, true).into())),
                BuiltinName::output => vm
                    .builtin_runners
                    .push((name, OutputBuiltinRunner::new(true).into())),
                BuiltinName::ecdsa => vm.builtin_runners.push((
                    name,
                    SignatureBuiltinRunner::new(&EcdsaInstanceDef::new(1), true).into(),
                )),
                BuiltinName::bitwise => vm.builtin_runners.push((
                    name,
                    BitwiseBuiltinRunner::new(&BitwiseInstanceDef::new(1), true).into(),
                )),
                BuiltinName::ec_op => vm.builtin_runners.push((
                    name,
                    EcOpBuiltinRunner::new(&EcOpInstanceDef::new(1), true).into(),
                )),
                BuiltinName::keccak => vm.builtin_runners.push((
                    name,
                    EcOpBuiltinRunner::new(&EcOpInstanceDef::new(1), true).into(),
                )),
                _ => {}
            }
        }

        for builtin_name in &self.program.builtins {
            initialize_builtin(*builtin_name, vm);
        }
        for builtin_name in starknet_preset_builtins {
            if !self.program.builtins.contains(&builtin_name) {
                initialize_builtin(builtin_name, vm)
            }
        }
        Ok(())
    }

    ///Creates the necessary segments for the program, execution, and each builtin on the MemorySegmentManager and stores the first adress of each of this new segments as each owner's base
    pub fn initialize_segments(
        &mut self,
        vm: &mut VirtualMachine,
        program_base: Option<Relocatable>,
    ) {
        self.program_base = match program_base {
            Some(base) => Some(base),
            None => Some(vm.segments.add()),
        };
        self.execution_base = Some(vm.segments.add());
        for (_key, builtin_runner) in vm.builtin_runners.iter_mut() {
            builtin_runner.initialize_segments(&mut vm.segments);
        }
    }

    fn initialize_state(
        &mut self,
        vm: &mut VirtualMachine,
        entrypoint: usize,
        stack: Vec<MaybeRelocatable>,
    ) -> Result<(), RunnerError> {
        if let Some(prog_base) = self.program_base {
            let initial_pc = Relocatable {
                segment_index: prog_base.segment_index,
                offset: prog_base.offset + entrypoint,
            };
            self.initial_pc = Some(initial_pc);
            vm.segments
                .load_data(prog_base, &self.program.data)
                .map_err(RunnerError::MemoryInitializationError)?;

            // Mark all addresses from the program segment as accessed
            let base = self
                .program_base
                .unwrap_or_else(|| Relocatable::from((0, 0)));
            for i in 0..self.program.data.len() {
                vm.segments.memory.mark_as_accessed((base + i)?);
            }
        }
        if let Some(exec_base) = self.execution_base {
            vm.segments
                .load_data(exec_base, &stack)
                .map_err(RunnerError::MemoryInitializationError)?;
        } else {
            return Err(RunnerError::NoProgBase);
        }
        Ok(())
    }

    pub fn initialize_function_entrypoint(
        &mut self,
        vm: &mut VirtualMachine,
        entrypoint: usize,
        mut stack: Vec<MaybeRelocatable>,
        return_fp: MaybeRelocatable,
    ) -> Result<Relocatable, RunnerError> {
        let end = vm.segments.add();
        stack.append(&mut vec![
            return_fp,
            MaybeRelocatable::RelocatableValue(end),
        ]);
        if let Some(base) = &self.execution_base {
            self.initial_fp = Some(Relocatable {
                segment_index: base.segment_index,
                offset: base.offset + stack.len(),
            });
            self.initial_ap = self.initial_fp;
        } else {
            return Err(RunnerError::NoExecBase);
        }
        self.initialize_state(vm, entrypoint, stack)?;
        self.final_pc = Some(end);
        Ok(end)
    }

    ///Initializes state for running a program from the main() entrypoint.
    ///If self.proof_mode == True, the execution starts from the start label rather then the main() function.
    ///Returns the value of the program counter after returning from main.
    fn initialize_main_entrypoint(
        &mut self,
        vm: &mut VirtualMachine,
    ) -> Result<Relocatable, RunnerError> {
        let mut stack = Vec::new();
        for (_name, builtin_runner) in vm.builtin_runners.iter() {
            stack.append(&mut builtin_runner.initial_stack());
        }
        //Different process if proof_mode is enabled
        if self.proof_mode {
            // Add the dummy last fp and pc to the public memory, so that the verifier can enforce [fp - 2] = fp.
            let mut stack_prefix = vec![
                Into::<MaybeRelocatable>::into(
                    self.execution_base
                        .as_ref()
                        .ok_or(RunnerError::NoExecBase)?
                        + 2,
                ),
                MaybeRelocatable::from(Felt::zero()),
            ];
            stack_prefix.extend(stack);
            self.execution_public_memory = Some(Vec::from_iter(0..stack_prefix.len()));
            self.initialize_state(
                vm,
                self.program.start.ok_or(RunnerError::NoProgramStart)?,
                stack_prefix,
            )?;
            self.initial_fp = Some(
                self.execution_base
                    .as_ref()
                    .ok_or(RunnerError::NoExecBase)?
                    + 2,
            );
            self.initial_ap = self.initial_fp;
            return Ok(self.program_base.as_ref().ok_or(RunnerError::NoProgBase)?
                + self.program.end.ok_or(RunnerError::NoProgramEnd)?);
        }
        let return_fp = vm.segments.add();
        if let Some(main) = &self.program.main {
            let main_clone = *main;
            Ok(self.initialize_function_entrypoint(
                vm,
                main_clone,
                stack,
                MaybeRelocatable::RelocatableValue(return_fp),
            )?)
        } else {
            Err(RunnerError::MissingMain)
        }
    }

    pub fn initialize_vm(&mut self, vm: &mut VirtualMachine) -> Result<(), RunnerError> {
        vm.run_context.pc = *self.initial_pc.as_ref().ok_or(RunnerError::NoPC)?;
        vm.run_context.ap = self.initial_ap.as_ref().ok_or(RunnerError::NoAP)?.offset;
        vm.run_context.fp = self.initial_fp.as_ref().ok_or(RunnerError::NoFP)?.offset;
        for (_, builtin) in vm.builtin_runners.iter() {
            builtin.add_validation_rule(&mut vm.segments.memory);
        }

        vm.segments
            .memory
            .validate_existing_memory()
            .map_err(RunnerError::MemoryValidationError)
    }

    pub fn get_initial_fp(&self) -> Option<Relocatable> {
        self.initial_fp
    }

    pub fn get_reference_list(&self) -> HashMap<usize, HintReference> {
        let mut references = HashMap::<usize, HintReference>::new();

        for (i, reference) in self.program.reference_manager.references.iter().enumerate() {
            references.insert(
                i,
                HintReference {
                    offset1: reference.value_address.offset1.clone(),
                    offset2: reference.value_address.offset2.clone(),
                    dereference: reference.value_address.dereference,
                    // only store `ap` tracking data if the reference is referred to it
                    ap_tracking_data: match (
                        &reference.value_address.offset1,
                        &reference.value_address.offset2,
                    ) {
                        (OffsetValue::Reference(Register::AP, _, _), _)
                        | (_, OffsetValue::Reference(Register::AP, _, _)) => {
                            Some(reference.ap_tracking_data.clone())
                        }
                        _ => None,
                    },
                    cairo_type: Some(reference.value_address.value_type.clone()),
                },
            );
        }
        references
    }

    /// Gets the data used by the HintProcessor to execute each hint
    pub fn get_hint_data_dictionary(
        &self,
        references: &HashMap<usize, HintReference>,
        hint_executor: &mut dyn HintProcessor,
    ) -> Result<HashMap<usize, Vec<Box<dyn Any>>>, VirtualMachineError> {
        let mut hint_data_dictionary = HashMap::<usize, Vec<Box<dyn Any>>>::new();
        for (hint_index, hints) in self.program.hints.iter() {
            for hint in hints {
                let hint_data = hint_executor.compile_hint(
                    &hint.code,
                    &hint.flow_tracking_data.ap_tracking,
                    &hint.flow_tracking_data.reference_ids,
                    references,
                );
                hint_data_dictionary.entry(*hint_index).or_default().push(
                    hint_data
                        .map_err(|_| VirtualMachineError::CompileHintFail(hint.code.clone()))?,
                );
            }
        }
        Ok(hint_data_dictionary)
    }

    pub fn get_constants(&self) -> &HashMap<String, Felt> {
        &self.program.constants
    }

    pub fn get_program_builtins(&self) -> &Vec<BuiltinName> {
        &self.program.builtins
    }

    pub fn run_until_pc(
        &mut self,
        address: Relocatable,
        vm: &mut VirtualMachine,
        hint_processor: &mut dyn HintProcessor,
    ) -> Result<(), VirtualMachineError> {
        let references = self.get_reference_list();
        let hint_data_dictionary = self.get_hint_data_dictionary(&references, hint_processor)?;
        #[cfg(feature = "hooks")]
        vm.execute_before_first_step(self, &hint_data_dictionary)?;
        while vm.run_context.pc != address {
            vm.step(
                hint_processor,
                &mut self.exec_scopes,
                &hint_data_dictionary,
                &self.program.constants,
            )?;
        }
        Ok(())
    }

    /// Execute an exact number of steps on the program from the actual position.
    pub fn run_for_steps(
        &mut self,
        steps: usize,
        vm: &mut VirtualMachine,
        hint_processor: &mut dyn HintProcessor,
    ) -> Result<(), VirtualMachineError> {
        let references = self.get_reference_list();
        let hint_data_dictionary = self.get_hint_data_dictionary(&references, hint_processor)?;

        for remaining_steps in (1..=steps).rev() {
            if self.final_pc.as_ref() == Some(&vm.run_context.pc) {
                return Err(VirtualMachineError::EndOfProgram(remaining_steps));
            }

            vm.step(
                hint_processor,
                &mut self.exec_scopes,
                &hint_data_dictionary,
                &self.program.constants,
            )?;
        }

        Ok(())
    }

    /// Execute steps until a number of steps since the start of the program is reached.
    pub fn run_until_steps(
        &mut self,
        steps: usize,
        vm: &mut VirtualMachine,
        hint_processor: &mut dyn HintProcessor,
    ) -> Result<(), VirtualMachineError> {
        self.run_for_steps(steps.saturating_sub(vm.current_step), vm, hint_processor)
    }

    /// Execute steps until the step counter reaches a power of two.
    pub fn run_until_next_power_of_2(
        &mut self,
        vm: &mut VirtualMachine,
        hint_processor: &mut dyn HintProcessor,
    ) -> Result<(), VirtualMachineError> {
        self.run_until_steps(vm.current_step.next_power_of_two(), vm, hint_processor)
    }

    pub fn get_perm_range_check_limits(
        &self,
        vm: &VirtualMachine,
    ) -> Result<Option<(isize, isize)>, VirtualMachineError> {
        let limits = get_perm_range_check_limits(
            vm.trace.as_ref().ok_or(VirtualMachineError::TracerError(
                TraceError::TraceNotEnabled,
            ))?,
            &vm.segments.memory,
        )?;

        match limits {
            Some((mut rc_min, mut rc_max)) => {
                for (_, runner) in &vm.builtin_runners {
                    let (runner_min, runner_max) =
                        match runner.get_range_check_usage(&vm.segments.memory) {
                            Some(x) => x,
                            None => continue,
                        };

                    rc_min = rc_min.min(runner_min as isize);
                    rc_max = rc_max.max(runner_max as isize);
                }

                Ok(Some((rc_min, rc_max)))
            }
            None => Ok(None),
        }
    }

    /// Checks that there are enough trace cells to fill the entire range check
    /// range.
    pub fn check_range_check_usage(&self, vm: &VirtualMachine) -> Result<(), VirtualMachineError> {
        let (rc_min, rc_max) = match self.get_perm_range_check_limits(vm)? {
            Some(x) => x,
            None => return Ok(()),
        };

        let mut rc_units_used_by_builtins = 0;
        for (_, builtin_runner) in &vm.builtin_runners {
            rc_units_used_by_builtins += builtin_runner.get_used_perm_range_check_units(vm)?;
        }

        let unused_rc_units =
            (self.layout.rc_units as usize - 3) * vm.current_step - rc_units_used_by_builtins;
        if unused_rc_units < (rc_max - rc_min) as usize {
            return Err(MemoryError::InsufficientAllocatedCells(
                InsufficientAllocatedCellsError::RangeCheckUnits(
                    unused_rc_units,
                    (rc_max - rc_min) as usize,
                ),
            )
            .into());
        }

        Ok(())
    }

    /// Count the number of holes present in the segments.
    pub fn get_memory_holes(&self, vm: &VirtualMachine) -> Result<usize, MemoryError> {
        vm.segments.get_memory_holes()
    }

    /// Check if there are enough trace cells to fill the entire diluted checks.
    pub fn check_diluted_check_usage(
        &self,
        vm: &VirtualMachine,
    ) -> Result<(), VirtualMachineError> {
        let diluted_pool_instance = match &self.layout.diluted_pool_instance_def {
            Some(x) => x,
            None => return Ok(()),
        };

        let mut used_units_by_builtins = 0;
        for (_, builtin_runner) in &vm.builtin_runners {
            let used_units = builtin_runner.get_used_diluted_check_units(
                diluted_pool_instance.spacing,
                diluted_pool_instance.n_bits,
            );

            let multiplier = safe_div_usize(
                vm.current_step,
                builtin_runner.ratio().unwrap_or(1) as usize,
            )?;
            used_units_by_builtins += used_units * multiplier;
        }

        let diluted_units = diluted_pool_instance.units_per_step as usize * vm.current_step;
        let unused_diluted_units = diluted_units - used_units_by_builtins;

        let diluted_usage_upper_bound = 1usize << diluted_pool_instance.n_bits;
        if unused_diluted_units < diluted_usage_upper_bound {
            return Err(MemoryError::InsufficientAllocatedCells(
                InsufficientAllocatedCellsError::DilutedCells(
                    unused_diluted_units,
                    diluted_usage_upper_bound,
                ),
            )
            .into());
        }

        Ok(())
    }

    pub fn end_run(
        &mut self,
        disable_trace_padding: bool,
        disable_finalize_all: bool,
        vm: &mut VirtualMachine,
        hint_processor: &mut dyn HintProcessor,
    ) -> Result<(), VirtualMachineError> {
        if self.run_ended {
            return Err(RunnerError::EndRunCalledTwice.into());
        }

        vm.segments.memory.relocate_memory()?;
        vm.end_run(&self.exec_scopes)?;

        if disable_finalize_all {
            return Ok(());
        }

        vm.segments.compute_effective_sizes();
        if self.proof_mode && !disable_trace_padding {
            self.run_until_next_power_of_2(vm, hint_processor)?;
            loop {
                match self.check_used_cells(vm) {
                    Ok(_) => break,
                    Err(e) => match e {
                        VirtualMachineError::Memory(MemoryError::InsufficientAllocatedCells(_)) => {
                        }
                        e => return Err(e),
                    },
                }

                self.run_for_steps(1, vm, hint_processor)?;
                self.run_until_next_power_of_2(vm, hint_processor)?;
            }
        }

        self.run_ended = true;
        Ok(())
    }

    /// Relocates the VM's memory, turning bidimensional indexes into contiguous numbers, and values
    /// into Felts. Uses the relocation_table to asign each index a number according to the value
    /// on its segment number.
    fn relocate_memory(
        &mut self,
        vm: &mut VirtualMachine,
        relocation_table: &Vec<usize>,
    ) -> Result<(), MemoryError> {
        if !(self.relocated_memory.is_empty()) {
            return Err(MemoryError::Relocation);
        }
        //Relocated addresses start at 1
        self.relocated_memory.push(None);
        for (index, segment) in vm.segments.memory.data.iter().enumerate() {
            for (seg_offset, cell) in segment.iter().enumerate() {
                match cell {
                    Some(cell) => {
                        let relocated_addr = relocate_address(
                            Relocatable::from((index as isize, seg_offset)),
                            relocation_table,
                        )?;
                        let value = relocate_value(cell.get_value().clone(), relocation_table)?;
                        if self.relocated_memory.len() <= relocated_addr {
                            self.relocated_memory.resize(relocated_addr + 1, None);
                        }
                        self.relocated_memory[relocated_addr] = Some(value);
                    }
                    None => self.relocated_memory.push(None),
                }
            }
        }
        Ok(())
    }

    ///Relocates the VM's trace, turning relocatable registers to numbered ones
    fn relocate_trace(
        &mut self,
        vm: &mut VirtualMachine,
        relocation_table: &Vec<usize>,
    ) -> Result<(), TraceError> {
        if self.relocated_trace.is_some() {
            return Err(TraceError::AlreadyRelocated);
        }

        let trace = vm.trace.as_ref().ok_or(TraceError::TraceNotEnabled)?.iter();
        let mut relocated_trace = Vec::<RelocatedTraceEntry>::with_capacity(trace.len());
        for entry in trace {
            relocated_trace.push(RelocatedTraceEntry {
                pc: relocate_trace_register(entry.pc, relocation_table)?,
                ap: relocate_trace_register(entry.ap, relocation_table)?,
                fp: relocate_trace_register(entry.fp, relocation_table)?,
            })
        }
        self.relocated_trace = Some(relocated_trace);
        Ok(())
    }

    pub fn relocate(&mut self, vm: &mut VirtualMachine) -> Result<(), TraceError> {
        vm.segments.compute_effective_sizes();
        // relocate_segments can fail if compute_effective_sizes is not called before.
        // The expect should be unreachable.
        let relocation_table = vm
            .segments
            .relocate_segments()
            .expect("compute_effective_sizes called but relocate_memory still returned error");
        if let Err(memory_error) = self.relocate_memory(vm, &relocation_table) {
            return Err(TraceError::MemoryError(memory_error));
        }
        if vm.trace.is_some() {
            self.relocate_trace(vm, &relocation_table)?;
        }
        Ok(())
    }

    // Returns a map from builtin base's segment index to stop_ptr offset
    // Aka the builtin's segment number and its maximum offset
    pub fn get_builtin_segments_info(
        &self,
        vm: &VirtualMachine,
    ) -> Result<Vec<(usize, usize)>, RunnerError> {
        let mut builtin_segment_info = Vec::new();

        for (_, builtin) in &vm.builtin_runners {
            let (index, stop_ptr) = builtin.get_memory_segment_addresses();

            builtin_segment_info.push((
                index,
                stop_ptr.ok_or(RunnerError::NoStopPointer(builtin.name()))?,
            ));
        }

        Ok(builtin_segment_info)
    }

    pub fn get_execution_resources(
        &self,
        vm: &VirtualMachine,
    ) -> Result<ExecutionResources, TraceError> {
        let n_steps = match self.original_steps {
            Some(x) => x,
            None => vm.trace.as_ref().map(|x| x.len()).unwrap_or(0),
        };
        let n_memory_holes = self.get_memory_holes(vm)?;

        let mut builtin_instance_counter = HashMap::new();
        for (builtin_name, builtin_runner) in &vm.builtin_runners {
            builtin_instance_counter.insert(
                *builtin_name,
                builtin_runner.get_used_instances(&vm.segments)?,
            );
        }

        Ok(ExecutionResources {
            n_steps,
            n_memory_holes,
            builtin_instance_counter,
        })
    }

    // Finalizes the segments.
    //     Note:
    //     1.  end_run() must precede a call to this method.
    //     2.  Call read_return_values() *before* finalize_segments(), otherwise the return values
    //         will not be included in the public memory.
    pub fn finalize_segments(&mut self, vm: &mut VirtualMachine) -> Result<(), RunnerError> {
        if self.segments_finalized {
            return Ok(());
        }
        if !self.run_ended {
            return Err(RunnerError::FinalizeNoEndRun);
        }
        let size = self.program.data.len();
        let mut public_memory = Vec::with_capacity(size);
        for i in 0..size {
            public_memory.push((i, 0_usize))
        }
        vm.segments.finalize(
            Some(size),
            self.program_base
                .as_ref()
                .ok_or(RunnerError::NoProgBase)?
                .segment_index as usize,
            Some(&public_memory),
        );
        let mut public_memory = Vec::with_capacity(size);
        let exec_base = self
            .execution_base
            .as_ref()
            .ok_or(RunnerError::NoExecBase)?;
        for elem in self
            .execution_public_memory
            .as_ref()
            .ok_or(RunnerError::FinalizeSegmentsNoProofMode)?
            .iter()
        {
            public_memory.push((elem + exec_base.offset, 0))
        }
        vm.segments
            .finalize(None, exec_base.segment_index as usize, Some(&public_memory));
        for (_, builtin_runner) in vm.builtin_runners.iter() {
            let (_, size) = builtin_runner
                .get_used_cells_and_allocated_size(vm)
                .map_err(RunnerError::FinalizeSegements)?;
            vm.segments
                .finalize(Some(size), builtin_runner.base(), None)
        }
        self.segments_finalized = true;
        Ok(())
    }

    pub fn run_from_entrypoint(
        &mut self,
        entrypoint: usize,
        args: &[&CairoArg],
        verify_secure: bool,
        vm: &mut VirtualMachine,
        hint_processor: &mut dyn HintProcessor,
    ) -> Result<(), CairoRunError> {
        let stack = args
            .iter()
            .map(|arg| vm.segments.gen_cairo_arg(arg))
            .collect::<Result<Vec<MaybeRelocatable>, VirtualMachineError>>()?;
        let return_fp = MaybeRelocatable::from(0);
        let end = self.initialize_function_entrypoint(vm, entrypoint, stack, return_fp)?;

        self.initialize_vm(vm)?;

        self.run_until_pc(end, vm, hint_processor)
            .map_err(|err| VmException::from_vm_error(self, vm, err))?;
        self.end_run(true, false, vm, hint_processor)?;

        if verify_secure {
            verify_secure_runner(self, false, vm)?;
        }

        Ok(())
    }

    // Returns Ok(()) if there are enough allocated cells for the builtins.
    // If not, the number of steps should be increased or a different layout should be used.
    pub fn check_used_cells(&self, vm: &VirtualMachine) -> Result<(), VirtualMachineError> {
        vm.builtin_runners
            .iter()
            .map(|(_builtin_runner_name, builtin_runner)| {
                builtin_runner.get_used_cells_and_allocated_size(vm)
            })
            .collect::<Result<Vec<(usize, usize)>, MemoryError>>()?;
        self.check_range_check_usage(vm)?;
        self.check_memory_usage(vm)?;
        self.check_diluted_check_usage(vm)?;
        Ok(())
    }

    // Checks that there are enough trace cells to fill the entire memory range.
    pub fn check_memory_usage(&self, vm: &VirtualMachine) -> Result<(), VirtualMachineError> {
        let instance = &self.layout;

        let builtins_memory_units: usize = vm
            .builtin_runners
            .iter()
            .map(|(_builtin_runner_name, builtin_runner)| {
                builtin_runner.get_allocated_memory_units(vm)
            })
            .collect::<Result<Vec<usize>, MemoryError>>()?
            .iter()
            .sum();

        let builtins_memory_units = builtins_memory_units as u32;

        let vm_current_step_u32 = vm.current_step as u32;

        // Out of the memory units available per step, a fraction is used for public memory, and
        // four are used for the instruction.
        let total_memory_units = instance._memory_units_per_step * vm_current_step_u32;
        let (public_memory_units, rem) =
            div_rem(total_memory_units, instance._public_memory_fraction);
        if rem != 0 {
            return Err(MathError::SafeDivFailU32(
                total_memory_units,
                instance._public_memory_fraction,
            )
            .into());
        }

        let instruction_memory_units = 4 * vm_current_step_u32;

        let unused_memory_units = total_memory_units
            - (public_memory_units + instruction_memory_units + builtins_memory_units);
        let memory_address_holes = self.get_memory_holes(vm)?;
        if unused_memory_units < memory_address_holes as u32 {
            Err(MemoryError::InsufficientAllocatedCells(
                InsufficientAllocatedCellsError::MemoryAddresses(
                    unused_memory_units,
                    memory_address_holes,
                ),
            ))?
        }
        Ok(())
    }

    pub fn initialize_function_runner(
        &mut self,
        vm: &mut VirtualMachine,
    ) -> Result<(), RunnerError> {
        self.initialize_all_builtins(vm)?;
        self.initialize_segments(vm, self.program_base);
        Ok(())
    }

    /// Overrides the previous entrypoint with a custom one, or "main" if none
    /// is specified.
    pub fn set_entrypoint(&mut self, new_entrypoint: Option<&str>) -> Result<(), ProgramError> {
        let new_entrypoint = new_entrypoint.unwrap_or("main");
        self.program.main = Some(
            self.program
                .identifiers
                .get(&format!("__main__.{new_entrypoint}"))
                .and_then(|x| x.pc)
                .ok_or_else(|| ProgramError::EntrypointNotFound(new_entrypoint.to_string()))?,
        );

        Ok(())
    }

    pub fn read_return_values(&mut self, vm: &mut VirtualMachine) -> Result<(), RunnerError> {
        if !self.run_ended {
            return Err(RunnerError::ReadReturnValuesNoEndRun);
        }
        let mut pointer = vm.get_ap();
        for (_, builtin_runner) in vm.builtin_runners.iter_mut().rev() {
            let new_pointer = builtin_runner.final_stack(&vm.segments, pointer)?;
            pointer = new_pointer;
        }
        if self.segments_finalized {
            return Err(RunnerError::FailedAddingReturnValues);
        }
        if self.proof_mode {
            let exec_base = *self
                .execution_base
                .as_ref()
                .ok_or(RunnerError::NoExecBase)?;
            let begin = pointer.offset - exec_base.offset;
            let ap = vm.get_ap();
            let end = ap.offset - exec_base.offset;
            self.execution_public_memory
                .as_mut()
                .ok_or(RunnerError::NoExecPublicMemory)?
                .extend(begin..end);
        }
        Ok(())
    }

    /// Add (or replace if already present) a custom hash builtin. Returns a Relocatable
    /// with the new builtin base as the segment index.
    pub fn add_additional_hash_builtin(&self, vm: &mut VirtualMachine) -> Relocatable {
        // Remove the custom hash runner if it was already present.
        vm.builtin_runners
            .retain(|(name, _)| name != &BuiltinName::hash_builtin);

        // Create, initialize and insert the new custom hash runner.
        let mut builtin: BuiltinRunner = HashBuiltinRunner::new(32, true).into();
        builtin.initialize_segments(&mut vm.segments);
        let segment_index = builtin.base() as isize;
        vm.builtin_runners
            .push((BuiltinName::hash_builtin, builtin));

        Relocatable {
            segment_index,
            offset: 0,
        }
    }

    // Iterates over the program builtins in reverse, calling BuiltinRunner::final_stack on each of them and returns the final pointer
    // This method is used by cairo_rs_py to replace starknet functionality
    pub fn get_builtins_final_stack(
        &self,
        vm: &mut VirtualMachine,
        stack_ptr: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        let mut stack_ptr = Relocatable::from(&stack_ptr);
        for (_, runner) in
            vm.builtin_runners
                .iter_mut()
                .rev()
                .filter(|(builtin_name, _builtin_runner)| {
                    self.get_program_builtins().contains(builtin_name)
                })
        {
            stack_ptr = runner.final_stack(&vm.segments, stack_ptr)?
        }
        Ok(stack_ptr)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SegmentInfo {
    pub index: isize,
    pub size: usize,
}

//* ----------------------
//*   ExecutionResources
//* ----------------------

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ExecutionResources {
    pub n_steps: usize,
    pub n_memory_holes: usize,
    pub builtin_instance_counter: HashMap<BuiltinName, usize>,
}

/// Returns a copy of the execution resources where all the builtins with a usage counter
/// of 0 are omitted.
impl ExecutionResources {
    pub fn filter_unused_builtins(&self) -> ExecutionResources {
        ExecutionResources {
            n_steps: self.n_steps,
            n_memory_holes: self.n_memory_holes,
            builtin_instance_counter: self
                .clone()
                .builtin_instance_counter
                .into_iter()
                .filter(|builtin| !builtin.1.is_zero())
                .collect(),
        }
    }
}

impl Add for ExecutionResources {
    type Output = ExecutionResources;

    fn add(self, rhs: ExecutionResources) -> ExecutionResources {
        let mut builtin_instance_counter_union: HashMap<BuiltinName, usize> = HashMap::new();

        self.builtin_instance_counter
            .keys()
            .filter(|k| rhs.builtin_instance_counter.contains_key(*k))
            .for_each(|k| {
                builtin_instance_counter_union.insert(
                    *k,
                    self.builtin_instance_counter.get(k).unwrap()
                        + rhs.builtin_instance_counter.get(k).unwrap(),
                );
            });

        ExecutionResources {
            n_steps: self.n_steps + rhs.n_steps,
            n_memory_holes: self.n_memory_holes + rhs.n_memory_holes,
            builtin_instance_counter: builtin_instance_counter_union,
        }
    }
}

impl Sub for ExecutionResources {
    type Output = ExecutionResources;

    fn sub(self, rhs: ExecutionResources) -> ExecutionResources {
        let mut builtin_instance_counter_union: HashMap<BuiltinName, usize> = HashMap::new();

        self.builtin_instance_counter
            .keys()
            .filter(|k| rhs.builtin_instance_counter.contains_key(*k))
            .for_each(|k| {
                builtin_instance_counter_union.insert(
                    *k,
                    self.builtin_instance_counter
                        .get(k)
                        .unwrap()
                        .saturating_sub(*rhs.builtin_instance_counter.get(k).unwrap()),
                );
            });

        ExecutionResources {
            n_steps: self.n_steps.saturating_sub(rhs.n_steps),
            n_memory_holes: self.n_memory_holes.saturating_sub(rhs.n_memory_holes),
            builtin_instance_counter: builtin_instance_counter_union,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::collections::{HashMap, HashSet};
    use crate::vm::vm_memory::memory::MemoryCell;
    use crate::vm::vm_memory::memory_segments::MemorySegmentManager;
    use crate::{
        hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
        relocatable,
        serde::deserialize_program::{Identifier, ReferenceManager},
        types::instance_definitions::bitwise_instance_def::BitwiseInstanceDef,
        utils::test_utils::*,
        vm::{trace::trace_entry::TraceEntry, vm_memory::memory::Memory},
    };
    use assert_matches::assert_matches;
    use felt::felt_str;
    use num_traits::One;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_memory_usage_ok_case() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::range_check, BuiltinName::output];
        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);

        assert_matches!(cairo_runner.check_memory_usage(&vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_memory_usage_err_case() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.builtin_runners = vec![{
            let mut builtin_runner: BuiltinRunner = OutputBuiltinRunner::new(true).into();
            builtin_runner.initialize_segments(&mut vm.segments);

            (BuiltinName::output, builtin_runner)
        }];
        vm.segments.segment_used_sizes = Some(vec![4, 12]);
        vm.segments.memory = memory![((0, 0), 0), ((0, 1), 1), ((0, 2), 1)];
        vm.segments.memory.mark_as_accessed((0, 0).into());
        assert_matches!(
            cairo_runner.check_memory_usage(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::InsufficientAllocatedCells(_)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_builtins_with_disordered_builtins() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::range_check, BuiltinName::output];
        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        assert!(cairo_runner.initialize_builtins(&mut vm).is_err());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn create_cairo_runner_with_ordered_but_missing_builtins() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output, BuiltinName::ecdsa];
        //We only check that the creation doesnt panic
        let _cairo_runner = cairo_runner!(program);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_segments_with_base() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        let program_base = Some(Relocatable {
            segment_index: 5,
            offset: 9,
        });
        add_segments!(vm, 6);
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, program_base);
        assert_eq!(
            cairo_runner.program_base,
            Some(Relocatable {
                segment_index: 5,
                offset: 9,
            })
        );
        assert_eq!(
            cairo_runner.execution_base,
            Some(Relocatable {
                segment_index: 6,
                offset: 0,
            })
        );
        assert_eq!(vm.builtin_runners[0].0, BuiltinName::output);
        assert_eq!(vm.builtin_runners[0].1.base(), 7);

        assert_eq!(vm.segments.num_segments(), 8);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_segments_no_base() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        assert_eq!(
            cairo_runner.program_base,
            Some(Relocatable {
                segment_index: 0,
                offset: 0
            })
        );
        assert_eq!(
            cairo_runner.execution_base,
            Some(Relocatable {
                segment_index: 1,
                offset: 0
            })
        );
        assert_eq!(vm.builtin_runners[0].0, BuiltinName::output);
        assert_eq!(vm.builtin_runners[0].1.base(), 2);

        assert_eq!(vm.segments.num_segments(), 3);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_state_empty_data_and_stack() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.program_base = Some(relocatable!(1, 0));
        cairo_runner.execution_base = Some(relocatable!(2, 0));
        let stack = Vec::new();
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_state(&mut vm, 1, stack).unwrap();
        assert_eq!(
            cairo_runner.initial_pc,
            Some(Relocatable {
                segment_index: 1,
                offset: 1
            })
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_state_some_data_empty_stack() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program!(
            builtins = vec![BuiltinName::output],
            data = vec_data!((4), (6)),
        );
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        for _ in 0..2 {
            vm.segments.add();
        }
        cairo_runner.program_base = Some(Relocatable {
            segment_index: 1,
            offset: 0,
        });
        cairo_runner.execution_base = Some(relocatable!(2, 0));
        let stack = Vec::new();
        cairo_runner.initialize_state(&mut vm, 1, stack).unwrap();
        check_memory!(vm.segments.memory, ((1, 0), 4), ((1, 1), 6));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_state_empty_data_some_stack() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        for _ in 0..3 {
            vm.segments.add();
        }
        cairo_runner.program_base = Some(relocatable!(1, 0));
        cairo_runner.execution_base = Some(relocatable!(2, 0));
        let stack = vec![mayberelocatable!(4), mayberelocatable!(6)];
        cairo_runner.initialize_state(&mut vm, 1, stack).unwrap();
        check_memory!(vm.segments.memory, ((2, 0), 4), ((2, 1), 6));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_state_no_program_base() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        for _ in 0..2 {
            vm.segments.add();
        }
        cairo_runner.execution_base = Some(Relocatable {
            segment_index: 2,
            offset: 0,
        });
        let stack = vec![
            MaybeRelocatable::from(Felt::new(4_i32)),
            MaybeRelocatable::from(Felt::new(6_i32)),
        ];
        assert!(cairo_runner.initialize_state(&mut vm, 1, stack).is_err());
    }

    #[test]
    #[should_panic]
    fn initialize_state_no_execution_base() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        for _ in 0..2 {
            vm.segments.add();
        }
        cairo_runner.program_base = Some(relocatable!(1, 0));
        let stack = vec![
            MaybeRelocatable::from(Felt::new(4_i32)),
            MaybeRelocatable::from(Felt::new(6_i32)),
        ];
        cairo_runner.initialize_state(&mut vm, 1, stack).unwrap();
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_function_entrypoint_empty_stack() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        for _ in 0..2 {
            vm.segments.add();
        }
        cairo_runner.program_base = Some(relocatable!(0, 0));
        cairo_runner.execution_base = Some(relocatable!(1, 0));
        let stack = Vec::new();
        let return_fp = MaybeRelocatable::from(Felt::new(9_i32));
        cairo_runner
            .initialize_function_entrypoint(&mut vm, 0, stack, return_fp)
            .unwrap();
        assert_eq!(cairo_runner.initial_fp, cairo_runner.initial_ap);
        assert_eq!(cairo_runner.initial_fp, Some(relocatable!(1, 2)));
        check_memory!(vm.segments.memory, ((1, 0), 9), ((1, 1), (2, 0)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_function_entrypoint_some_stack() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        for _ in 0..2 {
            vm.segments.add();
        }
        cairo_runner.program_base = Some(relocatable!(0, 0));
        cairo_runner.execution_base = Some(relocatable!(1, 0));
        let stack = vec![MaybeRelocatable::from(Felt::new(7_i32))];
        let return_fp = MaybeRelocatable::from(Felt::new(9_i32));
        cairo_runner
            .initialize_function_entrypoint(&mut vm, 1, stack, return_fp)
            .unwrap();
        assert_eq!(cairo_runner.initial_fp, cairo_runner.initial_ap);
        assert_eq!(cairo_runner.initial_fp, Some(relocatable!(1, 3)));
        check_memory!(
            vm.segments.memory,
            ((1, 0), 7),
            ((1, 1), 9),
            ((1, 2), (2, 0))
        );
    }

    #[test]
    #[should_panic]
    fn initialize_function_entrypoint_no_execution_base() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        let stack = vec![MaybeRelocatable::from(Felt::new(7_i32))];
        let return_fp = MaybeRelocatable::from(Felt::new(9_i32));
        cairo_runner
            .initialize_function_entrypoint(&mut vm, 1, stack, return_fp)
            .unwrap();
    }

    #[test]
    #[should_panic]
    fn initialize_main_entrypoint_no_main() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_main_entrypoint() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program!(main = Some(1),);
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.program_base = Some(relocatable!(0, 0));
        cairo_runner.execution_base = Some(relocatable!(0, 0));
        let return_pc = cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        assert_eq!(return_pc, Relocatable::from((1, 0)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_state_program_segment_accessed_addrs() {
        // This test checks that all addresses from the program segment are marked as accessed at VM state initialization.
        // The fibonacci program has 24 instructions, so there should be 24 accessed addresses,
        // from (0, 0) to (0, 23).
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/fibonacci.json"),
            Some("main"),
        )
        .unwrap();

        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner.initialize(&mut vm).unwrap();
        assert_eq!(
            vm.segments
                .memory
                .get_amount_of_accessed_addresses_for_segment(0),
            Some(24)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_vm_no_builtins() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program!(main = Some(1),);
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.program_base = Some(relocatable!(0, 0));
        cairo_runner.initial_pc = Some(relocatable!(0, 1));
        cairo_runner.initial_ap = Some(relocatable!(1, 2));
        cairo_runner.initial_fp = Some(relocatable!(1, 2));
        cairo_runner.initialize_vm(&mut vm).unwrap();
        assert_eq!(vm.run_context.pc, relocatable!(0, 1));
        assert_eq!(vm.run_context.ap, 2);
        assert_eq!(vm.run_context.fp, 2);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_vm_with_range_check_valid() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program!(builtins = vec![BuiltinName::range_check], main = Some(1),);
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initial_pc = Some(relocatable!(0, 1));
        cairo_runner.initial_ap = Some(relocatable!(1, 2));
        cairo_runner.initial_fp = Some(relocatable!(1, 2));
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        vm.segments = segments![((2, 0), 23), ((2, 1), 233)];
        assert_eq!(vm.builtin_runners[0].0, BuiltinName::range_check);
        assert_eq!(vm.builtin_runners[0].1.base(), 2);
        cairo_runner.initialize_vm(&mut vm).unwrap();
        assert!(vm
            .segments
            .memory
            .validated_addresses
            .contains(&Relocatable::from((2, 0))));
        assert!(vm
            .segments
            .memory
            .validated_addresses
            .contains(&Relocatable::from((2, 1))));
        assert_eq!(vm.segments.memory.validated_addresses.len(), 2);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_vm_with_range_check_invalid() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program!(builtins = vec![BuiltinName::range_check], main = Some(1),);
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initial_pc = Some(relocatable!(0, 1));
        cairo_runner.initial_ap = Some(relocatable!(1, 2));
        cairo_runner.initial_fp = Some(relocatable!(1, 2));
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        vm.segments = segments![((2, 1), 23), ((2, 4), (-1))];

        assert_eq!(
            cairo_runner.initialize_vm(&mut vm),
            Err(RunnerError::MemoryValidationError(
                MemoryError::RangeCheckFoundNonInt((2, 0).into())
            ))
        );
    }

    //Integration tests for initialization phase

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /* Program used:
    func myfunc(a: felt) -> (r: felt):
        let b = a * 2
        return(b)
    end

    func main():
        let a = 1
        let b = myfunc(a)
        return()
    end

    main = 3
    data = [5207990763031199744, 2, 2345108766317314046, 5189976364521848832, 1, 1226245742482522112, 3618502788666131213697322783095070105623107215331596699973092056135872020476, 2345108766317314046]
    */
    fn initialization_phase_no_builtins() {
        let program = program!(
            data = vec_data!(
                (5207990763031199744_u64),
                (2),
                (2345108766317314046_u64),
                (5189976364521848832_u64),
                (1),
                (1226245742482522112_u64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020476",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(3),
        );
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initialize_segments(&mut vm, None);
        cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();

        assert_eq!(cairo_runner.program_base, Some(relocatable!(0, 0)));
        assert_eq!(cairo_runner.execution_base, Some(relocatable!(1, 0)));
        assert_eq!(cairo_runner.final_pc, Some(relocatable!(3, 0)));

        //RunContext check
        //Registers
        assert_eq!(vm.run_context.pc, relocatable!(0, 3));
        assert_eq!(vm.run_context.ap, 2);
        assert_eq!(vm.run_context.fp, 2);
        //Memory
        check_memory!(
            vm.segments.memory,
            ((0, 0), 5207990763031199744_u64),
            ((0, 1), 2),
            ((0, 2), 2345108766317314046_u64),
            ((0, 3), 5189976364521848832_u64),
            ((0, 4), 1),
            ((0, 5), 1226245742482522112_u64),
            (
                (0, 6),
                (
                    "3618502788666131213697322783095070105623107215331596699973092056135872020476",
                    10
                )
            ),
            ((0, 7), 2345108766317314046_u64),
            ((1, 0), (2, 0)),
            ((1, 1), (3, 0))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins output

    from starkware.cairo.common.serialize import serialize_word

    func main{output_ptr: felt*}():
        let a = 1
        serialize_word(a)
        return()
    end

    main = 4
    data = [4612671182993129469, 5198983563776393216, 1, 2345108766317314046, 5191102247248822272, 5189976364521848832, 1, 1226245742482522112, 3618502788666131213697322783095070105623107215331596699973092056135872020474, 2345108766317314046]
    */
    fn initialization_phase_output_builtin() {
        let program = program!(
            builtins = vec![BuiltinName::output],
            data = vec_data!(
                (4612671182993129469_u64),
                (5198983563776393216_u64),
                (1),
                (2345108766317314046_u64),
                (5191102247248822272_u64),
                (5189976364521848832_u64),
                (1),
                (1226245742482522112_u64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020474",
                    10
                )),
                (2345108766317314046_u64)
            ),
            main = Some(4),
        );
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();

        assert_eq!(cairo_runner.program_base, Some(relocatable!(0, 0)));
        assert_eq!(cairo_runner.execution_base, Some(relocatable!(1, 0)));
        assert_eq!(cairo_runner.final_pc, Some(relocatable!(4, 0)));

        //RunContext check
        //Registers
        assert_eq!(vm.run_context.pc, relocatable!(0, 4));
        assert_eq!(vm.run_context.ap, 3);
        assert_eq!(vm.run_context.fp, 3);
        //Memory
        check_memory!(
            vm.segments.memory,
            ((0, 0), 4612671182993129469_u64),
            ((0, 1), 5198983563776393216_u64),
            ((0, 2), 1),
            ((0, 3), 2345108766317314046_u64),
            ((0, 4), 5191102247248822272_u64),
            ((0, 5), 5189976364521848832_u64),
            ((0, 6), 1),
            ((0, 7), 1226245742482522112_u64),
            (
                (0, 8),
                (
                    "3618502788666131213697322783095070105623107215331596699973092056135872020474",
                    10
                )
            ),
            ((0, 9), 2345108766317314046_u64),
            ((1, 0), (2, 0)),
            ((1, 1), (3, 0)),
            ((1, 2), (4, 0))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins range_check

    func check_range{range_check_ptr}(num):

        # Check that 0 <= num < 2**64.
        [range_check_ptr] = num
        assert [range_check_ptr + 1] = 2 ** 64 - 1 - num
        let range_check_ptr = range_check_ptr + 2
        return()
    end

    func main{range_check_ptr}():
        check_range(7)
        return()
    end

    main = 8
    data = [4612671182993129469, 5189976364521848832, 18446744073709551615, 5199546496550207487, 4612389712311386111, 5198983563776393216, 2, 2345108766317314046, 5191102247248822272, 5189976364521848832, 7, 1226245742482522112, 3618502788666131213697322783095070105623107215331596699973092056135872020470, 2345108766317314046]
    */
    fn initialization_phase_range_check_builtin() {
        let program = program!(
            builtins = vec![BuiltinName::range_check],
            data = vec_data!(
                (4612671182993129469_u64),
                (5189976364521848832_u64),
                (18446744073709551615_u128),
                (5199546496550207487_u64),
                (4612389712311386111_u64),
                (5198983563776393216_u64),
                (2),
                (2345108766317314046_u64),
                (5191102247248822272_u64),
                (5189976364521848832_u64),
                (7),
                (1226245742482522112_u64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020474",
                    10
                )),
                (2345108766317314046_u64)
            ),
            main = Some(8),
        );

        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();

        assert_eq!(cairo_runner.program_base, Some(relocatable!(0, 0)));
        assert_eq!(cairo_runner.execution_base, Some(relocatable!(1, 0)));
        assert_eq!(cairo_runner.final_pc, Some(relocatable!(4, 0)));

        //RunContext check
        //Registers
        assert_eq!(vm.run_context.pc, relocatable!(0, 8));
        assert_eq!(vm.run_context.ap, 3);
        assert_eq!(vm.run_context.fp, 3);
        //Memory
        check_memory!(
            vm.segments.memory,
            ((0, 0), 4612671182993129469_u64),
            ((0, 1), 5189976364521848832_u64),
            ((0, 2), 18446744073709551615_u128),
            ((0, 3), 5199546496550207487_u64),
            ((0, 4), 4612389712311386111_u64),
            ((0, 5), 5198983563776393216_u64),
            ((0, 6), 2),
            ((0, 7), 2345108766317314046_u64),
            ((0, 8), 5191102247248822272_u64),
            ((0, 9), 5189976364521848832_u64),
            ((0, 10), 7),
            ((0, 11), 1226245742482522112_u64),
            (
                (0, 12),
                (
                    "3618502788666131213697322783095070105623107215331596699973092056135872020474",
                    10
                )
            ),
            ((0, 13), 2345108766317314046_u64),
            ((1, 0), (2, 0)),
            ((1, 1), (3, 0)),
            ((1, 2), (4, 0))
        );
    }

    //Integration tests for initialization + execution phase

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    func myfunc(a: felt) -> (r: felt):
        let b = a * 2
        return(b)
    end

    func main():
        let a = 1
        let b = myfunc(a)
        return()
    end

    main = 3
    data = [5207990763031199744, 2, 2345108766317314046, 5189976364521848832, 1, 1226245742482522112, 3618502788666131213697322783095070105623107215331596699973092056135872020476, 2345108766317314046]
    */
    fn initialize_and_run_function_call() {
        //Initialization Phase
        let program = program!(
            data = vec_data!(
                (5207990763031199744_i64),
                (2),
                (2345108766317314046_i64),
                (5189976364521848832_i64),
                (1),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020476",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(3),
        );
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!(true);
        cairo_runner.initialize_segments(&mut vm, None);
        let end = cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        assert_eq!(end, Relocatable::from((3, 0)));
        cairo_runner.initialize_vm(&mut vm).unwrap();
        //Execution Phase
        assert_matches!(
            cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
            Ok(())
        );
        //Check final values against Python VM
        //Check final register values
        assert_eq!(vm.run_context.pc, Relocatable::from((3, 0)));

        assert_eq!(vm.run_context.ap, 6);

        assert_eq!(vm.run_context.fp, 0);

        //Check each TraceEntry in trace
        let trace = vm.trace.unwrap();
        assert_eq!(trace.len(), 5);
        trace_check!(
            trace,
            [
                ((0, 3), (1, 2), (1, 2)),
                ((0, 5), (1, 3), (1, 2)),
                ((0, 0), (1, 5), (1, 5)),
                ((0, 2), (1, 6), (1, 5)),
                ((0, 7), (1, 6), (1, 2))
            ]
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins range_check

    func check_range{range_check_ptr}(num):

        # Check that 0 <= num < 2**64.
        [range_check_ptr] = num
        assert [range_check_ptr + 1] = 2 ** 64 - 1 - num
        let range_check_ptr = range_check_ptr + 2
        return()
    end

    func main{range_check_ptr}():
        check_range(7)
        return()
    end

    main = 8
    data = [4612671182993129469, 5189976364521848832, 18446744073709551615, 5199546496550207487, 4612389712311386111, 5198983563776393216, 2, 2345108766317314046, 5191102247248822272, 5189976364521848832, 7, 1226245742482522112, 3618502788666131213697322783095070105623107215331596699973092056135872020470, 2345108766317314046]
    */
    fn initialize_and_run_range_check_builtin() {
        //Initialization Phase
        let program = program!(
            builtins = vec![BuiltinName::range_check],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!(true);
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        let end = cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();
        //Execution Phase
        assert_matches!(
            cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
            Ok(())
        );
        //Check final values against Python VM
        //Check final register values
        assert_eq!(vm.run_context.pc, Relocatable::from((4, 0)));

        assert_eq!(vm.run_context.ap, 10);

        assert_eq!(vm.run_context.fp, 0);

        //Check each TraceEntry in trace
        let trace = vm.trace.unwrap();
        assert_eq!(trace.len(), 10);
        trace_check!(
            trace,
            [
                ((0, 8), (1, 3), (1, 3)),
                ((0, 9), (1, 4), (1, 3)),
                ((0, 11), (1, 5), (1, 3)),
                ((0, 0), (1, 7), (1, 7)),
                ((0, 1), (1, 7), (1, 7)),
                ((0, 3), (1, 8), (1, 7)),
                ((0, 4), (1, 9), (1, 7)),
                ((0, 5), (1, 9), (1, 7)),
                ((0, 7), (1, 10), (1, 7)),
                ((0, 13), (1, 10), (1, 3))
            ]
        );
        //Check the range_check builtin segment
        assert_eq!(vm.builtin_runners[0].0, BuiltinName::range_check);
        assert_eq!(vm.builtin_runners[0].1.base(), 2);

        check_memory!(
            vm.segments.memory,
            ((2, 0), 7),
            ((2, 1), 18446744073709551608_i128)
        );
        assert!(vm
            .segments
            .memory
            .get(&MaybeRelocatable::from((2, 2)))
            .is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins output

    from starkware.cairo.common.serialize import serialize_word

    func main{output_ptr: felt*}():
        let a = 1
        serialize_word(a)
        let b = 17 * a
        serialize_word(b)
        return()
    end

    main = 4
    data = [
    4612671182993129469,
    5198983563776393216,
    1,
    2345108766317314046,
    5191102247248822272,
    5189976364521848832,
    1,
    1226245742482522112,
    3618502788666131213697322783095070105623107215331596699973092056135872020474,
    5189976364521848832,
    17,
    1226245742482522112,
    3618502788666131213697322783095070105623107215331596699973092056135872020470,
    2345108766317314046
    ]
    */
    fn initialize_and_run_output_builtin() {
        //Initialization Phase
        let program = program!(
            builtins = vec![BuiltinName::output],
            data = vec_data!(
                (4612671182993129469_i64),
                (5198983563776393216_i64),
                (1),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (1),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020474",
                    10
                )),
                (5189976364521848832_i64),
                (17),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(4),
        );
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!(true);
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        let end = cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();
        //Execution Phase
        assert_matches!(
            cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
            Ok(())
        );
        //Check final values against Python VM
        //Check final register values
        //todo
        assert_eq!(vm.run_context.pc, Relocatable::from((4, 0)));

        assert_eq!(vm.run_context.ap, 12);

        assert_eq!(vm.run_context.fp, 0);

        //Check each TraceEntry in trace
        let trace = vm.trace.unwrap();
        assert_eq!(trace.len(), 12);
        trace_check!(
            trace,
            [
                ((0, 4), (1, 3), (1, 3)),
                ((0, 5), (1, 4), (1, 3)),
                ((0, 7), (1, 5), (1, 3)),
                ((0, 0), (1, 7), (1, 7)),
                ((0, 1), (1, 7), (1, 7)),
                ((0, 3), (1, 8), (1, 7)),
                ((0, 9), (1, 8), (1, 3)),
                ((0, 11), (1, 9), (1, 3)),
                ((0, 0), (1, 11), (1, 11)),
                ((0, 1), (1, 11), (1, 11)),
                ((0, 3), (1, 12), (1, 11)),
                ((0, 13), (1, 12), (1, 3))
            ]
        );
        //Check that the output to be printed is correct
        assert_eq!(vm.builtin_runners[0].0, BuiltinName::output);
        assert_eq!(vm.builtin_runners[0].1.base(), 2);
        check_memory!(vm.segments.memory, ((2, 0), 1), ((2, 1), 17));
        assert!(vm
            .segments
            .memory
            .get(&MaybeRelocatable::from((2, 2)))
            .is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins output range_check

    from starkware.cairo.common.serialize import serialize_word

    func check_range{range_check_ptr}(num) -> (num : felt):

        # Check that 0 <= num < 2**64.
        [range_check_ptr] = num
        assert [range_check_ptr + 1] = 2 ** 64 - 1 - num
        let range_check_ptr = range_check_ptr + 2
        return(num)
    end

    func main{output_ptr: felt*, range_check_ptr: felt}():
        let num: felt = check_range(7)
        serialize_word(num)
        return()
    end

    main = 13
    data = [
    4612671182993129469,
    5198983563776393216,
    1,
    2345108766317314046,
    4612671182993129469,
    5189976364521848832,
    18446744073709551615,
    5199546496550207487,
    4612389712311386111,
    5198983563776393216,
    2,
    5191102247248822272,
    2345108766317314046,
    5191102247248822272,
    5189976364521848832,
    7,
    1226245742482522112,
    3618502788666131213697322783095070105623107215331596699973092056135872020469,
    5191102242953854976,
    5193354051357474816,
    1226245742482522112,
    3618502788666131213697322783095070105623107215331596699973092056135872020461,
    5193354029882638336,
    2345108766317314046]
    */
    fn initialize_and_run_output_range_check_builtin() {
        //Initialization Phase
        let program = program!(
            builtins = vec![BuiltinName::output, BuiltinName::range_check],
            data = vec_data!(
                (4612671182993129469_i64),
                (5198983563776393216_i64),
                (1),
                (2345108766317314046_i64),
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (5191102247248822272_i64),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020469",
                    10
                )),
                (5191102242953854976_i64),
                (5193354051357474816_i64),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020461",
                    10
                )),
                (5193354029882638336_i64),
                (2345108766317314046_i64)
            ),
            main = Some(13),
        );
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!(true);
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        let end = cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();
        //Execution Phase
        assert_matches!(
            cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
            Ok(())
        );
        //Check final values against Python VM
        //Check final register values
        assert_eq!(vm.run_context.pc, Relocatable::from((5, 0)));

        assert_eq!(vm.run_context.ap, 18);

        assert_eq!(vm.run_context.fp, 0);

        //Check each TraceEntry in trace
        let trace = vm.trace.unwrap();
        assert_eq!(trace.len(), 18);
        trace_check!(
            trace,
            [
                ((0, 13), (1, 4), (1, 4)),
                ((0, 14), (1, 5), (1, 4)),
                ((0, 16), (1, 6), (1, 4)),
                ((0, 4), (1, 8), (1, 8)),
                ((0, 5), (1, 8), (1, 8)),
                ((0, 7), (1, 9), (1, 8)),
                ((0, 8), (1, 10), (1, 8)),
                ((0, 9), (1, 10), (1, 8)),
                ((0, 11), (1, 11), (1, 8)),
                ((0, 12), (1, 12), (1, 8)),
                ((0, 18), (1, 12), (1, 4)),
                ((0, 19), (1, 13), (1, 4)),
                ((0, 20), (1, 14), (1, 4)),
                ((0, 0), (1, 16), (1, 16)),
                ((0, 1), (1, 16), (1, 16)),
                ((0, 3), (1, 17), (1, 16)),
                ((0, 22), (1, 17), (1, 4)),
                ((0, 23), (1, 18), (1, 4))
            ]
        );
        //Check the range_check builtin segment
        assert_eq!(vm.builtin_runners[1].0, BuiltinName::range_check);
        assert_eq!(vm.builtin_runners[1].1.base(), 3);

        check_memory!(
            vm.segments.memory,
            ((3, 0), 7),
            ((3, 1), 18446744073709551608_i128)
        );
        assert!(vm
            .segments
            .memory
            .get(&MaybeRelocatable::from((2, 2)))
            .is_none());

        //Check the output segment
        assert_eq!(vm.builtin_runners[0].0, BuiltinName::output);
        assert_eq!(vm.builtin_runners[0].1.base(), 2);

        check_memory!(vm.segments.memory, ((2, 0), 7));
        assert!(vm
            .segments
            .memory
            .get(&(MaybeRelocatable::from((2, 1))))
            .is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Memory from this test is taken from a cairo program execution
    Program used:
        func main():
        let a = 1
        [ap + 3] = 5
        return()

    end
    Final Memory:
    {RelocatableValue(segment_index=0, offset=0): 4613515612218425347,
     RelocatableValue(segment_index=0, offset=1): 5,
     RelocatableValue(segment_index=0, offset=2): 2345108766317314046,
     RelocatableValue(segment_index=1, offset=0): RelocatableValue(segment_index=2, offset=0),
     RelocatableValue(segment_index=1, offset=1): RelocatableValue(segment_index=3, offset=0),
     RelocatableValue(segment_index=1, offset=5): 5}
    Relocated Memory:
        1     4613515612218425347
        2     5
        3     2345108766317314046
        4     10
        5     10
        ⋮
        9     5
    */
    fn relocate_memory_with_gap() {
        let program = program!();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!(true);
        for _ in 0..4 {
            vm.segments.add();
        }
        // Memory initialization without macro
        vm.segments
            .memory
            .insert(
                &MaybeRelocatable::from((0, 0)),
                &MaybeRelocatable::from(Felt::new(4613515612218425347_i64)),
            )
            .unwrap();
        vm.segments
            .memory
            .insert(
                &MaybeRelocatable::from((0, 1)),
                &MaybeRelocatable::from(Felt::new(5)),
            )
            .unwrap();
        vm.segments
            .memory
            .insert(
                &MaybeRelocatable::from((0, 2)),
                &MaybeRelocatable::from(Felt::new(2345108766317314046_i64)),
            )
            .unwrap();
        vm.segments
            .memory
            .insert(
                &MaybeRelocatable::from((1, 0)),
                &MaybeRelocatable::from((2, 0)),
            )
            .unwrap();
        vm.segments
            .memory
            .insert(
                &MaybeRelocatable::from((1, 1)),
                &MaybeRelocatable::from((3, 0)),
            )
            .unwrap();
        vm.segments
            .memory
            .insert(
                &MaybeRelocatable::from((1, 5)),
                &MaybeRelocatable::from(Felt::new(5)),
            )
            .unwrap();
        vm.segments.compute_effective_sizes();
        let rel_table = vm
            .segments
            .relocate_segments()
            .expect("Couldn't relocate after compute effective sizes");
        assert_eq!(cairo_runner.relocate_memory(&mut vm, &rel_table), Ok(()));
        assert_eq!(cairo_runner.relocated_memory[0], None);
        assert_eq!(
            cairo_runner.relocated_memory[1],
            Some(Felt::new(4613515612218425347_i64))
        );
        assert_eq!(cairo_runner.relocated_memory[2], Some(Felt::new(5)));
        assert_eq!(
            cairo_runner.relocated_memory[3],
            Some(Felt::new(2345108766317314046_i64))
        );
        assert_eq!(cairo_runner.relocated_memory[4], Some(Felt::new(10)));
        assert_eq!(cairo_runner.relocated_memory[5], Some(Felt::new(10)));
        assert_eq!(cairo_runner.relocated_memory[6], None);
        assert_eq!(cairo_runner.relocated_memory[7], None);
        assert_eq!(cairo_runner.relocated_memory[8], None);
        assert_eq!(cairo_runner.relocated_memory[9], Some(Felt::new(5)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /* Program used:
    %builtins output

    from starkware.cairo.common.serialize import serialize_word

    func main{output_ptr: felt*}():
        let a = 1
        serialize_word(a)
        let b = 17 * a
        serialize_word(b)
        return()
    end
    Relocated Memory:
        1     4612671182993129469
        2     5198983563776393216
        3     1
        4     2345108766317314046
        5     5191102247248822272
        6     5189976364521848832
        7     1
        8     1226245742482522112
        9     -7
        10    5189976364521848832
        11    17
        12    1226245742482522112
        13    -11
        14    2345108766317314046
        15    27
        16    29
        17    29
        18    27
        19    1
        20    18
        21    10
        22    28
        23    17
        24    18
        25    14
        26    29
        27    1
        28    17
     */
    fn initialize_run_and_relocate_output_builtin() {
        let program = program!(
            builtins = vec![BuiltinName::output],
            data = vec_data!(
                (4612671182993129469_i64),
                (5198983563776393216_i64),
                (1),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (1),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020474",
                    10
                )),
                (5189976364521848832_i64),
                (17),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(4),
        );
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!(true);
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        let end = cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();
        assert_matches!(
            cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
            Ok(())
        );
        vm.segments.compute_effective_sizes();
        let rel_table = vm
            .segments
            .relocate_segments()
            .expect("Couldn't relocate after compute effective sizes");
        assert_eq!(cairo_runner.relocate_memory(&mut vm, &rel_table), Ok(()));
        assert_eq!(cairo_runner.relocated_memory[0], None);
        assert_eq!(
            cairo_runner.relocated_memory[1],
            Some(Felt::new(4612671182993129469_i64))
        );
        assert_eq!(
            cairo_runner.relocated_memory[2],
            Some(Felt::new(5198983563776393216_i64))
        );
        assert_eq!(cairo_runner.relocated_memory[3], Some(Felt::one()));
        assert_eq!(
            cairo_runner.relocated_memory[4],
            Some(Felt::new(2345108766317314046_i64))
        );
        assert_eq!(
            cairo_runner.relocated_memory[5],
            Some(Felt::new(5191102247248822272_i64))
        );
        assert_eq!(
            cairo_runner.relocated_memory[6],
            Some(Felt::new(5189976364521848832_i64))
        );
        assert_eq!(cairo_runner.relocated_memory[7], Some(Felt::one()));
        assert_eq!(
            cairo_runner.relocated_memory[8],
            Some(Felt::new(1226245742482522112_i64))
        );
        assert_eq!(
            cairo_runner.relocated_memory[9],
            Some(felt_str!(
                "3618502788666131213697322783095070105623107215331596699973092056135872020474"
            ))
        );
        assert_eq!(
            cairo_runner.relocated_memory[10],
            Some(Felt::new(5189976364521848832_i64))
        );
        assert_eq!(cairo_runner.relocated_memory[11], Some(Felt::new(17)));
        assert_eq!(
            cairo_runner.relocated_memory[12],
            Some(Felt::new(1226245742482522112_i64))
        );
        assert_eq!(
            cairo_runner.relocated_memory[13],
            Some(felt_str!(
                "3618502788666131213697322783095070105623107215331596699973092056135872020470"
            ))
        );
        assert_eq!(
            cairo_runner.relocated_memory[14],
            Some(Felt::new(2345108766317314046_i64))
        );
        assert_eq!(cairo_runner.relocated_memory[15], Some(Felt::new(27_i32)));
        assert_eq!(cairo_runner.relocated_memory[16], Some(Felt::new(29)));
        assert_eq!(cairo_runner.relocated_memory[17], Some(Felt::new(29)));
        assert_eq!(cairo_runner.relocated_memory[18], Some(Felt::new(27)));
        assert_eq!(cairo_runner.relocated_memory[19], Some(Felt::one()));
        assert_eq!(cairo_runner.relocated_memory[20], Some(Felt::new(18)));
        assert_eq!(cairo_runner.relocated_memory[21], Some(Felt::new(10)));
        assert_eq!(cairo_runner.relocated_memory[22], Some(Felt::new(28)));
        assert_eq!(cairo_runner.relocated_memory[23], Some(Felt::new(17)));
        assert_eq!(cairo_runner.relocated_memory[24], Some(Felt::new(18)));
        assert_eq!(cairo_runner.relocated_memory[25], Some(Felt::new(14)));
        assert_eq!(cairo_runner.relocated_memory[26], Some(Felt::new(29)));
        assert_eq!(cairo_runner.relocated_memory[27], Some(Felt::one()));
        assert_eq!(cairo_runner.relocated_memory[28], Some(Felt::new(17)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /* Program used:
    %builtins output

    from starkware.cairo.common.serialize import serialize_word

    func main{output_ptr: felt*}():
        let a = 1
        serialize_word(a)
        let b = 17 * a
        serialize_word(b)
        return()
    end

    Relocated Trace:
    [TraceEntry(pc=5, ap=18, fp=18),
     TraceEntry(pc=6, ap=19, fp=18),
     TraceEntry(pc=8, ap=20, fp=18),
     TraceEntry(pc=1, ap=22, fp=22),
     TraceEntry(pc=2, ap=22, fp=22),
     TraceEntry(pc=4, ap=23, fp=22),
     TraceEntry(pc=10, ap=23, fp=18),
    */
    fn relocate_trace_output_builtin() {
        let program = program!(
            builtins = vec![BuiltinName::output],
            data = vec_data!(
                (4612671182993129469_i64),
                (5198983563776393216_i64),
                (1),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (1),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020474",
                    10
                )),
                (5189976364521848832_i64),
                (17),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(4),
        );
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!(true);
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        let end = cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();
        assert_matches!(
            cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
            Ok(())
        );
        vm.segments.compute_effective_sizes();
        let rel_table = vm
            .segments
            .relocate_segments()
            .expect("Couldn't relocate after compute effective sizes");
        cairo_runner.relocate_trace(&mut vm, &rel_table).unwrap();
        let relocated_trace = cairo_runner.relocated_trace.unwrap();
        assert_eq!(relocated_trace.len(), 12);
        assert_eq!(
            relocated_trace[0],
            RelocatedTraceEntry {
                pc: 5,
                ap: 18,
                fp: 18
            }
        );
        assert_eq!(
            relocated_trace[1],
            RelocatedTraceEntry {
                pc: 6,
                ap: 19,
                fp: 18
            }
        );
        assert_eq!(
            relocated_trace[2],
            RelocatedTraceEntry {
                pc: 8,
                ap: 20,
                fp: 18
            }
        );
        assert_eq!(
            relocated_trace[3],
            RelocatedTraceEntry {
                pc: 1,
                ap: 22,
                fp: 22
            }
        );
        assert_eq!(
            relocated_trace[4],
            RelocatedTraceEntry {
                pc: 2,
                ap: 22,
                fp: 22
            }
        );
        assert_eq!(
            relocated_trace[5],
            RelocatedTraceEntry {
                pc: 4,
                ap: 23,
                fp: 22
            }
        );
        assert_eq!(
            relocated_trace[6],
            RelocatedTraceEntry {
                pc: 10,
                ap: 23,
                fp: 18
            }
        );
        assert_eq!(
            relocated_trace[7],
            RelocatedTraceEntry {
                pc: 12,
                ap: 24,
                fp: 18
            }
        );
        assert_eq!(
            relocated_trace[8],
            RelocatedTraceEntry {
                pc: 1,
                ap: 26,
                fp: 26
            }
        );
        assert_eq!(
            relocated_trace[9],
            RelocatedTraceEntry {
                pc: 2,
                ap: 26,
                fp: 26
            }
        );
        assert_eq!(
            relocated_trace[10],
            RelocatedTraceEntry {
                pc: 4,
                ap: 27,
                fp: 26
            }
        );
        assert_eq!(
            relocated_trace[11],
            RelocatedTraceEntry {
                pc: 14,
                ap: 27,
                fp: 18
            }
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_output_from_preset_memory() {
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        assert_eq!(vm.builtin_runners[0].0, BuiltinName::output);
        assert_eq!(vm.builtin_runners[0].1.base(), 2);

        vm.segments = segments![((2, 0), 1), ((2, 1), 2)];
        vm.segments.segment_used_sizes = Some(vec![0, 0, 2]);

        let mut output_buffer = String::new();
        vm.write_output(&mut output_buffer).unwrap();
        assert_eq!(&output_buffer, "1\n2\n");
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins output

    from starkware.cairo.common.serialize import serialize_word

    func main{output_ptr: felt*}():
        let a = 1
        serialize_word(a)
        return()
    end */
    fn get_output_from_program() {
        //Initialization Phase
        let program = program!(
            builtins = vec![BuiltinName::output],
            data = vec_data!(
                (4612671182993129469_i64),
                (5198983563776393216_i64),
                (1),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (1),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020474",
                    10
                )),
                (5189976364521848832_i64),
                (17),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(4),
        );
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        let end = cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();
        //Execution Phase
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        assert_matches!(
            cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
            Ok(())
        );

        let mut output_buffer = String::new();
        vm.write_output(&mut output_buffer).unwrap();
        assert_eq!(&output_buffer, "1\n17\n");
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins output

    func main{output_ptr: felt*}() {
        //Memory Gap + Relocatable value
        assert [output_ptr + 1] = cast(output_ptr, felt);
        let output_ptr = output_ptr + 2;
        return ();
    }*/
    fn write_output_from_program_gap_relocatable_output() {
        //Initialization Phase
        let program = program!(
            builtins = vec![BuiltinName::output],
            data = vec_data!(
                (4612671187288162301),
                (5198983563776458752),
                (2),
                (2345108766317314046)
            ),
            main = Some(0),
        );
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        let end = cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();
        //Execution Phase
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        assert_matches!(
            cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
            Ok(())
        );

        let mut output_buffer = String::new();
        vm.write_output(&mut output_buffer).unwrap();
        assert_eq!(&output_buffer, "<missing>\n2:0\n");
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_output_from_preset_memory_neg_output() {
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        assert_eq!(vm.builtin_runners[0].0, BuiltinName::output);
        assert_eq!(vm.builtin_runners[0].1.base(), 2);
        vm.segments = segments![(
            (2, 0),
            (
                "800000000000011000000000000000000000000000000000000000000000000",
                16
            )
        )];
        vm.segments.segment_used_sizes = Some(vec![0, 0, 1]);

        let mut output_buffer = String::new();
        vm.write_output(&mut output_buffer).unwrap();
        assert_eq!(&output_buffer, "-1\n");
    }

    /// Test that `get_output()` works when the `output` builtin is not the first one.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_output_unordered_builtins() {
        //Initialization Phase
        let program = program!(
            builtins = vec![BuiltinName::output, BuiltinName::bitwise],
            data = vec_data!(
                (4612671182993129469_i64),
                (5198983563776393216_i64),
                (1),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (1),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020474",
                    10
                )),
                (5189976364521848832_i64),
                (17),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(4),
        );

        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner
            .initialize_builtins(&mut vm)
            .expect("Couldn't initialize builtins.");

        // Swap the first and second builtins (first should be `output`).
        vm.builtin_runners.swap(0, 1);

        cairo_runner.initialize_segments(&mut vm, None);

        let end = cairo_runner
            .initialize_main_entrypoint(&mut vm)
            .expect("Couldn't initialize the main entrypoint.");
        cairo_runner
            .initialize_vm(&mut vm)
            .expect("Couldn't initialize the VM.");

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        assert_matches!(
            cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
            Ok(())
        );

        let mut output_buffer = String::new();
        vm.write_output(&mut output_buffer).unwrap();
        assert_eq!(&output_buffer, "1\n17\n");
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn insert_all_builtins_in_order() {
        let program = program![
            BuiltinName::output,
            BuiltinName::pedersen,
            BuiltinName::range_check,
            BuiltinName::bitwise,
            BuiltinName::ec_op
        ];
        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        assert_eq!(vm.builtin_runners[0].0, BuiltinName::output);
        assert_eq!(vm.builtin_runners[1].0, BuiltinName::pedersen);
        assert_eq!(vm.builtin_runners[2].0, BuiltinName::range_check);
        assert_eq!(vm.builtin_runners[3].0, BuiltinName::bitwise);
        assert_eq!(vm.builtin_runners[4].0, BuiltinName::ec_op);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins range_check

    func check_range{range_check_ptr}(num):
        # Check that 0 <= num < 2**64.
        [range_check_ptr] = num
        assert [range_check_ptr + 1] = 2 ** 64 - 1 - num
        let range_check_ptr = range_check_ptr + 2
        return()
    end

    func main{range_check_ptr}():
        check_range(7)
        return()
    end

    main = 8
    data = [4612671182993129469, 5189976364521848832, 18446744073709551615, 5199546496550207487, 4612389712311386111, 5198983563776393216, 2, 2345108766317314046, 5191102247248822272, 5189976364521848832, 7, 1226245742482522112, 3618502788666131213697322783095070105623107215331596699973092056135872020470, 2345108766317314046]
    */
    fn run_for_steps() {
        let program = program!(
            builtins = vec![BuiltinName::range_check],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(&program);

        let mut vm = vm!(true);
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);

        cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();

        // Full takes 10 steps.
        assert_matches!(
            cairo_runner.run_for_steps(8, &mut vm, &mut hint_processor),
            Ok(())
        );
        assert_matches!(
            cairo_runner.run_for_steps(8, &mut vm, &mut hint_processor),
            Err(VirtualMachineError::EndOfProgram(x)) if x == 8 - 2
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins range_check

    func check_range{range_check_ptr}(num):
        # Check that 0 <= num < 2**64.
        [range_check_ptr] = num
        assert [range_check_ptr + 1] = 2 ** 64 - 1 - num
        let range_check_ptr = range_check_ptr + 2
        return()
    end

    func main{range_check_ptr}():
        check_range(7)
        return()
    end

    main = 8
    data = [4612671182993129469, 5189976364521848832, 18446744073709551615, 5199546496550207487, 4612389712311386111, 5198983563776393216, 2, 2345108766317314046, 5191102247248822272, 5189976364521848832, 7, 1226245742482522112, 3618502788666131213697322783095070105623107215331596699973092056135872020470, 2345108766317314046]
    */
    fn run_until_steps() {
        let program = program!(
            builtins = vec![BuiltinName::range_check],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(&program);

        let mut vm = vm!(true);
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);

        cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();

        // Full takes 10 steps.
        assert_matches!(
            cairo_runner.run_until_steps(8, &mut vm, &mut hint_processor),
            Ok(())
        );
        assert_matches!(
            cairo_runner.run_until_steps(10, &mut vm, &mut hint_processor),
            Ok(())
        );
        assert_matches!(
            cairo_runner.run_until_steps(11, &mut vm, &mut hint_processor),
            Err(VirtualMachineError::EndOfProgram(1))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    /*Program used:
    %builtins range_check

    func check_range{range_check_ptr}(num):
        # Check that 0 <= num < 2**64.
        [range_check_ptr] = num
        assert [range_check_ptr + 1] = 2 ** 64 - 1 - num
        let range_check_ptr = range_check_ptr + 2
        return()
    end

    func main{range_check_ptr}():
        check_range(7)
        return()
    end

    main = 8
    data = [4612671182993129469, 5189976364521848832, 18446744073709551615, 5199546496550207487, 4612389712311386111, 5198983563776393216, 2, 2345108766317314046, 5191102247248822272, 5189976364521848832, 7, 1226245742482522112, 3618502788666131213697322783095070105623107215331596699973092056135872020470, 2345108766317314046]
    */
    /// Verify that run_until_next_power_2() executes steps until the current
    /// step reaches a power of two, or an error occurs.
    fn run_until_next_power_of_2() {
        let program = program!(
            builtins = vec![BuiltinName::range_check],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(&program);

        let mut vm = vm!(true);
        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);

        cairo_runner.initialize_main_entrypoint(&mut vm).unwrap();
        cairo_runner.initialize_vm(&mut vm).unwrap();

        // Full takes 10 steps.
        assert_matches!(
            cairo_runner.run_for_steps(1, &mut vm, &mut hint_processor),
            Ok(())
        );
        assert_matches!(
            cairo_runner.run_until_next_power_of_2(&mut vm, &mut hint_processor),
            Ok(())
        );
        assert_eq!(vm.current_step, 1);

        assert_matches!(
            cairo_runner.run_for_steps(1, &mut vm, &mut hint_processor),
            Ok(())
        );
        assert_matches!(
            cairo_runner.run_until_next_power_of_2(&mut vm, &mut hint_processor),
            Ok(())
        );
        assert_eq!(vm.current_step, 2);

        assert_matches!(
            cairo_runner.run_for_steps(1, &mut vm, &mut hint_processor),
            Ok(())
        );
        assert_matches!(
            cairo_runner.run_until_next_power_of_2(&mut vm, &mut hint_processor),
            Ok(())
        );
        assert_eq!(vm.current_step, 4);

        assert_matches!(
            cairo_runner.run_for_steps(1, &mut vm, &mut hint_processor),
            Ok(())
        );
        assert_matches!(
            cairo_runner.run_until_next_power_of_2(&mut vm, &mut hint_processor),
            Ok(())
        );
        assert_eq!(vm.current_step, 8);

        assert_matches!(
            cairo_runner.run_for_steps(1, &mut vm, &mut hint_processor),
            Ok(())
        );
        assert_matches!(
            cairo_runner.run_until_next_power_of_2(&mut vm, &mut hint_processor),
            Err(VirtualMachineError::EndOfProgram(6))
        );
        assert_eq!(vm.current_step, 10);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_constants() {
        let program_constants = HashMap::from([
            ("MAX".to_string(), Felt::new(300)),
            ("MIN".to_string(), Felt::new(20)),
        ]);
        let program = program!(constants = program_constants.clone(),);
        let cairo_runner = cairo_runner!(program);
        assert_eq!(cairo_runner.get_constants(), &program_constants);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_holes_missing_segment_used_sizes() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        // Add element into memory and mark it as accessed so that get_memory_holes tries to access a segment size
        vm.segments.memory = memory![((0, 0), 9)];
        vm.segments.memory.mark_as_accessed((0, 0).into());

        vm.builtin_runners = Vec::new();
        assert_eq!(
            cairo_runner.get_memory_holes(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_holes_empty() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        vm.builtin_runners = Vec::new();
        vm.segments.segment_used_sizes = Some(Vec::new());
        assert_eq!(cairo_runner.get_memory_holes(&vm), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_holes_empty_builtins() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.segments.memory = memory![((0, 0), 0), ((0, 2), 0)];
        vm.segments.memory.mark_as_accessed((0, 0).into());
        vm.segments.memory.mark_as_accessed((0, 2).into());
        vm.builtin_runners = Vec::new();
        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(cairo_runner.get_memory_holes(&vm), Ok(2));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_holes_empty_accesses() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        vm.builtin_runners = vec![{
            let mut builtin_runner: BuiltinRunner = OutputBuiltinRunner::new(true).into();
            builtin_runner.initialize_segments(&mut vm.segments);

            (BuiltinName::output, builtin_runner)
        }];
        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(cairo_runner.get_memory_holes(&vm), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_holes() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.segments.memory = memory![((1, 0), 0), ((1, 2), 2)];
        vm.segments.memory.mark_as_accessed((1, 0).into());
        vm.segments.memory.mark_as_accessed((1, 2).into());
        vm.builtin_runners = vec![{
            let mut builtin_runner: BuiltinRunner = OutputBuiltinRunner::new(true).into();
            builtin_runner.initialize_segments(&mut vm.segments);

            (BuiltinName::output, builtin_runner)
        }];
        vm.segments.segment_used_sizes = Some(vec![4, 4]);
        assert_eq!(cairo_runner.get_memory_holes(&vm), Ok(2));
    }

    /// Test that check_diluted_check_usage() works without a diluted pool
    /// instance.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_diluted_check_usage_without_pool_instance() {
        let program = program!();

        let mut cairo_runner = cairo_runner!(program);
        let vm = vm!();

        cairo_runner.layout.diluted_pool_instance_def = None;
        assert_matches!(cairo_runner.check_diluted_check_usage(&vm), Ok(()));
    }

    /// Test that check_diluted_check_usage() works without builtin runners.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_diluted_check_usage_without_builtin_runners() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        vm.current_step = 10000;
        vm.builtin_runners = vec![];
        assert_matches!(cairo_runner.check_diluted_check_usage(&vm), Ok(()));
    }

    /// Test that check_diluted_check_usage() fails when there aren't enough
    /// allocated units.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_diluted_check_usage_insufficient_allocated_cells() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        vm.current_step = 100;
        vm.builtin_runners = vec![];
        assert_matches!(
            cairo_runner.check_diluted_check_usage(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::InsufficientAllocatedCells(_)
            ))
        );
    }

    /// Test that check_diluted_check_usage() succeeds when all the conditions
    /// are met.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_diluted_check_usage() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        vm.current_step = 8192;
        vm.builtin_runners = vec![(
            BuiltinName::bitwise,
            BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true).into(),
        )];
        assert_matches!(cairo_runner.check_diluted_check_usage(&vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn end_run_run_already_finished() {
        let program = program!();

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner.run_ended = true;
        assert_matches!(
            cairo_runner.end_run(true, false, &mut vm, &mut hint_processor),
            Err(VirtualMachineError::RunnerError(
                RunnerError::EndRunCalledTwice
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn end_run() {
        let program = program!();

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        assert_matches!(
            cairo_runner.end_run(true, false, &mut vm, &mut hint_processor),
            Ok(())
        );

        cairo_runner.run_ended = false;
        cairo_runner.relocated_memory.clear();
        assert_matches!(
            cairo_runner.end_run(true, true, &mut vm, &mut hint_processor),
            Ok(())
        );
        assert!(!cairo_runner.run_ended);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn end_run_proof_mode_insufficient_allocated_cells() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/proof_programs/fibonacci.json"),
            Some("main"),
        )
        .unwrap();

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program, "all", true);
        let mut vm = vm!(true);

        let end = cairo_runner.initialize(&mut vm).unwrap();
        cairo_runner
            .run_until_pc(end, &mut vm, &mut hint_processor)
            .expect("Call to `CairoRunner::run_until_pc()` failed.");
        assert_matches!(
            cairo_runner.end_run(false, false, &mut vm, &mut hint_processor),
            Ok(())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtin_segments_info_empty() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let vm = vm!();

        assert_eq!(cairo_runner.get_builtin_segments_info(&vm), Ok(Vec::new()),);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtin_segments_info_base_not_finished() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        vm.builtin_runners = vec![(
            BuiltinName::output,
            BuiltinRunner::Output(OutputBuiltinRunner::new(true)),
        )];
        assert_eq!(
            cairo_runner.get_builtin_segments_info(&vm),
            Err(RunnerError::NoStopPointer(BuiltinName::output.name())),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_execution_resources_trace_not_enabled() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(
            cairo_runner.get_execution_resources(&vm),
            Ok(ExecutionResources {
                n_steps: 0,
                n_memory_holes: 0,
                builtin_instance_counter: HashMap::new(),
            }),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_execution_resources_empty_builtins() {
        let program = program!();

        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner.original_steps = Some(10);
        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(
            cairo_runner.get_execution_resources(&vm),
            Ok(ExecutionResources {
                n_steps: 10,
                n_memory_holes: 0,
                builtin_instance_counter: HashMap::new(),
            }),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_execution_resources() {
        let program = program!();

        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner.original_steps = Some(10);
        vm.segments.segment_used_sizes = Some(vec![4]);
        vm.builtin_runners = vec![{
            let mut builtin = OutputBuiltinRunner::new(true);
            builtin.initialize_segments(&mut vm.segments);

            (BuiltinName::output, BuiltinRunner::Output(builtin))
        }];
        assert_eq!(
            cairo_runner.get_execution_resources(&vm),
            Ok(ExecutionResources {
                n_steps: 10,
                n_memory_holes: 0,
                builtin_instance_counter: HashMap::from([(BuiltinName::output, 4)]),
            }),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_segments_run_not_ended() {
        let program = program!();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        assert_eq!(
            cairo_runner.finalize_segments(&mut vm),
            Err(RunnerError::FinalizeNoEndRun)
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_segments_run_ended_empty_no_prog_base() {
        let program = program!();
        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = true;
        let mut vm = vm!();
        assert_eq!(
            cairo_runner.finalize_segments(&mut vm),
            Err(RunnerError::NoProgBase)
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_segments_run_ended_empty_no_exec_base() {
        let program = program!();
        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.proof_mode = true;
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.run_ended = true;
        let mut vm = vm!();
        assert_eq!(
            cairo_runner.finalize_segments(&mut vm),
            Err(RunnerError::NoExecBase)
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_segments_run_ended_empty_noproof_mode() {
        let program = program!();
        let mut cairo_runner = cairo_runner!(program);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = true;
        let mut vm = vm!();
        assert_eq!(
            cairo_runner.finalize_segments(&mut vm),
            Err(RunnerError::FinalizeSegmentsNoProofMode)
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_segments_run_ended_emptyproof_mode() {
        let program = program!();
        let mut cairo_runner = cairo_runner!(program, "plain", true);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = true;
        let mut vm = vm!();
        assert_eq!(cairo_runner.finalize_segments(&mut vm), Ok(()));
        assert!(cairo_runner.segments_finalized);
        assert!(cairo_runner.execution_public_memory.unwrap().is_empty())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_segments_run_ended_not_emptyproof_mode_empty_execution_public_memory() {
        let mut program = program!();
        program.data = vec_data![(1), (2), (3), (4), (5), (6), (7), (8)];
        //Program data len = 8
        let mut cairo_runner = cairo_runner!(program, "plain", true);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = true;
        let mut vm = vm!();
        assert_eq!(cairo_runner.finalize_segments(&mut vm), Ok(()));
        assert!(cairo_runner.segments_finalized);
        //Check values written by first call to segments.finalize()
        assert_eq!(vm.segments.segment_sizes.get(&0), Some(&8_usize));
        assert_eq!(
            vm.segments.public_memory_offsets.get(&0),
            Some(&vec![
                (0_usize, 0_usize),
                (1_usize, 0_usize),
                (2_usize, 0_usize),
                (3_usize, 0_usize),
                (4_usize, 0_usize),
                (5_usize, 0_usize),
                (6_usize, 0_usize),
                (7_usize, 0_usize)
            ])
        );
        //Check values written by second call to segments.finalize()
        assert_eq!(vm.segments.segment_sizes.get(&1), None);
        assert_eq!(vm.segments.public_memory_offsets.get(&1), Some(&vec![]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn finalize_segments_run_ended_not_emptyproof_mode_with_execution_public_memory() {
        let mut program = program!();
        program.data = vec_data![(1), (2), (3), (4)];
        //Program data len = 4
        let mut cairo_runner = cairo_runner!(program, "plain", true);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 1)));
        cairo_runner.execution_public_memory = Some(vec![1_usize, 3_usize, 5_usize, 4_usize]);
        cairo_runner.run_ended = true;
        let mut vm = vm!();
        assert_eq!(cairo_runner.finalize_segments(&mut vm), Ok(()));
        assert!(cairo_runner.segments_finalized);
        //Check values written by first call to segments.finalize()
        assert_eq!(vm.segments.segment_sizes.get(&0), Some(&4_usize));
        assert_eq!(
            vm.segments.public_memory_offsets.get(&0),
            Some(&vec![
                (0_usize, 0_usize),
                (1_usize, 0_usize),
                (2_usize, 0_usize),
                (3_usize, 0_usize)
            ])
        );
        //Check values written by second call to segments.finalize()
        assert_eq!(vm.segments.segment_sizes.get(&1), None);
        assert_eq!(
            vm.segments.public_memory_offsets.get(&1),
            Some(&vec![
                (2_usize, 0_usize),
                (4_usize, 0_usize),
                (6_usize, 0_usize),
                (5_usize, 0_usize)
            ])
        );
    }

    /// Test that ensures get_perm_range_check_limits() returns an error when
    /// trace is not enabled.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_perm_range_check_limits_trace_not_enabled() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let vm = vm!();

        assert_matches!(
            cairo_runner.get_perm_range_check_limits(&vm),
            Err(VirtualMachineError::TracerError(
                TraceError::TraceNotEnabled
            ))
        );
    }

    /// Test that ensures get_perm_range_check_limits() returns None when the
    /// trace is empty (get_perm_range_check_limits returns None).
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_perm_range_check_limits_empty() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.trace = Some(vec![]);

        assert_matches!(cairo_runner.get_perm_range_check_limits(&vm), Ok(None));
    }

    /// Test that get_perm_range_check_limits() works correctly when there are
    /// no builtins.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_perm_range_check_limits_no_builtins() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        vm.trace = Some(vec![
            TraceEntry {
                pc: (0, 0).into(),
                ap: (0, 0).into(),
                fp: (0, 0).into(),
            },
            TraceEntry {
                pc: (0, 1).into(),
                ap: (0, 0).into(),
                fp: (0, 0).into(),
            },
            TraceEntry {
                pc: (0, 2).into(),
                ap: (0, 0).into(),
                fp: (0, 0).into(),
            },
        ]);
        vm.segments.memory.data = vec![vec![
            Some(MemoryCell::new(Felt::new(0x80FF_8000_0530u64).into())),
            Some(MemoryCell::new(Felt::new(0xBFFF_8000_0620u64).into())),
            Some(MemoryCell::new(Felt::new(0x8FFF_8000_0750u64).into())),
        ]];

        assert_matches!(
            cairo_runner.get_perm_range_check_limits(&vm),
            Ok(Some((-31440, 16383)))
        );
    }

    /// Test that get_perm_range_check_limits() works correctly when there are
    /// builtins.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_perm_range_check_limits() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        vm.trace = Some(vec![TraceEntry {
            pc: (0, 0).into(),
            ap: (0, 0).into(),
            fp: (0, 0).into(),
        }]);
        vm.segments.memory.data = vec![vec![Some(MemoryCell::new(mayberelocatable!(
            0x80FF_8000_0530u64
        )))]];
        vm.builtin_runners = vec![(
            BuiltinName::range_check,
            RangeCheckBuiltinRunner::new(12, 5, true).into(),
        )];

        assert_matches!(
            cairo_runner.get_perm_range_check_limits(&vm),
            Ok(Some((-31440, 1328)))
        );
    }

    /// Test that check_range_check_usage() returns successfully when trace is
    /// not enabled.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_range_check_usage_perm_range_limits_none() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.trace = Some(vec![]);

        assert_matches!(cairo_runner.check_range_check_usage(&vm), Ok(()));
    }

    /// Test that check_range_check_usage() returns successfully when all the
    /// conditions are met.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_range_check_usage_without_builtins() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.builtin_runners = vec![];
        vm.current_step = 10000;
        vm.segments.memory.data = vec![vec![Some(MemoryCell::new(mayberelocatable!(
            0x80FF_8000_0530u64
        )))]];
        vm.trace = Some(vec![TraceEntry {
            pc: (0, 0).into(),
            ap: (0, 0).into(),
            fp: (0, 0).into(),
        }]);

        assert_matches!(cairo_runner.check_range_check_usage(&vm), Ok(()));
    }

    /// Test that check_range_check_usage() returns an error if there are
    /// insufficient allocated cells.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_range_check_usage_insufficient_allocated_cells() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.builtin_runners = vec![(
            BuiltinName::range_check,
            RangeCheckBuiltinRunner::new(8, 8, true).into(),
        )];
        vm.segments.memory.data = vec![vec![Some(MemoryCell::new(mayberelocatable!(
            0x80FF_8000_0530u64
        )))]];
        vm.trace = Some(vec![TraceEntry {
            pc: (0, 0).into(),
            ap: (0, 0).into(),
            fp: (0, 0).into(),
        }]);

        assert_matches!(
            cairo_runner.check_range_check_usage(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::InsufficientAllocatedCells(_)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_initial_fp_is_none_without_initialization() {
        let program = program!();

        let runner = cairo_runner!(program);

        assert_eq!(None, runner.get_initial_fp());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_initial_fp_can_be_obtained() {
        //This test works with basic Program definition, will later be updated to use Program::new() when fully defined
        let program = program![BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        for _ in 0..2 {
            vm.segments.add();
        }
        cairo_runner.program_base = Some(relocatable!(0, 0));
        cairo_runner.execution_base = Some(relocatable!(1, 0));
        let return_fp = Felt::new(9_i32).into();
        cairo_runner
            .initialize_function_entrypoint(&mut vm, 0, vec![], return_fp)
            .unwrap();
        assert_eq!(Some(relocatable!(1, 2)), cairo_runner.get_initial_fp());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_used_cells_valid_case() {
        let program = program![BuiltinName::range_check, BuiltinName::output];
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);
        vm.trace = Some(vec![]);
        cairo_runner.layout.diluted_pool_instance_def = None;

        assert_matches!(cairo_runner.check_used_cells(&vm), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_used_cells_get_used_cells_and_allocated_size_error() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.builtin_runners = vec![(
            BuiltinName::range_check,
            RangeCheckBuiltinRunner::new(8, 8, true).into(),
        )];
        vm.segments.memory.data = vec![vec![Some(MemoryCell::new(mayberelocatable!(
            0x80FF_8000_0530u64
        )))]];
        vm.trace = Some(vec![TraceEntry {
            pc: (0, 0).into(),
            ap: (0, 0).into(),
            fp: (0, 0).into(),
        }]);

        assert_matches!(
            cairo_runner.check_used_cells(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::InsufficientAllocatedCells(_)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_used_cells_check_memory_usage_error() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.segments.memory.mark_as_accessed((1, 0).into());
        vm.segments.memory.mark_as_accessed((1, 3).into());
        vm.builtin_runners = vec![{
            let mut builtin_runner: BuiltinRunner = OutputBuiltinRunner::new(true).into();
            builtin_runner.initialize_segments(&mut vm.segments);

            (BuiltinName::output, builtin_runner)
        }];
        vm.segments.segment_used_sizes = Some(vec![4, 12]);
        vm.trace = Some(vec![]);

        assert_matches!(
            cairo_runner.check_used_cells(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::InsufficientAllocatedCells(_)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn check_used_cells_check_diluted_check_usage_error() {
        let program = program![BuiltinName::range_check, BuiltinName::output];
        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![4]);
        vm.trace = Some(vec![]);

        assert_matches!(
            cairo_runner.check_used_cells(&vm),
            Err(VirtualMachineError::Memory(
                MemoryError::InsufficientAllocatedCells(_)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_all_builtins() {
        let program = program!();

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner
            .initialize_all_builtins(&mut vm)
            .expect("Builtin initialization failed.");

        let given_output = vm.get_builtin_runners();

        assert_eq!(given_output[0].0, BuiltinName::pedersen);
        assert_eq!(given_output[1].0, BuiltinName::range_check);
        assert_eq!(given_output[2].0, BuiltinName::output);
        assert_eq!(given_output[3].0, BuiltinName::ecdsa);
        assert_eq!(given_output[4].0, BuiltinName::bitwise);
        assert_eq!(given_output[5].0, BuiltinName::ec_op);
        assert_eq!(given_output[6].0, BuiltinName::keccak);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_all_builtins_maintain_program_order() {
        let program = program![
            BuiltinName::pedersen,
            BuiltinName::range_check,
            BuiltinName::ecdsa
        ];

        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner
            .initialize_all_builtins(&mut vm)
            .expect("Builtin initialization failed.");

        let given_output = vm.get_builtin_runners();

        assert_eq!(given_output[0].0, BuiltinName::pedersen);
        assert_eq!(given_output[1].0, BuiltinName::range_check);
        assert_eq!(given_output[2].0, BuiltinName::ecdsa);
        assert_eq!(given_output[3].0, BuiltinName::output);
        assert_eq!(given_output[4].0, BuiltinName::bitwise);
        assert_eq!(given_output[5].0, BuiltinName::ec_op);
        assert_eq!(given_output[6].0, BuiltinName::keccak);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_function_runner() {
        let program = program!();

        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        cairo_runner
            .initialize_function_runner(&mut vm)
            .expect("initialize_function_runner failed.");

        let builtin_runners = vm.get_builtin_runners();

        assert_eq!(builtin_runners[0].0, BuiltinName::pedersen);
        assert_eq!(builtin_runners[1].0, BuiltinName::range_check);
        assert_eq!(builtin_runners[2].0, BuiltinName::output);
        assert_eq!(builtin_runners[3].0, BuiltinName::ecdsa);
        assert_eq!(builtin_runners[4].0, BuiltinName::bitwise);
        assert_eq!(builtin_runners[5].0, BuiltinName::ec_op);
        assert_eq!(builtin_runners[6].0, BuiltinName::keccak);

        assert_eq!(
            cairo_runner.program_base,
            Some(Relocatable {
                segment_index: 0,
                offset: 0,
            })
        );
        assert_eq!(
            cairo_runner.execution_base,
            Some(Relocatable {
                segment_index: 1,
                offset: 0,
            })
        );
        assert_eq!(vm.segments.num_segments(), 9);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_segments_incorrect_layout_plain_one_builtin() {
        let program = program![BuiltinName::output];
        let mut vm = vm!();
        let cairo_runner = cairo_runner!(program, "plain");
        assert_eq!(
            cairo_runner.initialize_builtins(&mut vm),
            Err(RunnerError::NoBuiltinForInstance(
                HashSet::from([BuiltinName::output]),
                String::from("plain")
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_segments_incorrect_layout_plain_two_builtins() {
        let program = program![BuiltinName::output, BuiltinName::pedersen];
        let mut vm = vm!();
        let cairo_runner = cairo_runner!(program, "plain");
        assert_eq!(
            cairo_runner.initialize_builtins(&mut vm),
            Err(RunnerError::NoBuiltinForInstance(
                HashSet::from([BuiltinName::output, BuiltinName::pedersen]),
                String::from("plain")
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_segments_incorrect_layout_small_two_builtins() {
        let program = program![BuiltinName::output, BuiltinName::bitwise];
        let mut vm = vm!();
        let cairo_runner = cairo_runner!(program, "small");
        assert_eq!(
            cairo_runner.initialize_builtins(&mut vm),
            Err(RunnerError::NoBuiltinForInstance(
                HashSet::from([BuiltinName::bitwise]),
                String::from("small")
            ))
        );
    }
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_main_entrypoint_proof_mode_empty_program() {
        let program = program!(start = Some(0), end = Some(0), main = Some(8),);
        let mut runner = cairo_runner!(program);
        runner.proof_mode = true;
        let mut vm = vm!();
        runner.initialize_segments(&mut vm, None);
        assert_eq!(runner.execution_base, Some(Relocatable::from((1, 0))));
        assert_eq!(runner.program_base, Some(Relocatable::from((0, 0))));
        assert_eq!(
            runner.initialize_main_entrypoint(&mut vm),
            Ok(Relocatable::from((0, 0)))
        );
        assert_eq!(runner.initial_ap, Some(Relocatable::from((1, 2))));
        assert_eq!(runner.initial_fp, runner.initial_ap);
        assert_eq!(runner.execution_public_memory, Some(vec![0, 1]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initialize_main_entrypoint_proof_mode_empty_program_two_builtins() {
        let program = program!(
            start = Some(0),
            end = Some(0),
            main = Some(8),
            builtins = vec![BuiltinName::output, BuiltinName::ec_op],
        );
        let mut runner = cairo_runner!(program);
        runner.proof_mode = true;
        let mut vm = vm!();
        runner.initialize_builtins(&mut vm).unwrap();
        runner.initialize_segments(&mut vm, None);
        assert_eq!(runner.execution_base, Some(Relocatable::from((1, 0))));
        assert_eq!(runner.program_base, Some(Relocatable::from((0, 0))));
        assert_eq!(
            runner.initialize_main_entrypoint(&mut vm),
            Ok(Relocatable::from((0, 0)))
        );
        assert_eq!(runner.initial_ap, Some(Relocatable::from((1, 2))));
        assert_eq!(runner.initial_fp, runner.initial_ap);
        assert_eq!(runner.execution_public_memory, Some(vec![0, 1, 2, 3]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn can_get_the_runner_program_builtins() {
        let program = program!(
            start = Some(0),
            end = Some(0),
            main = Some(8),
            builtins = vec![BuiltinName::output, BuiltinName::ec_op],
        );
        let runner = cairo_runner!(program);

        assert_eq!(&program.builtins, runner.get_program_builtins());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn set_entrypoint_main_default() {
        let program = program!();
        let mut cairo_runner = cairo_runner!(program);

        cairo_runner.program.identifiers = [(
            "__main__.main",
            Identifier {
                pc: Some(0),
                type_: None,
                value: None,
                full_name: None,
                members: None,
                cairo_type: None,
            },
        )]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

        cairo_runner
            .set_entrypoint(None)
            .expect("Call to `set_entrypoint()` failed.");
        assert_eq!(cairo_runner.program.main, Some(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn set_entrypoint_main() {
        let program = program!();
        let mut cairo_runner = cairo_runner!(program);

        cairo_runner.program.identifiers = [
            (
                "__main__.main",
                Identifier {
                    pc: Some(0),
                    type_: None,
                    value: None,
                    full_name: None,
                    members: None,
                    cairo_type: None,
                },
            ),
            (
                "__main__.alternate_main",
                Identifier {
                    pc: Some(1),
                    type_: None,
                    value: None,
                    full_name: None,
                    members: None,
                    cairo_type: None,
                },
            ),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

        cairo_runner
            .set_entrypoint(Some("alternate_main"))
            .expect("Call to `set_entrypoint()` failed.");
        assert_eq!(cairo_runner.program.main, Some(1));
    }

    /// Test that set_entrypoint() fails when the entrypoint doesn't exist.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn set_entrypoint_main_non_existent() {
        let program = program!();
        let mut cairo_runner = cairo_runner!(program);

        cairo_runner.program.identifiers = [(
            "__main__.main",
            Identifier {
                pc: Some(0),
                type_: None,
                value: None,
                full_name: None,
                members: None,
                cairo_type: None,
            },
        )]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

        cairo_runner
            .set_entrypoint(Some("nonexistent_main"))
            .expect_err("Call to `set_entrypoint()` succeeded (should've failed).");
        assert_eq!(cairo_runner.program.main, None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn read_return_values_test() {
        let mut program = program!();
        program.data = vec_data![(1), (2), (3), (4), (5), (6), (7), (8)];
        //Program data len = 8
        let mut cairo_runner = cairo_runner!(program, "plain", true);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = true;
        cairo_runner.segments_finalized = false;
        let mut vm = vm!();
        //Check values written by first call to segments.finalize()

        assert_eq!(cairo_runner.read_return_values(&mut vm), Ok(()));
        assert_eq!(
            cairo_runner
                .execution_public_memory
                .expect("missing execution public memory"),
            Vec::<usize>::new()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn read_return_values_test_with_run_not_ended() {
        let mut program = program!();
        program.data = vec_data![(1), (2), (3), (4), (5), (6), (7), (8)];
        //Program data len = 8
        let mut cairo_runner = cairo_runner!(program, "plain", true);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = false;
        let mut vm = vm!();
        assert_eq!(
            cairo_runner.read_return_values(&mut vm),
            Err(RunnerError::ReadReturnValuesNoEndRun)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn read_return_values_test_with_segments_finalized() {
        let mut program = program!();
        program.data = vec_data![(1), (2), (3), (4), (5), (6), (7), (8)];
        //Program data len = 8
        let mut cairo_runner = cairo_runner!(program, "plain", true);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = true;
        cairo_runner.segments_finalized = true;
        let mut vm = vm!();
        assert_eq!(
            cairo_runner.read_return_values(&mut vm),
            Err(RunnerError::FailedAddingReturnValues)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn read_return_values_updates_builtin_stop_ptr_one_builtin_empty() {
        let mut program = program![BuiltinName::output];
        program.data = vec_data![(1), (2), (3), (4), (5), (6), (7), (8)];
        //Program data len = 8
        let mut cairo_runner = cairo_runner!(program, "all", true);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = true;
        cairo_runner.segments_finalized = false;
        let mut vm = vm!();
        let output_builtin = OutputBuiltinRunner::new(true);
        vm.builtin_runners
            .push((BuiltinName::output, output_builtin.into()));
        vm.segments.memory.data = vec![
            vec![],
            vec![Some(MemoryCell::new(MaybeRelocatable::from((0, 0))))],
            vec![],
        ];
        vm.set_ap(1);
        vm.segments.segment_used_sizes = Some(vec![0, 1, 0]);
        //Check values written by first call to segments.finalize()
        assert_eq!(cairo_runner.read_return_values(&mut vm), Ok(()));
        let output_builtin = match &vm.builtin_runners[0].1 {
            BuiltinRunner::Output(runner) => runner,
            _ => unreachable!(),
        };
        assert_eq!(output_builtin.stop_ptr, Some(0))
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn read_return_values_updates_builtin_stop_ptr_one_builtin_one_element() {
        let mut program = program![BuiltinName::output];
        program.data = vec_data![(1), (2), (3), (4), (5), (6), (7), (8)];
        //Program data len = 8
        let mut cairo_runner = cairo_runner!(program, "all", true);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = true;
        cairo_runner.segments_finalized = false;
        let mut vm = vm!();
        let output_builtin = OutputBuiltinRunner::new(true);
        vm.builtin_runners
            .push((BuiltinName::output, output_builtin.into()));
        vm.segments.memory.data = vec![
            vec![Some(MemoryCell::new(MaybeRelocatable::from((0, 0))))],
            vec![Some(MemoryCell::new(MaybeRelocatable::from((0, 1))))],
            vec![],
        ];
        vm.set_ap(1);
        vm.segments.segment_used_sizes = Some(vec![1, 1, 0]);
        //Check values written by first call to segments.finalize()
        assert_eq!(cairo_runner.read_return_values(&mut vm), Ok(()));
        let output_builtin = match &vm.builtin_runners[0].1 {
            BuiltinRunner::Output(runner) => runner,
            _ => unreachable!(),
        };
        assert_eq!(output_builtin.stop_ptr, Some(1))
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn read_return_values_updates_builtin_stop_ptr_two_builtins() {
        let mut program = program![BuiltinName::output, BuiltinName::bitwise];
        program.data = vec_data![(1), (2), (3), (4), (5), (6), (7), (8)];
        //Program data len = 8
        let mut cairo_runner = cairo_runner!(program, "all", true);
        cairo_runner.program_base = Some(Relocatable::from((0, 0)));
        cairo_runner.execution_base = Some(Relocatable::from((1, 0)));
        cairo_runner.run_ended = true;
        cairo_runner.segments_finalized = false;
        let mut vm = vm!();
        let output_builtin = OutputBuiltinRunner::new(true);
        let bitwise_builtin = BitwiseBuiltinRunner::new(&BitwiseInstanceDef::default(), true);
        vm.builtin_runners
            .push((BuiltinName::output, output_builtin.into()));
        vm.builtin_runners
            .push((BuiltinName::bitwise, bitwise_builtin.into()));
        cairo_runner.initialize_segments(&mut vm, None);
        vm.segments.memory.data = vec![
            vec![Some(MemoryCell::new(MaybeRelocatable::from((0, 0))))],
            vec![
                Some(MemoryCell::new(MaybeRelocatable::from((2, 0)))),
                Some(MemoryCell::new(MaybeRelocatable::from((3, 5)))),
            ],
            vec![],
        ];
        vm.set_ap(2);
        // We use 5 as bitwise builtin's segment size as a bitwise instance is 5 cells
        vm.segments.segment_used_sizes = Some(vec![0, 2, 0, 5]);
        //Check values written by first call to segments.finalize()
        assert_eq!(cairo_runner.read_return_values(&mut vm), Ok(()));
        let output_builtin = match &vm.builtin_runners[0].1 {
            BuiltinRunner::Output(runner) => runner,
            _ => unreachable!(),
        };
        assert_eq!(output_builtin.stop_ptr, Some(0));
        assert_eq!(cairo_runner.read_return_values(&mut vm), Ok(()));
        let bitwise_builtin = match &vm.builtin_runners[1].1 {
            BuiltinRunner::Bitwise(runner) => runner,
            _ => unreachable!(),
        };
        assert_eq!(bitwise_builtin.stop_ptr, Some(5));
    }

    /// Test that add_additional_hash_builtin() creates an additional builtin.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_additional_hash_builtin() {
        let program = program!();
        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        let num_builtins = vm.builtin_runners.len();
        cairo_runner.add_additional_hash_builtin(&mut vm);
        assert_eq!(vm.builtin_runners.len(), num_builtins + 1);

        let (key, value) = vm
            .builtin_runners
            .last()
            .expect("missing last builtin runner");
        assert_eq!(*key, BuiltinName::hash_builtin);
        match value {
            BuiltinRunner::Hash(builtin) => {
                assert_eq!(builtin.base(), 0);
                assert_eq!(builtin.ratio(), 32);
                assert!(builtin.included);
            }
            _ => unreachable!(),
        }
    }

    /// Test that add_additional_hash_builtin() replaces the created runner if called multiple
    /// times.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_additional_hash_builtin_replace() {
        let program = program!();
        let cairo_runner = cairo_runner!(program);
        let mut vm = vm!();

        let num_builtins = vm.builtin_runners.len();
        cairo_runner.add_additional_hash_builtin(&mut vm);
        cairo_runner.add_additional_hash_builtin(&mut vm);
        assert_eq!(vm.builtin_runners.len(), num_builtins + 1);

        let (key, value) = vm
            .builtin_runners
            .last()
            .expect("missing last builtin runner");
        assert_eq!(*key, BuiltinName::hash_builtin);
        match value {
            BuiltinRunner::Hash(builtin) => {
                assert_eq!(builtin.base(), 1);
                assert_eq!(builtin.ratio(), 32);
                assert!(builtin.included);
            }
            _ => unreachable!(),
        }
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_from_entrypoint_custom_program_test() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/example_program.json"),
            None,
        )
        .unwrap();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!(true); //this true expression dictates that the trace is enabled
        let mut hint_processor = BuiltinHintProcessor::new_empty();

        //this entrypoint tells which function to run in the cairo program
        let main_entrypoint = program
            .identifiers
            .get("__main__.main")
            .unwrap()
            .pc
            .unwrap();

        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);
        assert_matches!(
            cairo_runner.run_from_entrypoint(
                main_entrypoint,
                &[
                    &mayberelocatable!(2).into(),
                    &MaybeRelocatable::from((2, 0)).into()
                ], //range_check_ptr
                true,
                &mut vm,
                &mut hint_processor,
            ),
            Ok(())
        );

        let mut new_cairo_runner = cairo_runner!(program);
        let mut new_vm = vm!(true); //this true expression dictates that the trace is enabled
        let mut hint_processor = BuiltinHintProcessor::new_empty();

        new_cairo_runner.initialize_builtins(&mut new_vm).unwrap();
        new_cairo_runner.initialize_segments(&mut new_vm, None);

        let fib_entrypoint = program
            .identifiers
            .get("__main__.evaluate_fib")
            .unwrap()
            .pc
            .unwrap();

        assert_matches!(
            new_cairo_runner.run_from_entrypoint(
                fib_entrypoint,
                &[
                    &mayberelocatable!(2).into(),
                    &MaybeRelocatable::from((2, 0)).into()
                ],
                true,
                &mut new_vm,
                &mut hint_processor,
            ),
            Ok(())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn cairo_arg_from_single() {
        let expected = CairoArg::Single(MaybeRelocatable::from((0, 0)));
        let value = MaybeRelocatable::from((0, 0));
        assert_eq!(expected, value.into())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn cairo_arg_from_array() {
        let expected = CairoArg::Array(vec![MaybeRelocatable::from((0, 0))]);
        let value = vec![MaybeRelocatable::from((0, 0))];
        assert_eq!(expected, value.into())
    }

    fn setup_execution_resources() -> (ExecutionResources, ExecutionResources) {
        let mut builtin_instance_counter: HashMap<BuiltinName, usize> = HashMap::new();
        builtin_instance_counter.insert(BuiltinName::output, 8);

        let execution_resources_1 = ExecutionResources {
            n_steps: 100,
            n_memory_holes: 5,
            builtin_instance_counter: builtin_instance_counter.clone(),
        };

        //Test that the combined Execution Resources only contains the shared builtins
        builtin_instance_counter.insert(BuiltinName::range_check, 8);

        let execution_resources_2 = ExecutionResources {
            n_steps: 100,
            n_memory_holes: 5,
            builtin_instance_counter,
        };

        (execution_resources_1, execution_resources_2)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn execution_resources_add() {
        let (execution_resources_1, execution_resources_2) = setup_execution_resources();
        let combined_resources = execution_resources_1 + execution_resources_2;

        assert_eq!(combined_resources.n_steps, 200);
        assert_eq!(combined_resources.n_memory_holes, 10);
        assert_eq!(
            combined_resources
                .builtin_instance_counter
                .get(&BuiltinName::output)
                .unwrap(),
            &16
        );
        assert!(!combined_resources
            .builtin_instance_counter
            .contains_key(&BuiltinName::range_check));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn execution_resources_sub() {
        let (execution_resources_1, execution_resources_2) = setup_execution_resources();

        let combined_resources = execution_resources_1 - execution_resources_2;

        assert_eq!(combined_resources.n_steps, 0);
        assert_eq!(combined_resources.n_memory_holes, 0);
        assert_eq!(
            combined_resources
                .builtin_instance_counter
                .get(&BuiltinName::output)
                .unwrap(),
            &0
        );
        assert!(!combined_resources
            .builtin_instance_counter
            .contains_key(&BuiltinName::range_check));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_from_entrypoint_substitute_error_message_test() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/bad_programs/error_msg_function.json"),
            None,
        )
        .unwrap();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!(true); //this true expression dictates that the trace is enabled
        let mut hint_processor = BuiltinHintProcessor::new_empty();

        //this entrypoint tells which function to run in the cairo program
        let main_entrypoint = program
            .identifiers
            .get("__main__.main")
            .unwrap()
            .pc
            .unwrap();

        cairo_runner.initialize_builtins(&mut vm).unwrap();
        cairo_runner.initialize_segments(&mut vm, None);

        let result = cairo_runner.run_from_entrypoint(
            main_entrypoint,
            &[],
            true,
            &mut vm,
            &mut hint_processor,
        );
        match result {
            Err(CairoRunError::VmException(exception)) => {
                assert_eq!(
                    exception.error_attr_value,
                    Some(String::from("Error message: Test error\n"))
                )
            }
            Err(_) => panic!("Wrong error returned, expected VmException"),
            Ok(_) => panic!("Expected run to fail"),
        }
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_final_stack_range_check_builtin() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/assert_le_felt_hint.json"),
            Some("main"),
        )
        .unwrap();
        let mut runner = cairo_runner!(program);
        let mut vm = vm!();
        let end = runner.initialize(&mut vm).unwrap();
        runner
            .run_until_pc(end, &mut vm, &mut BuiltinHintProcessor::new_empty())
            .unwrap();
        vm.segments.compute_effective_sizes();
        let initial_pointer = vm.get_ap();
        let expected_pointer = (vm.get_ap() - 1).unwrap();
        assert_eq!(
            runner.get_builtins_final_stack(&mut vm, initial_pointer),
            Ok(expected_pointer)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_final_stack_4_builtins() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/integration.json"),
            Some("main"),
        )
        .unwrap();
        let mut runner = cairo_runner!(program);
        let mut vm = vm!();
        let end = runner.initialize(&mut vm).unwrap();
        runner
            .run_until_pc(end, &mut vm, &mut BuiltinHintProcessor::new_empty())
            .unwrap();
        vm.segments.compute_effective_sizes();
        let initial_pointer = vm.get_ap();
        let expected_pointer = (vm.get_ap() - 4).unwrap();
        assert_eq!(
            runner.get_builtins_final_stack(&mut vm, initial_pointer),
            Ok(expected_pointer)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_final_stack_no_builtins() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/fibonacci.json"),
            Some("main"),
        )
        .unwrap();
        let mut runner = cairo_runner!(program);
        let mut vm = vm!();
        let end = runner.initialize(&mut vm).unwrap();
        runner
            .run_until_pc(end, &mut vm, &mut BuiltinHintProcessor::new_empty())
            .unwrap();
        vm.segments.compute_effective_sizes();
        let initial_pointer = vm.get_ap();
        let expected_pointer = vm.get_ap();
        assert_eq!(
            runner.get_builtins_final_stack(&mut vm, initial_pointer),
            Ok(expected_pointer)
        );
    }
}
