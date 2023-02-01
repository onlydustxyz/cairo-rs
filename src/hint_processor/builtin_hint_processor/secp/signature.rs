use std::prelude::v1::*;

use crate::{
    hint_processor::{
        builtin_hint_processor::{
            hint_utils::get_integer_from_var_name,
            secp::secp_utils::{pack_from_var_name, BASE_86, BETA, N0, N1, N2, SECP_REM},
        },
        hint_processor_definition::HintReference,
    },
    math_utils::{div_mod, safe_div_bigint},
    serde::deserialize_program::ApTracking,
    types::exec_scope::ExecutionScopes,
    vm::errors::hint_errors::HintError,
    vm::vm_core::VirtualMachine,
};
use felt::Felt;
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::One;
use std::{
    collections::HashMap,
    ops::{Shl, Shr},
};

/* Implements hint:
from starkware.cairo.common.cairo_secp.secp_utils import N, pack
from starkware.python.math_utils import div_mod, safe_div

a = pack(ids.a, PRIME)
b = pack(ids.b, PRIME)
value = res = div_mod(a, b, N)
*/
pub fn div_mod_n_packed_divmod(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let a = pack_from_var_name("a", vm, ids_data, ap_tracking)?;
    let b = pack_from_var_name("b", vm, ids_data, ap_tracking)?;

    let n = {
        let base = constants
            .get(BASE_86)
            .ok_or(HintError::MissingConstant(BASE_86))?
            .to_bigint();
        let n0 = constants
            .get(N0)
            .ok_or(HintError::MissingConstant(N0))?
            .to_bigint();
        let n1 = constants
            .get(N1)
            .ok_or(HintError::MissingConstant(N1))?
            .to_bigint();
        let n2 = constants
            .get(N2)
            .ok_or(HintError::MissingConstant(N2))?
            .to_bigint();

        (n2 * &base * &base) | (n1 * base) | n0
    };

    let value = div_mod(&a, &b, &n);
    exec_scopes.insert_value("a", a);
    exec_scopes.insert_value("b", b);
    exec_scopes.insert_value("value", value.clone());
    exec_scopes.insert_value("res", value);
    Ok(())
}

// Implements hint:
// value = k = safe_div(res * b - a, N)
pub fn div_mod_n_safe_div(
    exec_scopes: &mut ExecutionScopes,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let a = exec_scopes.get_ref::<BigInt>("a")?;
    let b = exec_scopes.get_ref::<BigInt>("b")?;
    let res = exec_scopes.get_ref::<BigInt>("res")?;

    let n = {
        let base = constants
            .get(BASE_86)
            .ok_or(HintError::MissingConstant(BASE_86))?
            .to_bigint();
        let n0 = constants
            .get(N0)
            .ok_or(HintError::MissingConstant(N0))?
            .to_bigint();
        let n1 = constants
            .get(N1)
            .ok_or(HintError::MissingConstant(N1))?
            .to_bigint();
        let n2 = constants
            .get(N2)
            .ok_or(HintError::MissingConstant(N2))?
            .to_bigint();

        n2 * &base * &base + n1 * base + n0
    };

    let value = safe_div_bigint(&(res * b - a), &n)?;

    exec_scopes.insert_value("value", value);
    Ok(())
}

