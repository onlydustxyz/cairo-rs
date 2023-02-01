use std::prelude::v1::*;

use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{
            get_integer_from_var_name, insert_value_from_var_name,
        },
        hint_processor_definition::HintReference,
    },
    serde::deserialize_program::ApTracking,
    types::exec_scope::ExecutionScopes,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use felt::Felt;
use num_traits::Signed;
use std::{any::Any, collections::HashMap};

//  Implements hint:
//  %{ vm_enter_scope({'n': ids.n}) %}
pub fn memset_enter_scope(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let n: Box<dyn Any> =
        Box::new(get_integer_from_var_name("n", vm, ids_data, ap_tracking)?.into_owned());
    exec_scopes.enter_scope(HashMap::from([(String::from("n"), n)]));
    Ok(())
}

/* Implements hint:
%{
    n -= 1
    ids.continue_loop = 1 if n > 0 else 0
%}
*/
pub fn memset_continue_loop(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    // get `n` variable from vm scope
    let n = exec_scopes.get_ref::<Felt>("n")?;
    // this variable will hold the value of `n - 1`
    let new_n = n - 1;
    // if `new_n` is positive, insert 1 in the address of `continue_loop`
    // else, insert 0
    let should_continue = Felt::new(new_n.is_positive() as i32);
    insert_value_from_var_name("continue_loop", should_continue, vm, ids_data, ap_tracking)?;
    // Reassign `n` with `n - 1`
    // we do it at the end of the function so that the borrow checker doesn't complain
    exec_scopes.insert_value("n", new_n);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::builtin_hint_processor_definition::{
                BuiltinHintProcessor, HintProcessorData,
            },
            hint_processor_definition::HintProcessor,
        },
        types::{exec_scope::ExecutionScopes, relocatable::MaybeRelocatable},
        utils::test_utils::*,
        vm::{
            errors::{memory_errors::MemoryError, vm_errors::VirtualMachineError},
            vm_memory::memory::Memory,
        },
    };
    use num_traits::{One, Zero};

    #[test]
    fn memset_enter_scope_valid() {
        let hint_code = "vm_enter_scope({'n': ids.n})";
        let mut vm = vm!();
        // initialize fp
        vm.run_context.fp = 2;
        // insert ids into memory
        vm.memory = memory![((1, 1), 5)];
        let ids_data = ids_data!["n"];
        assert!(run_hint!(vm, ids_data, hint_code).is_ok());
    }

    #[test]
    fn memset_enter_scope_invalid() {
        let hint_code = "vm_enter_scope({'n': ids.n})";
        let mut vm = vm!();
        // initialize fp
        vm.run_context.fp = 2;
        // insert ids.n into memory
        // insert a relocatable value in the address of ids.len so that it raises an error.
        vm.memory = memory![((1, 1), (1, 0))];
        let ids_data = ids_data!["n"];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 1))
            )))
        );
    }

    #[test]
    fn memset_continue_loop_valid_continue_loop_equal_1() {
        let hint_code = "n -= 1\nids.continue_loop = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize fp
        vm.run_context.fp = 1;
        // initialize vm scope with variable `n` = 1
        let mut exec_scopes = scope![("n", Felt::one())];
        // initialize ids.continue_loop
        // we create a memory gap so that there is None in (1, 0), the actual addr of continue_loop
        vm.memory = memory![((1, 1), 5)];
        let ids_data = ids_data!["continue_loop"];
        assert!(run_hint!(vm, ids_data, hint_code, &mut exec_scopes).is_ok());
        // assert ids.continue_loop = 0
        check_memory![vm.memory, ((1, 0), 0)];
    }

    #[test]
    fn memset_continue_loop_valid_continue_loop_equal_5() {
        let hint_code = "n -= 1\nids.continue_loop = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize fp
        vm.run_context.fp = 1;
        // initialize vm scope with variable `n` = 5
        let mut exec_scopes = scope![("n", Felt::new(5))];
        // initialize ids.continue_loop
        // we create a memory gap so that there is None in (0, 0), the actual addr of continue_loop
        vm.memory = memory![((1, 2), 5)];
        let ids_data = ids_data!["continue_loop"];
        assert!(run_hint!(vm, ids_data, hint_code, &mut exec_scopes).is_ok());

        // assert ids.continue_loop = 1
        check_memory![vm.memory, ((1, 0), 1)];
    }

    #[test]
    fn memset_continue_loop_variable_not_in_scope_error() {
        let hint_code = "n -= 1\nids.continue_loop = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize fp
        vm.run_context.fp = 3;

        // we don't initialize `n` now:
        /*  vm.exec_scopes
        .assign_or_update_variable("n",  Felt::one()));  */

        // initialize ids.continue_loop
        // we create a memory gap so that there is None in (0, 1), the actual addr of continue_loop
        vm.memory = memory![((1, 2), 5)];
        let ids_data = ids_data!["continue_loop"];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::VariableNotInScopeError("n".to_string()))
        );
    }

    #[test]
    fn memset_continue_loop_insert_error() {
        let hint_code = "n -= 1\nids.continue_loop = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize fp
        vm.run_context.fp = 1;
        // initialize with variable `n`
        let mut exec_scopes = scope![("n", Felt::one())];
        // initialize ids.continue_loop
        // a value is written in the address so the hint cant insert value there
        vm.memory = memory![((1, 0), 5)];
        let ids_data = ids_data!["continue_loop"];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code, &mut exec_scopes),
            Err(HintError::Internal(VirtualMachineError::MemoryError(
                MemoryError::InconsistentMemory(
                    MaybeRelocatable::from((1, 0)),
                    MaybeRelocatable::from(Felt::new(5)),
                    MaybeRelocatable::from(Felt::zero())
                )
            )))
        );
    }
}
