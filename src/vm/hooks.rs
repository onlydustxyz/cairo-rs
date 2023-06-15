//! VM hooks
//!
//! Make it possible to execute custom arbitrary code at different stages of the VM execution
//!
//! If added to the VM, hooks function will be called during the VM execution at specific stages.
//!
//! Available hooks:
//! - before_first_step, executed before entering the execution loop in [run_until_pc](CairoRunner::run_until_pc)
//! - pre_step_instruction, executed before each instruction_step in [step](VirtualMachine::step)
//! - post_step_instruction, executed after each instruction_step in [step](VirtualMachine::step)

use crate::stdlib::{any::Any, collections::HashMap, prelude::*, sync::Arc};

use felt::Felt252;

use crate::{
    hint_processor::hint_processor_definition::HintProcessor, types::exec_scope::ExecutionScopes,
};

use super::{
    errors::vm_errors::VirtualMachineError, runners::cairo_runner::CairoRunner,
    vm_core::VirtualMachine,
};

type BeforeFirstStepHookFunc = Arc<
    dyn Fn(
            &mut VirtualMachine,
            &mut CairoRunner,
            &HashMap<usize, Vec<Box<dyn Any>>>,
        ) -> Result<(), VirtualMachineError>
        + Sync
        + Send,
>;

type StepHookFunc = Arc<
    dyn Fn(
            &mut VirtualMachine,
            &mut dyn HintProcessor,
            &mut ExecutionScopes,
            &HashMap<usize, Vec<Box<dyn Any>>>,
            &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError>
        + Sync
        + Send,
>;

/// The hooks to be executed during the VM run
///
/// They can be individually ignored by setting them to [None]
#[derive(Clone, Default)]
pub struct Hooks {
    before_first_step: Option<BeforeFirstStepHookFunc>,
    pre_step_instruction: Option<StepHookFunc>,
    post_step_instruction: Option<StepHookFunc>,
}

impl Hooks {
    pub fn new(
        before_first_step: Option<BeforeFirstStepHookFunc>,
        pre_step_instruction: Option<StepHookFunc>,
        post_step_instruction: Option<StepHookFunc>,
    ) -> Self {
        Hooks {
            before_first_step,
            pre_step_instruction,
            post_step_instruction,
        }
    }
}

impl VirtualMachine {
    pub fn execute_before_first_step(
        &mut self,
        runner: &mut CairoRunner,
        hint_data_dictionary: &HashMap<usize, Vec<Box<dyn Any>>>,
    ) -> Result<(), VirtualMachineError> {
        if let Some(hook_func) = self.hooks.clone().before_first_step {
            (hook_func)(self, runner, hint_data_dictionary)?;
        }

        Ok(())
    }

    pub fn execute_pre_step_instruction(
        &mut self,
        hint_executor: &mut dyn HintProcessor,
        exec_scope: &mut ExecutionScopes,
        hint_data_dictionary: &HashMap<usize, Vec<Box<dyn Any>>>,
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), VirtualMachineError> {
        if let Some(hook_func) = self.hooks.clone().pre_step_instruction {
            (hook_func)(
                self,
                hint_executor,
                exec_scope,
                hint_data_dictionary,
                constants,
            )?;
        }

        Ok(())
    }

    pub fn execute_post_step_instruction(
        &mut self,
        hint_executor: &mut dyn HintProcessor,
        exec_scope: &mut ExecutionScopes,
        hint_data_dictionary: &HashMap<usize, Vec<Box<dyn Any>>>,
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), VirtualMachineError> {
        if let Some(hook_func) = self.hooks.clone().post_step_instruction {
            (hook_func)(
                self,
                hint_executor,
                exec_scope,
                hint_data_dictionary,
                constants,
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
        types::program::Program,
        utils::test_utils::{cairo_runner, vm},
    };

    #[test]
    fn empty_hooks() {
        let program = Program::from_bytes(
            include_bytes!("../../cairo_programs/sqrt.json"),
            Some("main"),
        )
        .expect("Call to `Program::from_file()` failed.");

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.hooks = Hooks::new(None, None, None);

        let end = cairo_runner.initialize(&mut vm).unwrap();
        assert!(cairo_runner
            .run_until_pc(end, &mut vm, &mut hint_processor)
            .is_ok());
    }

    #[test]
    fn hook_failure() {
        let program = Program::from_bytes(
            include_bytes!("../../cairo_programs/sqrt.json"),
            Some("main"),
        )
        .expect("Call to `Program::from_file()` failed.");

        fn before_first_step_hook(
            _vm: &mut VirtualMachine,
            _runner: &mut CairoRunner,
            _hint_data: &HashMap<usize, Vec<Box<dyn Any>>>,
        ) -> Result<(), VirtualMachineError> {
            Err(VirtualMachineError::Unexpected)
        }

        fn pre_step_hook(
            _vm: &mut VirtualMachine,
            _hint_processor: &mut dyn HintProcessor,
            _exec_scope: &mut ExecutionScopes,
            _hint_data: &HashMap<usize, Vec<Box<dyn Any>>>,
            _constants: &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError> {
            Err(VirtualMachineError::Unexpected)
        }

        fn post_step_hook(
            _vm: &mut VirtualMachine,
            _hint_processor: &mut dyn HintProcessor,
            _exec_scope: &mut ExecutionScopes,
            _hint_data: &HashMap<usize, Vec<Box<dyn Any>>>,
            _constants: &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError> {
            Err(VirtualMachineError::Unexpected)
        }

        // Before first fail
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.hooks = Hooks::new(Some(Arc::new(before_first_step_hook)), None, None);

        let end = cairo_runner.initialize(&mut vm).unwrap();
        assert!(cairo_runner
            .run_until_pc(end, &mut vm, &mut hint_processor)
            .is_err());

        // Pre step fail
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.hooks = Hooks::new(None, Some(Arc::new(pre_step_hook)), None);

        let end = cairo_runner.initialize(&mut vm).unwrap();
        assert!(cairo_runner
            .run_until_pc(end, &mut vm, &mut hint_processor)
            .is_err());

        // Post step fail
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.hooks = Hooks::new(None, None, Some(Arc::new(post_step_hook)));

        let end = cairo_runner.initialize(&mut vm).unwrap();
        assert!(cairo_runner
            .run_until_pc(end, &mut vm, &mut hint_processor)
            .is_err());
    }

    #[test]
    fn hook_success() {
        let program = Program::from_bytes(
            include_bytes!("../../cairo_programs/sqrt.json"),
            Some("main"),
        )
        .expect("Call to `Program::from_file()` failed.");

        fn before_first_step_hook(
            _vm: &mut VirtualMachine,
            _runner: &mut CairoRunner,
            _hint_data: &HashMap<usize, Vec<Box<dyn Any>>>,
        ) -> Result<(), VirtualMachineError> {
            Ok(())
        }

        fn pre_step_hook(
            _vm: &mut VirtualMachine,
            _hint_processor: &mut dyn HintProcessor,
            _exec_scope: &mut ExecutionScopes,
            _hint_data: &HashMap<usize, Vec<Box<dyn Any>>>,
            _constants: &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError> {
            Ok(())
        }

        fn post_step_hook(
            _vm: &mut VirtualMachine,
            _hint_processor: &mut dyn HintProcessor,
            _exec_scope: &mut ExecutionScopes,
            _hint_data: &HashMap<usize, Vec<Box<dyn Any>>>,
            _constants: &HashMap<String, Felt252>,
        ) -> Result<(), VirtualMachineError> {
            Ok(())
        }

        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let mut cairo_runner = cairo_runner!(program);
        let mut vm = vm!();
        vm.hooks = Hooks::new(
            Some(Arc::new(before_first_step_hook)),
            Some(Arc::new(pre_step_hook)),
            Some(Arc::new(post_step_hook)),
        );

        let end = cairo_runner.initialize(&mut vm).unwrap();
        assert!(cairo_runner
            .run_until_pc(end, &mut vm, &mut hint_processor)
            .is_ok());
    }
}
