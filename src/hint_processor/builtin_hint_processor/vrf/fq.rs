//! Fq stands for "a finite field of q elements"

use crate::{
    hint_processor::builtin_hint_processor::{uint256_utils::Uint256, uint512_utils::Uint512},
    hint_processor::hint_processor_definition::HintReference,
    serde::deserialize_program::ApTracking,
    stdlib::{collections::HashMap, prelude::*},
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use num_integer::div_rem;

/// Implements hint:
/// ```python
/// def split(num: int, num_bits_shift: int, length: int):
///     a = []
///     for _ in range(length):
///         a.append( num & ((1 << num_bits_shift) - 1) )
///         num = num >> num_bits_shift
///     return tuple(a)
///
/// def pack(z, num_bits_shift: int) -> int:
///     limbs = (z.low, z.high)
///     return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))
///
/// def pack_extended(z, num_bits_shift: int) -> int:
///     limbs = (z.d0, z.d1, z.d2, z.d3)
///     return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))
///
/// x = pack_extended(ids.x, num_bits_shift = 128)
/// div = pack(ids.div, num_bits_shift = 128)
///
/// quotient, remainder = divmod(x, div)
///
/// quotient_split = split(quotient, num_bits_shift=128, length=4)
///
/// ids.quotient.d0 = quotient_split[0]
/// ids.quotient.d1 = quotient_split[1]
/// ids.quotient.d2 = quotient_split[2]
/// ids.quotient.d3 = quotient_split[3]
///
/// remainder_split = split(remainder, num_bits_shift=128, length=2)
/// ids.remainder.low = remainder_split[0]
/// ids.remainder.high = remainder_split[1]
/// ```
pub fn uint512_unsigned_div_rem(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let x = Uint512::from_var_name("x", vm, ids_data, ap_tracking)?.pack();
    let div = Uint256::from_var_name("div", vm, ids_data, ap_tracking)?.pack();

    // Main logic:
    //  quotient, remainder = divmod(x, div)
    let (quotient, remainder) = div_rem(x, div);

    Uint512::from(&quotient).insert_from_var_name("quotient", vm, ids_data, ap_tracking)?;
    Uint256::from(&remainder).insert_from_var_name("remainder", vm, ids_data, ap_tracking)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::any_box;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::HintProcessorData;
    use crate::hint_processor::builtin_hint_processor::hint_code;
    use crate::hint_processor::hint_processor_definition::HintProcessor;
    use crate::types::exec_scope::ExecutionScopes;
    use crate::utils::test_utils::*;
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_uint512_unsigned_div_rem_ok() {
        let hint_code = hint_code::UINT512_UNSIGNED_DIV_REM;
        let mut vm = vm_with_range_check!();

        vm.segments = segments![
            ((1, 0), 2363463),
            ((1, 1), 566795),
            ((1, 2), 8760799),
            ((1, 3), 62362634),
            ((1, 4), 8340843),
            ((1, 5), 124152)
        ];
        // Create hint_data
        let ids_data =
            non_continuous_ids_data![("x", 0), ("div", 4), ("quotient", 6), ("remainder", 10)];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code, exec_scopes_ref!()),
            Ok(())
        );
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            // quotient
            ((1, 6), 158847186690949537631480225217589612243),
            ((1, 7), 105056890940778813909974456334651647691),
            ((1, 8), 502),
            ((1, 9), 0),
            // remainder
            ((1, 10), ("235556430256711128858231095164527378198", 10)),
            ((1, 11), 83573),
        ];
    }
}
