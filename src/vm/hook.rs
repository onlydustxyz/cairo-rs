use std::collections::HashMap;

use num_bigint::BigInt;

use crate::types::{exec_scope::ExecutionScopes, instruction::Instruction};

use super::{errors::vm_errors::VirtualMachineError, vm_core::VirtualMachine};

pub struct Hook(
    Box<
        dyn Fn(
                &mut VirtualMachine,
                &Instruction,
                &mut ExecutionScopes,
                &HashMap<String, BigInt>,
            ) -> Result<(), VirtualMachineError>
            + Sync,
    >,
);

impl Hook {
    pub fn new(
        function: Box<
            dyn Fn(
                    &mut VirtualMachine,
                    &Instruction,
                    &mut ExecutionScopes,
                    &HashMap<String, BigInt>,
                ) -> Result<(), VirtualMachineError>
                + Sync,
        >,
    ) -> Self {
        Hook(function)
    }

    pub fn execute(
        &self,
        vm: &mut VirtualMachine,
        instruction: &Instruction,
        exec_scopes: &mut ExecutionScopes,
        constants: &HashMap<String, BigInt>,
    ) -> Result<(), VirtualMachineError> {
        (self.0)(vm, instruction, exec_scopes, constants)
    }
}