pub fn get_point_from_x(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt>,
) -> Result<(), HintError> {
    let beta = constants
        .get(BETA)
        .ok_or(HintError::MissingConstant(BETA))?
        .to_bigint();
    let secp_p = BigInt::one().shl(256_u32)
        - constants
            .get(SECP_REM)
            .ok_or(HintError::MissingConstant(SECP_REM))?
            .to_bigint();

    let x_cube_int = pack_from_var_name("x_cube", vm, ids_data, ap_tracking)?.mod_floor(&secp_p);
    //.mod_floor(&BigInt::from_biguint(num_bigint::Sign::Plus, secp_p.clone()))
    //.to_biguint().ok_or(VirtualMachineError::BigIntToBigUintFail)?;
    let y_cube_int = (x_cube_int + beta).mod_floor(&secp_p);
    // Divide by 4
    let mut y = y_cube_int.modpow(&(&secp_p + 1_u32).shr(2_u32), &secp_p);

    let v = get_integer_from_var_name("v", vm, ids_data, ap_tracking)?.to_biguint();
    if v.is_even() != y.is_even() {
        y = &secp_p - y;
    }
    exec_scopes.insert_value("value", y);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::{
                builtin_hint_processor_definition::{BuiltinHintProcessor, HintProcessorData},
                hint_code,
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
    use num_traits::Zero;
    use std::{any::Any, ops::Shl};

    #[test]
    fn safe_div_ok() {
        let hint_code = hint_code::DIV_MOD_N_PACKED_DIVMOD;
        let mut vm = vm!();

        vm.memory = memory![
            ((1, 0), 15),
            ((1, 1), 3),
            ((1, 2), 40),
            ((1, 3), 0),
            ((1, 4), 10),
            ((1, 5), 1)
        ];
        vm.run_context.fp = 3;
        let ids_data = non_continuous_ids_data![("a", -3), ("b", 0)];
        let mut exec_scopes = ExecutionScopes::new();
        let constants = [
            (BASE_86, Felt::one().shl(86_u32)),
            (N0, Felt::new(10428087374290690730508609u128)),
            (N1, Felt::new(77371252455330678278691517u128)),
            (N2, Felt::new(19342813113834066795298815u128)),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();
        assert_eq!(
            run_hint!(vm, ids_data, hint_code, &mut exec_scopes, &constants),
            Ok(())
        );
        assert_eq!(div_mod_n_safe_div(&mut exec_scopes, &constants), Ok(()));
    }

    #[test]
    fn safe_div_fail() {
        let mut exec_scopes = scope![
            ("a", BigInt::zero()),
            ("b", BigInt::one()),
            ("res", BigInt::one())
        ];
        assert_eq!(
            Err(
                HintError::Internal(VirtualMachineError::SafeDivFailBigInt(
                    BigInt::one(),
                    bigint_str!("115792089237316195423570985008687907852837564279074904382605163141518161494337"),
                )
            )),
            div_mod_n_safe_div(
                &mut exec_scopes,
                &[
                    (BASE_86, Felt::one().shl(86_u32)),
                    (N0, Felt::new(10428087374290690730508609u128)),
                    (N1, Felt::new(77371252455330678278691517u128)),
                    (N2, Felt::new(19342813113834066795298815u128)),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
        );
    }

    #[test]
    fn get_point_from_x_ok() {
        let hint_code = hint_code::GET_POINT_FROM_X;
        let mut vm = vm!();
        vm.memory = memory![
            ((1, 0), 18),
            ((1, 1), 2147483647),
            ((1, 2), 2147483647),
            ((1, 3), 2147483647)
        ];
        vm.run_context.fp = 1;
        let ids_data = non_continuous_ids_data![("v", -1), ("x_cube", 0)];
        assert_eq!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                exec_scopes_ref!(),
                &[
                    (BETA, Felt::new(7)),
                    (
                        SECP_REM,
                        Felt::one().shl(32_u32)
                            + Felt::one().shl(9_u32)
                            + Felt::one().shl(8_u32)
                            + Felt::one().shl(7_u32)
                            + Felt::one().shl(6_u32)
                            + Felt::one().shl(4_u32)
                            + Felt::one()
                    ),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        )
    }

    #[test]
    fn get_point_from_x_negative_y() {
        let hint_code = hint_code::GET_POINT_FROM_X;
        let mut vm = vm!();
        let mut exec_scopes = ExecutionScopes::new();
        vm.memory = memory![
            ((1, 0), 1),
            ((1, 1), 2147483647),
            ((1, 2), 2147483647),
            ((1, 3), 2147483647)
        ];
        vm.run_context.fp = 2;

        let ids_data = ids_data!["v", "x_cube"];
        assert_eq!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[
                    (BETA, Felt::new(7)),
                    (
                        SECP_REM,
                        Felt::one().shl(32_u32)
                            + Felt::one().shl(9_u32)
                            + Felt::one().shl(8_u32)
                            + Felt::one().shl(7_u32)
                            + Felt::one().shl(6_u32)
                            + Felt::one().shl(4_u32)
                            + Felt::one()
                    ),
                ]
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect()
            ),
            Ok(())
        );

        check_scope!(
            &exec_scopes,
            [(
                "value",
                bigint_str!(
                    "94274691440067846579164151740284923997007081248613730142069408045642476712539"
                )
            )]
        );
    }
}
