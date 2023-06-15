use core::ops::Shl;

use crate::hint_processor::builtin_hint_processor::uint_utils::{pack, split};
use crate::stdlib::{borrow::Cow, collections::HashMap, prelude::*};
use crate::{
    hint_processor::{
        builtin_hint_processor::{
            hint_utils::{get_relocatable_from_var_name, insert_value_from_var_name},
            secp::secp_utils::{bigint3_split, BASE_86},
        },
        hint_processor_definition::HintReference,
    },
    serde::deserialize_program::ApTracking,
    types::{
        exec_scope::ExecutionScopes,
        relocatable::{MaybeRelocatable, Relocatable},
    },
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use felt::Felt252;
use num_bigint::{BigInt, BigUint};
use num_traits::Bounded;

// Uint384 and BigInt3 are used interchangeably with BigInt3
pub(crate) type Uint384<'a> = BigInt3<'a>;

#[derive(Debug, PartialEq)]
pub(crate) struct BigInt3<'a> {
    pub d0: Cow<'a, Felt252>,
    pub d1: Cow<'a, Felt252>,
    pub d2: Cow<'a, Felt252>,
}

impl BigInt3<'_> {
    pub(crate) fn from_base_addr<'a>(
        addr: Relocatable,
        name: &str,
        vm: &'a VirtualMachine,
    ) -> Result<BigInt3<'a>, HintError> {
        Ok(BigInt3 {
            d0: vm.get_integer(addr).map_err(|_| {
                HintError::IdentifierHasNoMember(name.to_string(), "d0".to_string())
            })?,
            d1: vm.get_integer((addr + 1)?).map_err(|_| {
                HintError::IdentifierHasNoMember(name.to_string(), "d1".to_string())
            })?,
            d2: vm.get_integer((addr + 2)?).map_err(|_| {
                HintError::IdentifierHasNoMember(name.to_string(), "d2".to_string())
            })?,
        })
    }

    pub(crate) fn from_var_name<'a>(
        name: &str,
        vm: &'a VirtualMachine,
        ids_data: &HashMap<String, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Result<BigInt3<'a>, HintError> {
        let base_addr = get_relocatable_from_var_name(name, vm, ids_data, ap_tracking)?;
        BigInt3::from_base_addr(base_addr, name, vm)
    }

    pub(crate) fn from_values(limbs: [Felt252; 3]) -> Self {
        let [d0, d1, d2] = limbs;
        let d0 = Cow::Owned(d0);
        let d1 = Cow::Owned(d1);
        let d2 = Cow::Owned(d2);
        Self { d0, d1, d2 }
    }

    pub(crate) fn insert_from_var_name(
        self,
        var_name: &str,
        vm: &mut VirtualMachine,
        ids_data: &HashMap<String, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Result<(), HintError> {
        let addr = get_relocatable_from_var_name(var_name, vm, ids_data, ap_tracking)?;

        vm.insert_value(addr, self.d0.into_owned())?;
        vm.insert_value((addr + 1)?, self.d1.into_owned())?;
        vm.insert_value((addr + 2)?, self.d2.into_owned())?;

        Ok(())
    }

    pub(crate) fn pack(self) -> BigUint {
        pack([self.d0, self.d1, self.d2], 128)
    }

    pub(crate) fn pack86(self) -> BigInt {
        let limbs = [self.d0, self.d1, self.d2];
        limbs
            .into_iter()
            .enumerate()
            .map(|(idx, value)| value.to_bigint().shl(idx * 86))
            .sum()
    }

    pub(crate) fn split(num: &BigUint) -> Self {
        let limbs = split(num, 128);
        Self::from_values(limbs)
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct BigInt5<'a> {
    pub d0: Cow<'a, Felt252>,
    pub d1: Cow<'a, Felt252>,
    pub d2: Cow<'a, Felt252>,
    pub d3: Cow<'a, Felt252>,
    pub d4: Cow<'a, Felt252>,
}

impl BigInt5<'_> {
    pub(crate) fn from_base_addr<'a>(
        addr: Relocatable,
        name: &str,
        vm: &'a VirtualMachine,
    ) -> Result<BigInt5<'a>, HintError> {
        Ok(BigInt5 {
            d0: vm.get_integer(addr).map_err(|_| {
                HintError::IdentifierHasNoMember(name.to_string(), "d0".to_string())
            })?,
            d1: vm.get_integer((addr + 1)?).map_err(|_| {
                HintError::IdentifierHasNoMember(name.to_string(), "d1".to_string())
            })?,
            d2: vm.get_integer((addr + 2)?).map_err(|_| {
                HintError::IdentifierHasNoMember(name.to_string(), "d2".to_string())
            })?,
            d3: vm.get_integer((addr + 3)?).map_err(|_| {
                HintError::IdentifierHasNoMember(name.to_string(), "d3".to_string())
            })?,
            d4: vm.get_integer((addr + 4)?).map_err(|_| {
                HintError::IdentifierHasNoMember(name.to_string(), "d4".to_string())
            })?,
        })
    }

    pub(crate) fn from_var_name<'a>(
        name: &str,
        vm: &'a VirtualMachine,
        ids_data: &HashMap<String, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Result<BigInt5<'a>, HintError> {
        let base_addr = get_relocatable_from_var_name(name, vm, ids_data, ap_tracking)?;
        BigInt5::from_base_addr(base_addr, name, vm)
    }
}
/*
Implements hint:
%{
    from starkware.cairo.common.cairo_secp.secp_utils import split

    segments.write_arg(ids.res.address_, split(value))
%}
*/
pub fn nondet_bigint3(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let res_reloc = get_relocatable_from_var_name("res", vm, ids_data, ap_tracking)?;
    let value = exec_scopes
        .get_ref::<num_bigint::BigInt>("value")?
        .to_biguint()
        .ok_or(HintError::BigIntToBigUintFail)?;
    let arg: Vec<MaybeRelocatable> = bigint3_split(&value)?
        .into_iter()
        .map(|n| MaybeRelocatable::from(Felt252::new(n)))
        .collect();
    vm.write_arg(res_reloc, &arg).map_err(HintError::Memory)?;
    Ok(())
}

