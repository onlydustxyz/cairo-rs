use std::{collections::HashMap, sync::Arc};

use num_bigint::BigInt;

use crate::types::exec_scope::ExecutionScopes;

use super::{errors::vm_errors::VirtualMachineError, vm_core::VirtualMachine};

type PreStepInstructionHook = Arc<
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
}

impl Hooks {
    pub fn new(pre_step_instruction: PreStepInstructionHook) -> Self {
        Hooks {
            pre_step_instruction,
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
}
