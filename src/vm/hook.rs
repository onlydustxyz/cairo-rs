use std::{collections::HashMap, sync::Arc};

use num_bigint::BigInt;

use crate::types::exec_scope::ExecutionScopes;

use super::{errors::vm_errors::VirtualMachineError, vm_core::VirtualMachine};

/// PreStepInstructionHook is called before the VirtualMachine
/// will execute an instruction
type PreStepInstructionHook = Arc<
    dyn Fn(
            &mut VirtualMachine,
            &mut ExecutionScopes,
            &HashMap<String, BigInt>,
        ) -> Result<(), VirtualMachineError>
        + Sync
        + Send,
>;

/// PostStepInstructionHook is called after the VirtualMachine
/// executed an instruction
type PostStepInstructionHook = Arc<
    dyn Fn(
            &mut VirtualMachine,
            &mut ExecutionScopes,
            &HashMap<String, BigInt>,
        ) -> Result<(), VirtualMachineError>
        + Sync
        + Send,
>;

#[derive(Clone)]
pub struct Hooks {
    pre_step_instruction: PreStepInstructionHook,
    post_step_instruction: PostStepInstructionHook,
}

impl Hooks {
    pub fn new(
        pre_step_instruction: PreStepInstructionHook,
        post_step_instruction: PostStepInstructionHook,
    ) -> Self {
        Hooks {
            pre_step_instruction,
            post_step_instruction,
        }
    }

    pub fn execute_pre_step_instruction(
        &self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        constants: &HashMap<String, BigInt>,
    ) -> Result<(), VirtualMachineError> {
        (self.pre_step_instruction)(vm, exec_scopes, constants)
    }

    pub fn execute_post_step_instruction(
        &self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        constants: &HashMap<String, BigInt>,
    ) -> Result<(), VirtualMachineError> {
        (self.post_step_instruction)(vm, exec_scopes, constants)
    }
}
