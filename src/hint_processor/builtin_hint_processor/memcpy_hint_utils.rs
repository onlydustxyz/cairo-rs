use std::prelude::v1::*;

use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{
            get_integer_from_var_name, insert_value_from_var_name, insert_value_into_ap,
        },
        hint_processor_definition::HintReference,
    },
    serde::deserialize_program::ApTracking,
    types::exec_scope::ExecutionScopes,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use felt::Felt;
use num_traits::{One, Zero};
use std::{any::Any, collections::HashMap};

//Implements hint: memory[ap] = segments.add()
pub fn add_segment(vm: &mut VirtualMachine) -> Result<(), HintError> {
    let new_segment_base = vm.add_memory_segment();
    insert_value_into_ap(vm, new_segment_base)
}

//Implements hint: vm_enter_scope()
pub fn enter_scope(exec_scopes: &mut ExecutionScopes) -> Result<(), HintError> {
    exec_scopes.enter_scope(HashMap::new());
    Ok(())
}

//  Implements hint:
//  %{ vm_exit_scope() %}
pub fn exit_scope(exec_scopes: &mut ExecutionScopes) -> Result<(), HintError> {
    exec_scopes.exit_scope().map_err(HintError::FromScopeError)
}

//  Implements hint:
//  %{ vm_enter_scope({'n': ids.len}) %}
pub fn memcpy_enter_scope(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let len: Box<dyn Any> =
        Box::new(get_integer_from_var_name("len", vm, ids_data, ap_tracking)?.into_owned());
    exec_scopes.enter_scope(HashMap::from([(String::from("n"), len)]));
    Ok(())
}

// Implements hint:
// %{
//     n -= 1
//     ids.continue_copying = 1 if n > 0 else 0
// %}
pub fn memcpy_continue_copying(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    // get `n` variable from vm scope
    let n = exec_scopes.get_ref::<Felt>("n")?;
    // this variable will hold the value of `n - 1`
    let new_n = n - 1;
    // if it is positive, insert 1 in the address of `continue_copying`
    // else, insert 0
    if new_n.is_zero() {
        insert_value_from_var_name("continue_copying", &new_n, vm, ids_data, ap_tracking)?;
    } else {
        insert_value_from_var_name("continue_copying", Felt::one(), vm, ids_data, ap_tracking)?;
    }
    exec_scopes.insert_value("n", new_n);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        types::relocatable::MaybeRelocatable,
        utils::test_utils::*,
        vm::{
            errors::{memory_errors::MemoryError, vm_errors::VirtualMachineError},
            vm_memory::memory::Memory,
        },
    };

    #[test]
    fn get_integer_from_var_name_valid() {
        let mut vm = vm!();
        // initialize memory segments
        vm.segments.add(&mut vm.memory);

        // initialize fp
        vm.run_context.fp = 1;

        let var_name: &str = "variable";

        //Create ids_data
        let ids_data = ids_data![var_name];

        //Insert ids.prev_locs.exp into memory
        vm.memory = memory![((1, 0), 10)];

        assert_eq!(
            get_integer_from_var_name(var_name, &vm, &ids_data, &ApTracking::default())
                .unwrap()
                .as_ref(),
            &Felt::new(10)
        );
    }

    #[test]
    fn get_integer_from_var_name_invalid_expected_integer() {
        let mut vm = vm!();

        // initialize fp
        vm.run_context.fp = 1;

        let var_name: &str = "variable";

        //Create ids_data
        let ids_data = ids_data![var_name];

        //Insert ids.variable into memory as a RelocatableValue
        vm.memory = memory![((1, 0), (1, 1))];

        assert_eq!(
            get_integer_from_var_name(var_name, &vm, &ids_data, &ApTracking::default()),
            Err(HintError::Internal(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 0))
            )))
        );
    }
}
