use crate::stdlib::collections::HashMap;
use crate::stdlib::prelude::String;

use crate::{
    hint_processor::hint_processor_definition::HintReference,
    serde::deserialize_program::ApTracking,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};

use super::hint_utils::{get_integer_from_var_name, insert_value_from_var_name};

/// Implements hint:
/// ```python
/// x = ids.x,
/// ids.bit_length = x.bit_length()
/// ```
pub fn get_felt_bitlenght(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let x = get_integer_from_var_name("x", vm, ids_data, ap_tracking)?;
    let bit_length = x.bits() as usize;
    insert_value_from_var_name("bit_length", bit_length, vm, ids_data, ap_tracking)
}

#[cfg(test)]
mod tests {
    use crate::any_box;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::HintProcessorData;
    use crate::hint_processor::hint_processor_definition::HintProcessor;
    use crate::types::exec_scope::ExecutionScopes;
    use crate::{hint_processor::builtin_hint_processor::hint_code, utils::test_utils::*};
    use felt::Felt252;
    use num_traits::{Bounded, One, Zero};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    use super::*;

    fn run_hint(x: Felt252) -> Result<Felt252, HintError> {
        let ids_data = non_continuous_ids_data![
            ("x", 0),          // located at `fp + 0`.
            ("bit_length", 1)  // located at `fp + 1`.
        ];

        let mut vm = vm!();
        vm.run_context.fp = 0;
        add_segments!(vm, 2); // Alloc space for `ids.x` and `ids.bit_length`
        vm.insert_value((1, 0).into(), x).unwrap();

        run_hint!(vm, ids_data, hint_code::GET_FELT_BIT_LENGTH).unwrap();
        Ok(vm.get_integer((1, 1).into()).unwrap().into_owned())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_simple() {
        let bit_length_result = run_hint(Felt252::new(7));
        assert!(bit_length_result.is_ok());
        assert_eq!(bit_length_result.unwrap(), Felt252::from(3));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_in_range() {
        for i in 0..252_usize {
            let x: Felt252 = Felt252::one() << i;

            let bit_length_result = run_hint(x);
            assert!(bit_length_result.is_ok());
            assert_eq!(bit_length_result.unwrap(), Felt252::from(i + 1));
        }
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_wraparound() {
        let x = Felt252::max_value() + Felt252::one();
        let bit_length_result = run_hint(x);
        assert!(bit_length_result.is_ok());
        assert_eq!(bit_length_result.unwrap(), Felt252::zero());
    }
}