// Implements hint
// %{ ids.low = (ids.x.d0 + ids.x.d1 * ids.BASE) & ((1 << 128) - 1) %}
pub fn bigint_to_uint256(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    let x_struct = get_relocatable_from_var_name("x", vm, ids_data, ap_tracking)?;
    let d0 = vm.get_integer(x_struct)?;
    let d1 = vm.get_integer((x_struct + 1_i32)?)?;
    let d0 = d0.as_ref();
    let d1 = d1.as_ref();
    let base_86 = constants
        .get(BASE_86)
        .ok_or(HintError::MissingConstant(BASE_86))?;
    let low = (d0 + &(d1 * base_86)) & &Felt252::new(u128::MAX);
    insert_value_from_var_name("low", low, vm, ids_data, ap_tracking)
}

// Implements hint
// %{ ids.len_hi = max(ids.scalar_u.d2.bit_length(), ids.scalar_v.d2.bit_length())-1 %}
pub fn hi_max_bitlen(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let scalar_u = BigInt3::from_var_name("scalar_u", vm, ids_data, ap_tracking)?;
    let scalar_v = BigInt3::from_var_name("scalar_v", vm, ids_data, ap_tracking)?;

    let len_hi_u = scalar_u.d2.bits();
    let len_hi_v = scalar_v.d2.bits();

    let len_hi = len_hi_u.max(len_hi_v);

    // equal to `len_hi.wrapping_sub(1)`
    let res = if len_hi == 0 {
        Felt252::max_value()
    } else {
        (len_hi - 1).into()
    };

    insert_value_from_var_name("len_hi", res, vm, ids_data, ap_tracking)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::any_box;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::{
        BuiltinHintProcessor, HintProcessorData,
    };
    use crate::hint_processor::hint_processor_definition::HintProcessor;
    use crate::stdlib::ops::Shl;
    use crate::stdlib::string::ToString;
    use crate::types::exec_scope::ExecutionScopes;

    use crate::types::relocatable::Relocatable;
    use crate::utils::test_utils::*;

    use crate::vm::vm_core::VirtualMachine;

    use assert_matches::assert_matches;
    use num_traits::One;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_nondet_bigint3_ok() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import split\n\nsegments.write_arg(ids.res.address_, split(value))";
        let mut vm = vm_with_range_check!();
        add_segments!(vm, 3);
        // initialize vm scope with variable `n`
        let mut exec_scopes = scope![(
            "value",
            bigint_str!("7737125245533626718119526477371252455336267181195264773712524553362")
        )];
        //Initialize RubContext
        run_context!(vm, 0, 6, 6);
        //Create hint_data
        let ids_data = non_continuous_ids_data![("res", 5)];
        assert_matches!(
            run_hint!(
                vm,
                ids_data,
                hint_code,
                &mut exec_scopes,
                &[(BASE_86, Felt252::one().shl(86_u32))]
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v))
                    .collect()
            ),
            Ok(())
        );
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            ((1, 11), 773712524553362_u64),
            ((1, 12), 57408430697461422066401280_u128),
            ((1, 13), 1292469707114105_u64)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_nondet_bigint3_value_not_in_scope() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import split\n\nsegments.write_arg(ids.res.address_, split(value))";
        let mut vm = vm_with_range_check!();
        // we don't initialize `value` now:
        //Initialize RubContext
        run_context!(vm, 0, 6, 6);
        //Create hint_data
        let ids_data = non_continuous_ids_data![("res", 5)];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::VariableNotInScopeError(x)) if x == "value"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_nondet_bigint3_split_error() {
        let hint_code = "from starkware.cairo.common.cairo_secp.secp_utils import split\n\nsegments.write_arg(ids.res.address_, split(value))";
        let mut vm = vm_with_range_check!();

        // initialize vm scope with variable `n`
        let mut exec_scopes = scope![("value", bigint!(-1))];
        //Create hint_data
        let ids_data = non_continuous_ids_data![("res", 5)];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code, &mut exec_scopes),
            Err(HintError::BigIntToBigUintFail)
        );
    }

    #[test]
    fn get_bigint3_from_base_addr_ok() {
        //BigInt3(1,2,3)
        let mut vm = vm!();
        vm.segments = segments![((0, 0), 1), ((0, 1), 2), ((0, 2), 3)];
        let x = BigInt3::from_base_addr((0, 0).into(), "x", &vm).unwrap();
        assert_eq!(x.d0.as_ref(), &Felt252::one());
        assert_eq!(x.d1.as_ref(), &Felt252::from(2));
        assert_eq!(x.d2.as_ref(), &Felt252::from(3));
    }

    #[test]
    fn get_bigint5_from_base_addr_ok() {
        //BigInt3(1,2,3, 4, 5)
        let mut vm = vm!();
        vm.segments = segments![
            ((0, 0), 1),
            ((0, 1), 2),
            ((0, 2), 3),
            ((0, 3), 4),
            ((0, 4), 5)
        ];
        let x = BigInt5::from_base_addr((0, 0).into(), "x", &vm).unwrap();
        assert_eq!(x.d0.as_ref(), &Felt252::one());
        assert_eq!(x.d1.as_ref(), &Felt252::from(2));
        assert_eq!(x.d2.as_ref(), &Felt252::from(3));
        assert_eq!(x.d3.as_ref(), &Felt252::from(4));
        assert_eq!(x.d4.as_ref(), &Felt252::from(5));
    }

    #[test]
    fn get_bigint3_from_base_addr_missing_member() {
        //BigInt3(1,2,x)
        let mut vm = vm!();
        vm.segments = segments![((0, 0), 1), ((0, 1), 2)];
        let r = BigInt3::from_base_addr((0, 0).into(), "x", &vm);
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d2")
    }

    #[test]
    fn get_bigint5_from_base_addr_missing_member() {
        let mut vm = vm!();
        vm.segments = segments![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4),];
        let r = BigInt5::from_base_addr((0, 0).into(), "x", &vm);
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d4")
    }

    #[test]
    fn get_bigint3_from_var_name_ok() {
        //BigInt3(1,2,3)
        let mut vm = vm!();
        vm.set_fp(1);
        vm.segments = segments![((1, 0), 1), ((1, 1), 2), ((1, 2), 3)];
        let ids_data = ids_data!["x"];
        let x = BigInt3::from_var_name("x", &vm, &ids_data, &ApTracking::default()).unwrap();
        assert_eq!(x.d0.as_ref(), &Felt252::one());
        assert_eq!(x.d1.as_ref(), &Felt252::from(2));
        assert_eq!(x.d2.as_ref(), &Felt252::from(3));
    }

    #[test]
    fn get_bigint5_from_var_name_ok() {
        // BigInt5(1,2,3,4,5)
        let mut vm = vm!();
        vm.set_fp(1);
        vm.segments = segments![
            ((1, 0), 1),
            ((1, 1), 2),
            ((1, 2), 3),
            ((1, 3), 4),
            ((1, 4), 5)
        ];
        let ids_data = ids_data!["x"];
        let x = BigInt5::from_var_name("x", &vm, &ids_data, &ApTracking::default()).unwrap();
        assert_eq!(x.d0.as_ref(), &Felt252::one());
        assert_eq!(x.d1.as_ref(), &Felt252::from(2));
        assert_eq!(x.d2.as_ref(), &Felt252::from(3));
        assert_eq!(x.d3.as_ref(), &Felt252::from(4));
        assert_eq!(x.d4.as_ref(), &Felt252::from(5));
    }

    #[test]
    fn get_bigint3_from_var_name_missing_member() {
        //BigInt3(1,2,x)
        let mut vm = vm!();
        vm.set_fp(1);
        vm.segments = segments![((1, 0), 1), ((1, 1), 2)];
        let ids_data = ids_data!["x"];
        let r = BigInt3::from_var_name("x", &vm, &ids_data, &ApTracking::default());
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d2")
    }

    #[test]
    fn get_bigint5_from_var_name_missing_member() {
        //BigInt5(1,2,3,4,x)
        let mut vm = vm!();
        vm.set_fp(1);
        vm.segments = segments![((1, 0), 1), ((1, 1), 2), ((1, 2), 3), ((1, 3), 4)];
        let ids_data = ids_data!["x"];
        let r = BigInt5::from_var_name("x", &vm, &ids_data, &ApTracking::default());
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d4")
    }

    #[test]
    fn get_bigint3_from_var_name_invalid_reference() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), 1), ((1, 1), 2), ((1, 2), 3)];
        let ids_data = ids_data!["x"];
        let r = BigInt3::from_var_name("x", &vm, &ids_data, &ApTracking::default());
        assert_matches!(r, Err(HintError::UnknownIdentifier(x)) if x == "x")
    }

    #[test]
    fn get_bigint5_from_var_name_invalid_reference() {
        let mut vm = vm!();
        // Will fail because fp was not set to 1.
        vm.segments = segments![
            ((1, 0), 1),
            ((1, 1), 2),
            ((1, 2), 3),
            ((1, 3), 4),
            ((1, 4), 5)
        ];
        let ids_data = ids_data!["x"];
        let r = BigInt5::from_var_name("x", &vm, &ids_data, &ApTracking::default());
        assert_matches!(r, Err(HintError::UnknownIdentifier(x)) if x == "x")
    }

    #[test]
    fn run_hi_max_bitlen_ok() {
        let hint_code =
            "ids.len_hi = max(ids.scalar_u.d2.bit_length(), ids.scalar_v.d2.bit_length())-1";

        let mut vm = vm_with_range_check!();

        // Initialize RunContext
        run_context!(vm, 0, 7, 0);

        vm.segments = segments![
            ((1, 0), 0),
            ((1, 1), 0),
            ((1, 2), 1),
            ((1, 3), 0),
            ((1, 4), 0),
            ((1, 5), 1)
        ];
        // Create hint_data
        let ids_data = non_continuous_ids_data![("scalar_u", 0), ("scalar_v", 3), ("len_hi", 6)];
        assert!(run_hint!(vm, ids_data, hint_code, exec_scopes_ref!()).is_ok());
        //Check hint memory inserts
        check_memory![vm.segments.memory, ((1, 6), 0)];
    }
}
