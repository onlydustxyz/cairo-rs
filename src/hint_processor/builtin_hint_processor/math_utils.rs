use crate::stdlib::{
    collections::HashMap,
    ops::{Shl, Shr},
    prelude::*,
};
use num_traits::{Bounded, Pow};

use crate::utils::CAIRO_PRIME;

use crate::{
    any_box,
    hint_processor::{
        builtin_hint_processor::hint_utils::{
            get_integer_from_var_name, get_ptr_from_var_name, insert_value_from_var_name,
            insert_value_into_ap,
        },
        hint_processor_definition::HintReference,
    },
    math_utils::isqrt,
    serde::deserialize_program::ApTracking,
    types::{exec_scope::ExecutionScopes, relocatable::MaybeRelocatable},
    vm::{
        errors::{hint_errors::HintError, vm_errors::VirtualMachineError},
        vm_core::VirtualMachine,
    },
};
use felt::Felt252;
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::One;
use num_traits::{Signed, Zero};

use super::hint_utils::get_maybe_relocatable_from_var_name;

//Implements hint: memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1
pub fn is_nn(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a = get_integer_from_var_name("a", vm, ids_data, ap_tracking)?;
    let range_check_builtin = vm.get_range_check_builtin()?;
    //Main logic (assert a is not negative and within the expected range)
    let value = match &range_check_builtin._bound {
        Some(bound) if a.as_ref() >= bound => Felt252::one(),
        _ => Felt252::zero(),
    };
    insert_value_into_ap(vm, value)
}

//Implements hint: memory[ap] = 0 if 0 <= ((-ids.a - 1) % PRIME) < range_check_builtin.bound else 1
pub fn is_nn_out_of_range(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a = get_integer_from_var_name("a", vm, ids_data, ap_tracking)?;
    let a = a.as_ref();
    let range_check_builtin = vm.get_range_check_builtin()?;
    //Main logic (assert a is not negative and within the expected range)
    //let value = if (-a - 1usize).mod_floor(vm.get_prime()) < range_check_builtin._bound {
    let value = match &range_check_builtin._bound {
        Some(bound) if Felt252::zero() - (a + 1) < *bound => Felt252::zero(),
        None => Felt252::zero(),
        _ => Felt252::one(),
    };
    insert_value_into_ap(vm, value)
}
/* Implements hint:from starkware.cairo.common.math_utils import assert_integer
%{
    import itertools

    from starkware.cairo.common.math_utils import assert_integer
    assert_integer(ids.a)
    assert_integer(ids.b)
    a = ids.a % PRIME
    b = ids.b % PRIME
    assert a <= b, f'a = {a} is not less than or equal to b = {b}.'

    # Find an arc less than PRIME / 3, and another less than PRIME / 2.
    lengths_and_indices = [(a, 0), (b - a, 1), (PRIME - 1 - b, 2)]
    lengths_and_indices.sort()
    assert lengths_and_indices[0][0] <= PRIME // 3 and lengths_and_indices[1][0] <= PRIME // 2
    excluded = lengths_and_indices[2][1]

    memory[ids.range_check_ptr + 1], memory[ids.range_check_ptr + 0] = (
        divmod(lengths_and_indices[0][0], ids.PRIME_OVER_3_HIGH))
    memory[ids.range_check_ptr + 3], memory[ids.range_check_ptr + 2] = (
        divmod(lengths_and_indices[1][0], ids.PRIME_OVER_2_HIGH))
%}
*/
pub fn assert_le_felt(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
    constants: &HashMap<String, Felt252>,
) -> Result<(), HintError> {
    const PRIME_OVER_3_HIGH: &str = "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_3_HIGH";
    const PRIME_OVER_2_HIGH: &str = "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_2_HIGH";

    let prime_over_3_high = constants
        .get(PRIME_OVER_3_HIGH)
        .ok_or(HintError::MissingConstant(PRIME_OVER_3_HIGH))?;
    let prime_over_2_high = constants
        .get(PRIME_OVER_2_HIGH)
        .ok_or(HintError::MissingConstant(PRIME_OVER_2_HIGH))?;
    let a = &get_integer_from_var_name("a", vm, ids_data, ap_tracking)?
        .clone()
        .into_owned();
    let b = &get_integer_from_var_name("b", vm, ids_data, ap_tracking)?
        .clone()
        .into_owned();
    let range_check_ptr = get_ptr_from_var_name("range_check_ptr", vm, ids_data, ap_tracking)?;

    if a > b {
        return Err(HintError::NonLeFelt252(a.clone(), b.clone()));
    }

    let arc1 = b - a;
    let arc2 = Felt252::zero() - Felt252::one() - b;
    let mut lengths_and_indices = vec![(a, 0_i32), (&arc1, 1_i32), (&arc2, 2_i32)];
    lengths_and_indices.sort();
    if lengths_and_indices[0].0 > &div_prime_by_bound(Felt252::new(3_i32))?
        || lengths_and_indices[1].0 > &div_prime_by_bound(Felt252::new(2_i32))?
    {
        return Err(HintError::ArcTooBig(
            lengths_and_indices[0].0.clone(),
            div_prime_by_bound(Felt252::new(3_i32))?,
            lengths_and_indices[1].0.clone(),
            div_prime_by_bound(Felt252::new(3_i32))?,
        ));
    }

    let excluded = lengths_and_indices[2].1;
    exec_scopes.assign_or_update_variable("excluded", any_box!(Felt252::new(excluded)));

    let (q_0, r_0) = (lengths_and_indices[0].0).div_mod_floor(prime_over_3_high);
    let (q_1, r_1) = (lengths_and_indices[1].0).div_mod_floor(prime_over_2_high);

    vm.insert_value((range_check_ptr + 1_i32)?, q_0)?;
    vm.insert_value(range_check_ptr, r_0)?;
    vm.insert_value((range_check_ptr + 3_i32)?, q_1)?;
    vm.insert_value((range_check_ptr + 2_i32)?, r_1)?;
    Ok(())
}

pub fn assert_le_felt_excluded_2(exec_scopes: &mut ExecutionScopes) -> Result<(), HintError> {
    let excluded: Felt252 = exec_scopes.get("excluded")?;

    if excluded != Felt252::new(2_i32) {
        Err(HintError::ExcludedNot2(excluded))
    } else {
        Ok(())
    }
}

pub fn assert_le_felt_excluded_1(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
) -> Result<(), HintError> {
    let excluded: Felt252 = exec_scopes.get("excluded")?;

    if excluded != Felt252::one() {
        insert_value_into_ap(vm, &Felt252::one())
    } else {
        insert_value_into_ap(vm, &Felt252::zero())
    }
}

pub fn assert_le_felt_excluded_0(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
) -> Result<(), HintError> {
    let excluded: Felt252 = exec_scopes.get("excluded")?;

    if !excluded.is_zero() {
        insert_value_into_ap(vm, Felt252::one())
    } else {
        insert_value_into_ap(vm, Felt252::zero())
    }
}

//Implements hint:from starkware.cairo.common.math_cmp import is_le_felt
//    memory[ap] = 0 if (ids.a % PRIME) <= (ids.b % PRIME) else 1
pub fn is_le_felt(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a_mod = get_integer_from_var_name("a", vm, ids_data, ap_tracking)?;
    let b_mod = get_integer_from_var_name("b", vm, ids_data, ap_tracking)?;
    let value = if a_mod > b_mod {
        Felt252::one()
    } else {
        Felt252::zero()
    };
    insert_value_into_ap(vm, value)
}

//Implements hint: from starkware.cairo.lang.vm.relocatable import RelocatableValue
//        both_ints = isinstance(ids.a, int) and isinstance(ids.b, int)
//        both_relocatable = (
//            isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and
//            ids.a.segment_index == ids.b.segment_index)
//        assert both_ints or both_relocatable, \
//            f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'
//        assert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'
pub fn assert_not_equal(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let maybe_rel_a = get_maybe_relocatable_from_var_name("a", vm, ids_data, ap_tracking)?;
    let maybe_rel_b = get_maybe_relocatable_from_var_name("b", vm, ids_data, ap_tracking)?;
    match (maybe_rel_a, maybe_rel_b) {
        (MaybeRelocatable::Int(a), MaybeRelocatable::Int(b)) => {
            if (&a - &b).is_zero() {
                return Err(HintError::AssertNotEqualFail(
                    MaybeRelocatable::Int(a),
                    MaybeRelocatable::Int(b),
                ));
            };
            Ok(())
        }
        (MaybeRelocatable::RelocatableValue(a), MaybeRelocatable::RelocatableValue(b)) => {
            if a.segment_index != b.segment_index {
                Err(VirtualMachineError::DiffIndexComp(a, b))?;
            };
            if a.offset == b.offset {
                return Err(HintError::AssertNotEqualFail(
                    MaybeRelocatable::RelocatableValue(a),
                    MaybeRelocatable::RelocatableValue(b),
                ));
            };
            Ok(())
        }
        (a, b) => Err(VirtualMachineError::DiffTypeComparison(a, b))?,
    }
}

//Implements hint:
// %{
//     from starkware.cairo.common.math_utils import assert_integer
//     assert_integer(ids.a)
//     assert 0 <= ids.a % PRIME < range_check_builtin.bound, f'a = {ids.a} is out of range.'
// %}
pub fn assert_nn(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a = get_integer_from_var_name("a", vm, ids_data, ap_tracking)?;
    let range_check_builtin = vm.get_range_check_builtin()?;
    // assert 0 <= ids.a % PRIME < range_check_builtin.bound
    // as prime > 0, a % prime will always be > 0
    match &range_check_builtin._bound {
        Some(bound) if a.as_ref() >= bound => {
            Err(HintError::AssertNNValueOutOfRange(a.into_owned()))
        }
        _ => Ok(()),
    }
}

//Implements hint:from starkware.cairo.common.math.cairo
// %{
// from starkware.cairo.common.math_utils import assert_integer
// assert_integer(ids.value)
// assert ids.value % PRIME != 0, f'assert_not_zero failed: {ids.value} = 0.'
// %}
pub fn assert_not_zero(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let value = get_integer_from_var_name("value", vm, ids_data, ap_tracking)?;
    if value.is_zero() {
        return Err(HintError::AssertNotZero(
            value.into_owned(),
            felt::PRIME_STR.to_string(),
        ));
    };
    Ok(())
}

//Implements hint: assert ids.value == 0, 'split_int(): value is out of range.'
pub fn split_int_assert_range(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let value = get_integer_from_var_name("value", vm, ids_data, ap_tracking)?;
    //Main logic (assert value == 0)
    if !value.is_zero() {
        return Err(HintError::SplitIntNotZero);
    }
    Ok(())
}

//Implements hint: memory[ids.output] = res = (int(ids.value) % PRIME) % ids.base
//        assert res < ids.bound, f'split_int(): Limb {res} is out of range.'
pub fn split_int(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let value = get_integer_from_var_name("value", vm, ids_data, ap_tracking)?;
    let base = get_integer_from_var_name("base", vm, ids_data, ap_tracking)?;
    let bound = get_integer_from_var_name("bound", vm, ids_data, ap_tracking)?;
    let base = base.as_ref();
    let bound = bound.as_ref();
    let output = get_ptr_from_var_name("output", vm, ids_data, ap_tracking)?;
    //Main Logic
    let res = value.mod_floor(base);
    if &res > bound {
        return Err(HintError::SplitIntLimbOutOfRange(res));
    }
    vm.insert_value(output, res).map_err(HintError::Memory)
}

//from starkware.cairo.common.math_utils import is_positive
//ids.is_positive = 1 if is_positive(
//    value=ids.value, prime=PRIME, rc_bound=range_check_builtin.bound) else 0
pub fn is_positive(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let value = get_integer_from_var_name("value", vm, ids_data, ap_tracking)?;
    let range_check_builtin = vm.get_range_check_builtin()?;
    //Main logic (assert a is positive)
    match &range_check_builtin._bound {
        Some(bound) if &value.abs() > bound => {
            return Err(HintError::ValueOutsideValidRange(value.into_owned()))
        }
        _ => {}
    };

    let result = if value.is_positive() {
        Felt252::one()
    } else {
        Felt252::zero()
    };
    insert_value_from_var_name("is_positive", result, vm, ids_data, ap_tracking)
}

//Implements hint:
// %{
//     from starkware.cairo.common.math_utils import assert_integer
//     assert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128
//     assert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW
//     assert_integer(ids.value)
//     ids.low = ids.value & ((1 << 128) - 1)
//     ids.high = ids.value >> 128
// %}
pub fn split_felt(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let value = get_integer_from_var_name("value", vm, ids_data, ap_tracking)?;
    let value = value.as_ref();
    //Main logic
    //assert_integer(ids.value) (done by match)
    // ids.low = ids.value & ((1 << 128) - 1)
    // ids.high = ids.value >> 128
    let low: Felt252 = value & ((Felt252::one().shl(128_u32)) - Felt252::one());
    let high: Felt252 = value.shr(128_u32);
    insert_value_from_var_name("high", high, vm, ids_data, ap_tracking)?;
    insert_value_from_var_name("low", low, vm, ids_data, ap_tracking)
}

//Implements hint: from starkware.python.math_utils import isqrt
//        value = ids.value % PRIME
//        assert value < 2 ** 250, f"value={value} is outside of the range [0, 2**250)."
//        assert 2 ** 250 < PRIME
//        ids.root = isqrt(value)
pub fn sqrt(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let mod_value = get_integer_from_var_name("value", vm, ids_data, ap_tracking)?;
    //This is equal to mod_value > Felt252::new(2).pow(250)
    if mod_value.as_ref().shr(250_u32).is_positive() {
        return Err(HintError::ValueOutside250BitRange(mod_value.into_owned()));
        //This is equal to mod_value > bigint!(2).pow(250)
    }
    #[allow(deprecated)]
    insert_value_from_var_name(
        "root",
        Felt252::new(isqrt(&mod_value.to_biguint())?),
        vm,
        ids_data,
        ap_tracking,
    )
}

pub fn signed_div_rem(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let div = get_integer_from_var_name("div", vm, ids_data, ap_tracking)?;
    let value = get_integer_from_var_name("value", vm, ids_data, ap_tracking)?;
    let value = value.as_ref();
    let bound = get_integer_from_var_name("bound", vm, ids_data, ap_tracking)?;
    let builtin = vm.get_range_check_builtin()?;

    match &builtin._bound {
        Some(builtin_bound)
            if div.is_zero() || div.as_ref() > &div_prime_by_bound(builtin_bound.clone())? =>
        {
            return Err(HintError::OutOfValidRange(
                div.into_owned(),
                builtin_bound.clone(),
            ));
        }
        Some(builtin_bound) if bound.as_ref() > &builtin_bound.shr(1) => {
            return Err(HintError::OutOfValidRange(
                bound.into_owned(),
                builtin_bound.shr(1),
            ));
        }
        None if div.is_zero() => {
            return Err(HintError::OutOfValidRange(
                div.into_owned(),
                Felt252::zero() - Felt252::one(),
            ));
        }
        _ => {}
    }

    #[allow(deprecated)]
    let int_value = value.to_bigint();
    #[allow(deprecated)]
    let int_div = div.to_bigint();
    #[allow(deprecated)]
    let int_bound = bound.to_bigint();
    let (q, r) = int_value.div_mod_floor(&int_div);

    if int_bound.abs() < q.abs() {
        return Err(HintError::OutOfValidRange(
            Felt252::new(q),
            bound.into_owned(),
        ));
    }

    let biased_q = q + int_bound;
    insert_value_from_var_name("r", Felt252::new(r), vm, ids_data, ap_tracking)?;
    insert_value_from_var_name(
        "biased_q",
        Felt252::new(biased_q),
        vm,
        ids_data,
        ap_tracking,
    )
}

/*
Implements hint:

from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.div)
assert 0 < ids.div <= PRIME // range_check_builtin.bound, \
    f'div={hex(ids.div)} is out of the valid range.'
ids.q, ids.r = divmod(ids.value, ids.div)
*/
pub fn unsigned_div_rem(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let div = get_integer_from_var_name("div", vm, ids_data, ap_tracking)?;
    let value = get_integer_from_var_name("value", vm, ids_data, ap_tracking)?;
    let builtin = vm.get_range_check_builtin()?;

    // Main logic
    match &builtin._bound {
        Some(builtin_bound)
            if div.is_zero() || div.as_ref() > &div_prime_by_bound(builtin_bound.clone())? =>
        {
            return Err(HintError::OutOfValidRange(
                div.into_owned(),
                builtin_bound.clone(),
            ));
        }
        None if div.is_zero() => {
            return Err(HintError::OutOfValidRange(
                div.into_owned(),
                Felt252::zero() - Felt252::one(),
            ));
        }
        _ => {}
    }

    let (q, r) = value.div_mod_floor(div.as_ref());
    insert_value_from_var_name("r", r, vm, ids_data, ap_tracking)?;
    insert_value_from_var_name("q", q, vm, ids_data, ap_tracking)
}

//Implements hint: from starkware.cairo.common.math_utils import as_int
//        # Correctness check.
//        value = as_int(ids.value, PRIME) % PRIME
//        assert value < ids.UPPER_BOUND, f'{value} is outside of the range [0, 2**250).'
//        # Calculation for the assertion.
//        ids.high, ids.low = divmod(ids.value, ids.SHIFT)
pub fn assert_250_bit(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    //Declare constant values
    let upper_bound = Felt252::one().shl(250u32);
    let shift = Felt252::one().shl(128u32);
    let value = get_integer_from_var_name("value", vm, ids_data, ap_tracking)?;
    //Main logic
    //can be deleted
    if value.as_ref() > &upper_bound {
        return Err(HintError::ValueOutside250BitRange(value.into_owned()));
    }
    let (high, low) = value.div_rem(&shift);
    insert_value_from_var_name("high", high, vm, ids_data, ap_tracking)?;
    insert_value_from_var_name("low", low, vm, ids_data, ap_tracking)
}

/*
Implements hint:
%{
    from starkware.cairo.common.math_utils import assert_integer
    assert_integer(ids.a)
    assert_integer(ids.b)
    assert (ids.a % PRIME) < (ids.b % PRIME), \
        f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'
%}
*/
pub fn assert_lt_felt(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a = get_integer_from_var_name("a", vm, ids_data, ap_tracking)?;
    let b = get_integer_from_var_name("b", vm, ids_data, ap_tracking)?;
    // Main logic
    // assert_integer(ids.a)
    // assert_integer(ids.b)
    // assert (ids.a % PRIME) < (ids.b % PRIME), \
    //     f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'
    if a >= b {
        return Err(HintError::AssertLtFelt252(a.into_owned(), b.into_owned()));
    };
    Ok(())
}

pub fn is_quad_residue(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let x = get_integer_from_var_name("x", vm, ids_data, ap_tracking)?;

    if x.is_zero() || x.is_one() {
        insert_value_from_var_name("y", x.as_ref().clone(), vm, ids_data, ap_tracking)
    } else if Pow::pow(x.as_ref(), &(Felt252::max_value() >> 1)).is_one() {
        insert_value_from_var_name("y", crate::math_utils::sqrt(&x), vm, ids_data, ap_tracking)
    } else {
        insert_value_from_var_name(
            "y",
            crate::math_utils::sqrt(&(x.as_ref() / Felt252::new(3_i32))),
            vm,
            ids_data,
            ap_tracking,
        )
    }
}

fn div_prime_by_bound(bound: Felt252) -> Result<Felt252, VirtualMachineError> {
    let prime: &BigUint = &CAIRO_PRIME;
    #[allow(deprecated)]
    let limit = prime / bound.to_biguint();
    Ok(Felt252::new(limit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::ops::Shl;

    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::{
                builtin_hint_processor_definition::{BuiltinHintProcessor, HintProcessorData},
                hint_code,
            },
            hint_processor_definition::HintProcessor,
        },
        relocatable,
        types::exec_scope::ExecutionScopes,
        types::relocatable::Relocatable,
        utils::test_utils::*,
        vm::{errors::memory_errors::MemoryError, vm_core::VirtualMachine},
    };
    use assert_matches::assert_matches;
    use felt::felt_str;
    use num_traits::Zero;

    #[cfg(not(target_arch = "wasm32"))]
    use proptest::prelude::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_nn_hint_false() {
        let hint_code = "memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Insert ids into memory
        vm.segments = segments![((1, 9), (-1))];
        add_segments!(vm, 1);
        //Create ids_data & hint_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        run_hint!(vm, ids_data, hint_code).expect("Error while executing hint");
        //Check that ap now contains false (1)
        check_memory![vm.segments.memory, ((1, 0), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_nn_hint_true() {
        let hint_code = "memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), 1)];
        add_segments!(vm, 1);
        //Create ids_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        run_hint!(vm, ids_data, hint_code).expect("Error while executing hint");
        //Check that ap now contains true (0)
        check_memory![vm.segments.memory, ((1, 0), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    //This test contemplates the case when the number itself is negative, but it is within the range (-prime, -range_check_bound)
    //Making the comparison return 1 (true)
    fn run_is_nn_hint_true_border_case() {
        let hint_code = "memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![(
            (1, 4),
            (
                "-3618502788666131213697322783095070105623107215331596699973092056135872020480",
                10
            )
        )];
        //Create ids_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        run_hint!(vm, ids_data, hint_code).expect("Error while executing hint");
        //Check that ap now contains true (0)
        check_memory![vm.segments.memory, ((1, 0), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_nn_hint_no_range_check_builtin() {
        let hint_code = "memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), 1)];
        //Create ids_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(
                VirtualMachineError::NoRangeCheckBuiltin
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_nn_hint_incorrect_ids() {
        let hint_code = "memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1";
        let mut vm = vm_with_range_check!();
        add_segments!(vm, 2);
        //Initialize ap
        //Create ids_data & hint_data
        let ids_data = ids_data!["b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::UnknownIdentifier(x)) if x == "a"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_nn_hint_cant_get_ids_from_memory() {
        let hint_code = "memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1";
        let mut vm = vm_with_range_check!();
        add_segments!(vm, 2);
        //Initialize fp
        vm.run_context.fp = 5;
        //Dont insert ids into memory
        //Create ids_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "a" && y == (1,4).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_nn_hint_ids_are_relocatable_values() {
        let hint_code = "memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), (2, 3))];
        //Create ids_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x,y
            )) if x == "a" && y == (1,4).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_le_felt_valid() {
        let mut constants = HashMap::new();
        constants.insert(
            "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_3_HIGH".to_string(),
            felt_str!("4000000000000088000000000000001", 16),
        );
        constants.insert(
            "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_2_HIGH".to_string(),
            felt_str!("2AAAAAAAAAAAAB05555555555555556", 16),
        );
        let mut vm = vm_with_range_check!();
        let mut exec_scopes = scope![("excluded", 1)];
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 1), ((1, 1), 2), ((1, 2), (2, 0))];
        add_segments!(vm, 1);
        //Create ids_data & hint_data
        let ids_data = ids_data!["a", "b", "range_check_ptr"];
        //Execute the hint
        assert_matches!(
            run_hint!(
                vm,
                ids_data,
                hint_code::ASSERT_LE_FELT,
                &mut exec_scopes,
                &constants
            ),
            Ok(())
        );
        //Hint would return an error if the assertion fails
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn is_le_felt_hint_true() {
        let hint_code = "memory[ap] = 0 if (ids.a % PRIME) <= (ids.b % PRIME) else 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Insert ids into memory
        vm.segments = segments![((1, 8), 1), ((1, 9), 2)];
        add_segments!(vm, 1);
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check result
        check_memory![vm.segments.memory, ((1, 0), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_le_felt_hint_inconsistent_memory() {
        let hint_code = "memory[ap] = 0 if (ids.a % PRIME) <= (ids.b % PRIME) else 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 2;
        vm.segments = segments![((1, 0), 1), ((1, 1), 2)];
        //Create ids_data & hint_data
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(
                    x,
                    y,
                    z
                )
            )) if x == Relocatable::from((1, 0)) &&
                    y == MaybeRelocatable::Int(Felt252::one()) &&
                    z == MaybeRelocatable::Int(Felt252::zero())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_le_felt_hint_incorrect_ids() {
        let hint_code = "memory[ap] = 0 if (ids.a % PRIME) <= (ids.b % PRIME) else 1";
        let mut vm = vm!();
        vm.run_context.fp = 10;
        vm.segments = segments![((1, 8), 1), ((1, 9), 2)];
        //Create ids_data & hint_data
        let ids_data = ids_data!["a", "c"];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::UnknownIdentifier(x)) if x == "b"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_nn_valid() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert 0 <= ids.a % PRIME < range_check_builtin.bound, f'a = {ids.a} is out of range.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 1)];
        //Create ids_data & hint_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Hint would return an error if the assertion fails
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_nn_invalid() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert 0 <= ids.a % PRIME < range_check_builtin.bound, f'a = {ids.a} is out of range.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Insert ids into memory
        vm.segments = segments![((1, 0), (-1))];
        //Create ids_data & hint_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::AssertNNValueOutOfRange(x)) if x == Felt252::new(-1)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_nn_incorrect_ids() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert 0 <= ids.a % PRIME < range_check_builtin.bound, f'a = {ids.a} is out of range.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 4;
        //Insert ids into memory
        vm.segments = segments![((1, 0), (-1))];
        let ids_data = ids_data!["incorrect_id"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::UnknownIdentifier(x)) if x == "a"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_nn_a_is_not_integer() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert 0 <= ids.a % PRIME < range_check_builtin.bound, f'a = {ids.a} is out of range.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 4;
        //Insert ids into memory
        vm.segments = segments![((1, 0), (10, 10))];
        let ids_data = ids_data!["a"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "a" && y == (1,3).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_nn_no_range_check_builtin() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert 0 <= ids.a % PRIME < range_check_builtin.bound, f'a = {ids.a} is out of range.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 1;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 1)];
        let ids_data = ids_data!["a"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(
                VirtualMachineError::NoRangeCheckBuiltin
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_nn_reference_is_not_in_memory() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert 0 <= ids.a % PRIME < range_check_builtin.bound, f'a = {ids.a} is out of range.'";
        let mut vm = vm_with_range_check!();
        add_segments!(vm, 1);
        //Initialize fp
        vm.run_context.fp = 4;
        let ids_data = ids_data!["a"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "a" && y == (1,3).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_assert_le_felt_invalid() {
        let mut vm = vm_with_range_check!();
        let mut constants = HashMap::new();
        constants.insert(
            "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_3_HIGH".to_string(),
            felt_str!("4000000000000088000000000000001", 16),
        );
        constants.insert(
            "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_2_HIGH".to_string(),
            felt_str!("2AAAAAAAAAAAAB05555555555555556", 16),
        );
        let mut exec_scopes = scope![("excluded", Felt252::one())];
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 2), ((1, 1), 1), ((1, 2), (2, 0))];
        let ids_data = ids_data!["a", "b", "range_check_ptr"];
        add_segments!(vm, 1);
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::ASSERT_LE_FELT, &mut exec_scopes, &constants),
            Err(HintError::NonLeFelt252(x, y)) if x == Felt252::new(2) && y == Felt252::one()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_assert_le_felt_a_is_not_integer() {
        let mut vm = vm_with_range_check!();
        let mut constants = HashMap::new();
        constants.insert(
            "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_3_HIGH".to_string(),
            felt_str!("4000000000000088000000000000001", 16),
        );
        constants.insert(
            "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_2_HIGH".to_string(),
            felt_str!("2AAAAAAAAAAAAB05555555555555556", 16),
        );
        let mut exec_scopes = scope![("excluded", 1)];
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), (1, 0)), ((1, 1), 1), ((1, 2), (2, 0))];
        let ids_data = ids_data!["a", "b", "range_check_ptr"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::ASSERT_LE_FELT, &mut exec_scopes, &constants),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "a" && y == (1,0).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_assert_le_felt_b_is_not_integer() {
        let mut vm = vm_with_range_check!();
        let mut constants = HashMap::new();
        constants.insert(
            "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_3_HIGH".to_string(),
            felt_str!("4000000000000088000000000000001", 16),
        );
        constants.insert(
            "starkware.cairo.common.math.assert_le_felt.PRIME_OVER_2_HIGH".to_string(),
            felt_str!("2AAAAAAAAAAAAB05555555555555556", 16),
        );
        let mut exec_scopes = scope![("excluded", 1)];
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 1), ((1, 1), (1, 0)), ((1, 2), (2, 0))];
        let ids_data = ids_data!["a", "b", "range_check_builtin"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::ASSERT_LE_FELT, &mut exec_scopes, &constants),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "b" && y == (1,1).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_nn_hint_out_of_range_false() {
        let hint_code =
            "memory[ap] = 0 if 0 <= ((-ids.a - 1) % PRIME) < range_check_builtin.bound else 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), 2)];
        add_segments!(vm, 1);
        //Create ids_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        run_hint!(vm, ids_data, hint_code).expect("Error while executing hint");
        check_memory![vm.segments.memory, ((1, 0), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_nn_hint_out_of_range_true() {
        let hint_code =
            "memory[ap] = 0 if 0 <= ((-ids.a - 1) % PRIME) < range_check_builtin.bound else 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), (-1))];
        add_segments!(vm, 1);
        //Create ids_data
        let ids_data = ids_data!["a"];
        //Execute the hint
        run_hint!(vm, ids_data, hint_code).expect("Error while executing hint");
        check_memory![vm.segments.memory, ((1, 0), 0)];
    }
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_equal_int_false() {
        let hint_code = "from starkware.cairo.lang.vm.relocatable import RelocatableValue\nboth_ints = isinstance(ids.a, int) and isinstance(ids.b, int)\nboth_relocatable = (\n    isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and\n    ids.a.segment_index == ids.b.segment_index)\nassert both_ints or both_relocatable, \\\n    f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'\nassert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Insert ids into memory
        vm.segments = segments![((1, 8), 1), ((1, 9), 1)];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::AssertNotEqualFail(
                x,
                y
            )) if x == MaybeRelocatable::from(Felt252::one()) &&
                    y == MaybeRelocatable::from(Felt252::one())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_equal_int_true() {
        let hint_code = "from starkware.cairo.lang.vm.relocatable import RelocatableValue\nboth_ints = isinstance(ids.a, int) and isinstance(ids.b, int)\nboth_relocatable = (\n    isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and\n    ids.a.segment_index == ids.b.segment_index)\nassert both_ints or both_relocatable, \\\n    f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'\nassert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Insert ids into memory
        vm.segments = segments![((1, 8), 1), ((1, 9), 3)];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_equal_int_bignum_true() {
        let hint_code = "from starkware.cairo.lang.vm.relocatable import RelocatableValue\nboth_ints = isinstance(ids.a, int) and isinstance(ids.b, int)\nboth_relocatable = (\n    isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and\n    ids.a.segment_index == ids.b.segment_index)\nassert both_ints or both_relocatable, \\\n    f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'\nassert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'";
        let mut vm = vm!();
        add_segments!(vm, 2);
        //Initialize fp
        vm.run_context.fp = 10;
        //Insert ids into memory
        vm.segments = segments![
            ((1, 8), (-1)),
            (
                (1, 9),
                (
                    "618502788666131213697322783095070105623107215331596699973092056135872020480",
                    10
                )
            )
        ];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_equal_relocatable_false() {
        let hint_code = "from starkware.cairo.lang.vm.relocatable import RelocatableValue\nboth_ints = isinstance(ids.a, int) and isinstance(ids.b, int)\nboth_relocatable = (\n    isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and\n    ids.a.segment_index == ids.b.segment_index)\nassert both_ints or both_relocatable, \\\n    f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'\nassert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Insert ids into memory
        vm.segments = segments![((1, 8), (1, 0)), ((1, 9), (1, 0))];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::AssertNotEqualFail(
                x,
                y
            )) if x == MaybeRelocatable::from((1, 0)) &&
                    y == MaybeRelocatable::from((1, 0))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_equal_relocatable_true() {
        let hint_code = "from starkware.cairo.lang.vm.relocatable import RelocatableValue\nboth_ints = isinstance(ids.a, int) and isinstance(ids.b, int)\nboth_relocatable = (\n    isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and\n    ids.a.segment_index == ids.b.segment_index)\nassert both_ints or both_relocatable, \\\n    f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'\nassert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Insert ids into memory
        vm.segments = segments![((1, 8), (0, 1)), ((1, 9), (0, 0))];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_non_equal_relocatable_diff_index() {
        let hint_code = "from starkware.cairo.lang.vm.relocatable import RelocatableValue\nboth_ints = isinstance(ids.a, int) and isinstance(ids.b, int)\nboth_relocatable = (\n    isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and\n    ids.a.segment_index == ids.b.segment_index)\nassert both_ints or both_relocatable, \\\n    f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'\nassert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Insert ids into memory
        vm.segments = segments![((1, 8), (2, 0)), ((1, 9), (1, 0))];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(VirtualMachineError::DiffIndexComp(
                x,
                y
            ))) if x == relocatable!(2, 0) && y == relocatable!(1, 0)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_equal_relocatable_and_integer() {
        let hint_code = "from starkware.cairo.lang.vm.relocatable import RelocatableValue\nboth_ints = isinstance(ids.a, int) and isinstance(ids.b, int)\nboth_relocatable = (\n    isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and\n    ids.a.segment_index == ids.b.segment_index)\nassert both_ints or both_relocatable, \\\n    f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'\nassert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Insert ids into memory
        vm.segments = segments![((1, 8), (1, 0)), ((1, 9), 1)];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(
                VirtualMachineError::DiffTypeComparison(
                    x,
                    y
                )
            )) if x == MaybeRelocatable::from((1, 0)) && y == MaybeRelocatable::from(Felt252::one())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_zero_true() {
        let hint_code =
    "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.value)\nassert ids.value % PRIME != 0, f'assert_not_zero failed: {ids.value} = 0.'";
        let mut vm = vm!();
        // //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), 5)];
        //Create ids
        let ids_data = ids_data!["value"];

        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_zero_false() {
        let hint_code =
    "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.value)\nassert ids.value % PRIME != 0, f'assert_not_zero failed: {ids.value} = 0.'";
        let mut vm = vm!();
        // //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), 0)];
        //Create ids
        let ids_data = ids_data!["value"];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::AssertNotZero(
                x,
                y
            )) if x == Felt252::zero() && y == *felt::PRIME_STR.to_string()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_zero_incorrect_id() {
        let hint_code =
    "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.value)\nassert ids.value % PRIME != 0, f'assert_not_zero failed: {ids.value} = 0.'";
        let mut vm = vm!();
        // //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), 0)];
        //Create invalid id key
        let ids_data = ids_data!["incorrect_id"];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::UnknownIdentifier(x))  if x == "value"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_not_zero_expected_integer_error() {
        let hint_code =
    "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.value)\nassert ids.value % PRIME != 0, f'assert_not_zero failed: {ids.value} = 0.'";
        let mut vm = vm!();
        // //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), (1, 0))];
        //Create ids_data & hint_data
        let ids_data = ids_data!["value"];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "value" && y == (1,4).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_int_assertion_invalid() {
        let hint_code = "assert ids.value == 0, 'split_int(): value is out of range.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), 1)];
        let ids_data = ids_data!["value"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::SplitIntNotZero)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_int_assertion_valid() {
        let hint_code = "assert ids.value == 0, 'split_int(): value is out of range.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Insert ids into memory
        vm.segments = segments![((1, 4), 0)];
        let ids_data = ids_data!["value"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_int_valid() {
        let hint_code = "memory[ids.output] = res = (int(ids.value) % PRIME) % ids.base\nassert res < ids.bound, f'split_int(): Limb {res} is out of range.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 4;
        //Insert ids into memory
        vm.segments = segments![((1, 0), (2, 0)), ((1, 1), 2), ((1, 2), 10), ((1, 3), 100)];
        add_segments!(vm, 2);
        let ids_data = ids_data!["output", "value", "base", "bound"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        check_memory![vm.segments.memory, ((2, 0), 2)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_int_invalid() {
        let hint_code = "memory[ids.output] = res = (int(ids.value) % PRIME) % ids.base\nassert res < ids.bound, f'split_int(): Limb {res} is out of range.'";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 4;
        //Insert ids into memory
        vm.segments = segments![
            ((1, 0), (2, 0)),
            ((1, 1), 100),
            ((1, 2), 10000),
            ((1, 3), 10)
        ];
        add_segments!(vm, 2);
        let ids_data = ids_data!["output", "value", "base", "bound"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::SplitIntLimbOutOfRange(x)) if x == Felt252::new(100)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_positive_hint_true() {
        let hint_code =
        "from starkware.cairo.common.math_utils import is_positive\nids.is_positive = 1 if is_positive(\n    value=ids.value, prime=PRIME, rc_bound=range_check_builtin.bound) else 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 2;
        //Insert ids.value into memory
        vm.segments = segments![((1, 0), 250)];
        //Dont insert ids.is_positive as we need to modify it inside the hint
        //Create ids
        let ids_data = ids_data!["value", "is_positive"];
        //Execute the hint
        run_hint!(vm, ids_data, hint_code).expect("Error while executing hint");
        //Check that is_positive now contains 1 (true)
        check_memory![vm.segments.memory, ((1, 1), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_positive_hint_false() {
        let hint_code =
        "from starkware.cairo.common.math_utils import is_positive\nids.is_positive = 1 if is_positive(\n    value=ids.value, prime=PRIME, rc_bound=range_check_builtin.bound) else 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 2;
        //Insert ids.value into memory
        vm.segments = segments![((1, 0), (-250))];
        //Dont insert ids.is_positive as we need to modify it inside the hint
        let ids_data = ids_data!["value", "is_positive"];
        //Execute the hint
        run_hint!(vm, ids_data, hint_code).expect("Error while executing hint");
        //Check that is_positive now contains 0 (false)
        check_memory![vm.segments.memory, ((1, 1), 0)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_positive_hint_outside_valid_range() {
        let hint_code =
        "from starkware.cairo.common.math_utils import is_positive\nids.is_positive = 1 if is_positive(\n    value=ids.value, prime=PRIME, rc_bound=range_check_builtin.bound) else 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 2;
        //Insert ids.value into memory
        vm.segments = segments![(
            (1, 0),
            (
                "618502761706184546546682988428055018603476541694452277432519575032261771265",
                10
            )
        )];
        //Dont insert ids.is_positive as we need to modify it inside the hint
        let ids_data = ids_data!["value", "is_positive"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::ValueOutsideValidRange(x)) if x == felt_str!(
                "618502761706184546546682988428055018603476541694452277432519575032261771265"
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_is_positive_hint_is_positive_not_empty() {
        let hint_code ="from starkware.cairo.common.math_utils import is_positive\nids.is_positive = 1 if is_positive(\n    value=ids.value, prime=PRIME, rc_bound=range_check_builtin.bound) else 0";
        let mut vm = vm_with_range_check!();
        add_segments!(vm, 2);
        //Initialize fp
        vm.run_context.fp = 2;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 2), ((1, 1), 4)];
        let ids_data = ids_data!["value", "is_positive"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(
                    x,
                    y,
                    z
                )
            )) if x == Relocatable::from((1, 1)) &&
                    y == MaybeRelocatable::from(Felt252::new(4)) &&
                    z == MaybeRelocatable::from(Felt252::one())
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_sqrt_valid() {
        let hint_code = "from starkware.python.math_utils import isqrt\nvalue = ids.value % PRIME\nassert value < 2 ** 250, f\"value={value} is outside of the range [0, 2**250).\"\nassert 2 ** 250 < PRIME\nids.root = isqrt(value)";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 2;
        //Insert ids.value into memory
        vm.segments = segments![((1, 0), 81)];
        //Create ids
        let ids_data = ids_data!["value", "root"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check that root (0,1) has the square root of 81
        check_memory![vm.segments.memory, ((1, 1), 9)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_sqrt_invalid_negative_number() {
        let hint_code = "from starkware.python.math_utils import isqrt\nvalue = ids.value % PRIME\nassert value < 2 ** 250, f\"value={value} is outside of the range [0, 2**250).\"\nassert 2 ** 250 < PRIME\nids.root = isqrt(value)";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 2;
        //Insert ids.value into memory
        vm.segments = segments![((1, 0), (-81))];
        //Create ids
        let ids_data = ids_data!["value", "root"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::ValueOutside250BitRange(x)) if x == felt_str!(
                "3618502788666131213697322783095070105623107215331596699973092056135872020400"
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_sqrt_invalid_mismatched_root() {
        let hint_code = "from starkware.python.math_utils import isqrt\nvalue = ids.value % PRIME\nassert value < 2 ** 250, f\"value={value} is outside of the range [0, 2**250).\"\nassert 2 ** 250 < PRIME\nids.root = isqrt(value)";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 2;
        //Insert ids.value into memory
        vm.segments = segments![((1, 0), 81), ((1, 1), 7)];
        //Create ids
        let ids_data = ids_data!["value", "root"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(
                    x,
                    y,
                    z
                )
            )) if x == Relocatable::from((1, 1)) &&
                    y == MaybeRelocatable::from(Felt252::new(7)) &&
                    z == MaybeRelocatable::from(Felt252::new(9))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsigned_div_rem_success() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\nids.q, ids.r = divmod(ids.value, ids.div)";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 4;
        //Insert ids into memory
        vm.segments = segments![((1, 2), 5), ((1, 3), 7)];
        //Create ids
        let ids_data = ids_data!["r", "q", "div", "value"];
        //Execute the hint
        assert!(run_hint!(vm, ids_data, hint_code).is_ok());
        check_memory![vm.segments.memory, ((1, 0), 2), ((1, 1), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsigned_div_rem_out_of_range() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\nids.q, ids.r = divmod(ids.value, ids.div)";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 4;
        //Insert ids into memory
        vm.segments = segments![((1, 2), (-5)), ((1, 3), 7)];
        //Create ids
        let ids_data = ids_data!["r", "q", "div", "value"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::OutOfValidRange(
                x,
                y
            )) if x == Felt252::new(-5) && y == felt_str!("340282366920938463463374607431768211456")
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsigned_div_rem_no_range_check_builtin() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\nids.q, ids.r = divmod(ids.value, ids.div)";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 4;
        //Insert ids into memory
        vm.segments = segments![((1, 2), 5), ((1, 3), 7)];
        //Create ids_data
        let ids_data = ids_data!["r", "q", "div", "value"];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(
                VirtualMachineError::NoRangeCheckBuiltin
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsigned_div_rem_inconsitent_memory() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\nids.q, ids.r = divmod(ids.value, ids.div)";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 4;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 5), ((1, 2), 5), ((1, 3), 7)];
        //Create ids_data
        let ids_data = ids_data!["r", "q", "div", "value"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(
                    x,
                    y,
                    z
                )
            )) if x == Relocatable::from((1, 0)) &&
                    y == MaybeRelocatable::Int(Felt252::new(5)) &&
                    z == MaybeRelocatable::Int(Felt252::new(2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsigned_div_rem_incorrect_ids() {
        let hint_code = "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\nids.q, ids.r = divmod(ids.value, ids.div)";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 4;
        //Insert ids into memory
        vm.segments = segments![((1, 2), 5), ((1, 3), 7)];
        //Create ids
        let ids_data = ids_data!["a", "b", "iv", "vlue"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::UnknownIdentifier(x)) if x == "div"
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn signed_div_rem_success() {
        let hint_code = "from starkware.cairo.common.math_utils import as_int, assert_integer\n\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\n\nassert_integer(ids.bound)\nassert ids.bound <= range_check_builtin.bound // 2, \\\n    f'bound={hex(ids.bound)} is out of the valid range.'\n\nint_value = as_int(ids.value, PRIME)\nq, ids.r = divmod(int_value, ids.div)\n\nassert -ids.bound <= q < ids.bound, \\\n    f'{int_value} / {ids.div} = {q} is out of the range [{-ids.bound}, {ids.bound}).'\n\nids.biased_q = q + ids.bound";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 6;
        //Insert ids into memory
        vm.segments = segments![((1, 3), 5), ((1, 4), 10), ((1, 5), 29)];
        //Create ids
        let ids_data = ids_data!["r", "biased_q", "range_check_ptr", "div", "value", "bound"];
        //Execute the hint
        assert!(run_hint!(vm, ids_data, hint_code).is_ok());
        check_memory![vm.segments.memory, ((1, 0), 0), ((1, 1), 31)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn signed_div_rem_negative_quotient() {
        let hint_code = "from starkware.cairo.common.math_utils import as_int, assert_integer\n\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\n\nassert_integer(ids.bound)\nassert ids.bound <= range_check_builtin.bound // 2, \\\n    f'bound={hex(ids.bound)} is out of the valid range.'\n\nint_value = as_int(ids.value, PRIME)\nq, ids.r = divmod(int_value, ids.div)\n\nassert -ids.bound <= q < ids.bound, \\\n    f'{int_value} / {ids.div} = {q} is out of the range [{-ids.bound}, {ids.bound}).'\n\nids.biased_q = q + ids.bound";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 6;
        //Insert ids into memory
        vm.segments = segments![((1, 3), 7), ((1, 4), (-10)), ((1, 5), 29)];
        //Create ids
        let ids_data = ids_data!["r", "biased_q", "range_check_ptr", "div", "value", "bound"];
        //Execute the hint
        assert!(run_hint!(vm, ids_data, hint_code).is_ok());
        check_memory![vm.segments.memory, ((1, 0), 4), ((1, 1), 27)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn signed_div_rem_out_of_range() {
        let hint_code = "from starkware.cairo.common.math_utils import as_int, assert_integer\n\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\n\nassert_integer(ids.bound)\nassert ids.bound <= range_check_builtin.bound // 2, \\\n    f'bound={hex(ids.bound)} is out of the valid range.'\n\nint_value = as_int(ids.value, PRIME)\nq, ids.r = divmod(int_value, ids.div)\n\nassert -ids.bound <= q < ids.bound, \\\n    f'{int_value} / {ids.div} = {q} is out of the range [{-ids.bound}, {ids.bound}).'\n\nids.biased_q = q + ids.bound";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 6;
        //Insert ids into memory
        vm.segments = segments![((1, 3), (-5)), ((1, 4), 10), ((1, 5), 29)];
        //Create ids
        let ids_data = ids_data!["r", "biased_q", "range_check_ptr", "div", "value", "bound"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::OutOfValidRange(
                x,
                y
            )) if x == Felt252::new(-5) && y == felt_str!("340282366920938463463374607431768211456")
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn signed_div_rem_no_range_check_builtin() {
        let hint_code = "from starkware.cairo.common.math_utils import as_int, assert_integer\n\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\n\nassert_integer(ids.bound)\nassert ids.bound <= range_check_builtin.bound // 2, \\\n    f'bound={hex(ids.bound)} is out of the valid range.'\n\nint_value = as_int(ids.value, PRIME)\nq, ids.r = divmod(int_value, ids.div)\n\nassert -ids.bound <= q < ids.bound, \\\n    f'{int_value} / {ids.div} = {q} is out of the range [{-ids.bound}, {ids.bound}).'\n\nids.biased_q = q + ids.bound";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 6;
        //Insert ids into memory
        vm.segments = segments![((1, 3), 5), ((1, 4), 10), ((1, 5), 29)];
        //Create ids
        let ids_data = ids_data!["r", "biased_q", "range_check_ptr", "div", "value", "bound"];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(
                VirtualMachineError::NoRangeCheckBuiltin
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn signed_div_rem_inconsitent_memory() {
        let hint_code = "from starkware.cairo.common.math_utils import as_int, assert_integer\n\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\n\nassert_integer(ids.bound)\nassert ids.bound <= range_check_builtin.bound // 2, \\\n    f'bound={hex(ids.bound)} is out of the valid range.'\n\nint_value = as_int(ids.value, PRIME)\nq, ids.r = divmod(int_value, ids.div)\n\nassert -ids.bound <= q < ids.bound, \\\n    f'{int_value} / {ids.div} = {q} is out of the range [{-ids.bound}, {ids.bound}).'\n\nids.biased_q = q + ids.bound";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 6;
        //Insert ids into memory
        vm.segments = segments![((1, 1), 10), ((1, 3), 5), ((1, 4), 10), ((1, 5), 29)];
        //Create ids
        let ids_data = ids_data!["r", "biased_q", "range_check_ptr", "div", "value", "bound"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(
                    x,
                    y,
                    z
                )
            )) if x == Relocatable::from((1, 1)) &&
                    y == MaybeRelocatable::Int(Felt252::new(10)) &&
                    z == MaybeRelocatable::Int(Felt252::new(31))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn signed_div_rem_incorrect_ids() {
        let hint_code = "from starkware.cairo.common.math_utils import as_int, assert_integer\n\nassert_integer(ids.div)\nassert 0 < ids.div <= PRIME // range_check_builtin.bound, \\\n    f'div={hex(ids.div)} is out of the valid range.'\n\nassert_integer(ids.bound)\nassert ids.bound <= range_check_builtin.bound // 2, \\\n    f'bound={hex(ids.bound)} is out of the valid range.'\n\nint_value = as_int(ids.value, PRIME)\nq, ids.r = divmod(int_value, ids.div)\n\nassert -ids.bound <= q < ids.bound, \\\n    f'{int_value} / {ids.div} = {q} is out of the range [{-ids.bound}, {ids.bound}).'\n\nids.biased_q = q + ids.bound";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 6;
        //Insert ids into memory
        vm.segments = segments![((1, 3), 5), ((1, 4), 10), ((1, 5), 29)];
        //Create ids
        let ids_data = ids_data!["r", "b", "r", "d", "v", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::UnknownIdentifier(x)) if x == "div"
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_250_bit_valid() {
        let hint_code = "from starkware.cairo.common.math_utils import as_int\n\n# Correctness check.\nvalue = as_int(ids.value, PRIME) % PRIME\nassert value < ids.UPPER_BOUND, f'{value} is outside of the range [0, 2**250).'\n\n# Calculation for the assertion.\nids.high, ids.low = divmod(ids.value, ids.SHIFT)";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 0), 1)];
        //Create ids
        let ids_data = ids_data!["value", "high", "low"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Hint would return an error if the assertion fails
        //Check ids.high and ids.low values
        check_memory![vm.segments.memory, ((1, 1), 0), ((1, 2), 1)];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_250_bit_invalid() {
        let hint_code = "from starkware.cairo.common.math_utils import as_int\n\n# Correctness check.\nvalue = as_int(ids.value, PRIME) % PRIME\nassert value < ids.UPPER_BOUND, f'{value} is outside of the range [0, 2**250).'\n\n# Calculation for the assertion.\nids.high, ids.low = divmod(ids.value, ids.SHIFT)";
        let mut vm = vm!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        //ids.value
        vm.segments = segments![(
            (1, 0),
            (
                "3618502788666131106986593281521497120414687020801267626233049500247285301248",
                10
            )
        )];
        //Create ids
        let ids_data = ids_data!["value", "high", "low"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::ValueOutside250BitRange(x)) if x == Felt252::one().shl(251_u32)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_felt_ok() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128\nassert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW\nassert_integer(ids.value)\nids.low = ids.value & ((1 << 128) - 1)\nids.high = ids.value >> 128";
        let mut vm = vm_with_range_check!();
        vm.segments = segments![
            ((1, 3), ("335438970432432812899076431678123043273", 10)),
            ((1, 4), (2, 0))
        ];
        add_segments!(vm, 1);
        //Initialize fp
        vm.run_context.fp = 7;
        //Create ids
        let ids_data = HashMap::from([
            ("value".to_string(), HintReference::new_simple(-4)),
            ("low".to_string(), HintReference::new(-3, 0, true, true)),
            ("high".to_string(), HintReference::new(-3, 1, true, true)),
        ]);
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            ((2, 0), ("335438970432432812899076431678123043273", 10)),
            ((2, 1), 0)
        ];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_felt_incorrect_ids() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128\nassert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW\nassert_integer(ids.value)\nids.low = ids.value & ((1 << 128) - 1)\nids.high = ids.value >> 128";
        let mut vm = vm_with_range_check!();
        vm.segments = segments![
            ((1, 3), ("335438970432432812899076431678123043273", 10)),
            ((1, 4), (2, 0))
        ];
        //Initialize fp
        vm.run_context.fp = 7;
        //Create incomplete ids
        //Create ids_data & hint_data
        let ids_data = ids_data!["low"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::UnknownIdentifier(x)) if x == "value"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_felt_fails_first_insert() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128\nassert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW\nassert_integer(ids.value)\nids.low = ids.value & ((1 << 128) - 1)\nids.high = ids.value >> 128";
        let mut vm = vm_with_range_check!();
        vm.segments = segments![
            ((1, 3), ("335438970432432812899076431678123043273", 10)),
            ((1, 4), (2, 0)),
            ((2, 0), 99)
        ];
        //Initialize fp
        vm.run_context.fp = 7;
        //Create ids_data & hint_data
        let ids_data = HashMap::from([
            ("value".to_string(), HintReference::new_simple(-4)),
            ("low".to_string(), HintReference::new(-3, 0, true, true)),
            ("high".to_string(), HintReference::new(-3, 1, true, true)),
        ]);

        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(
                    x,
                    y,
                    z
                )
            )) if x == Relocatable::from((2, 0)) &&
                    y == MaybeRelocatable::from(Felt252::new(99)) &&
                    z == MaybeRelocatable::from(felt_str!("335438970432432812899076431678123043273"))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_felt_fails_second_insert() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128\nassert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW\nassert_integer(ids.value)\nids.low = ids.value & ((1 << 128) - 1)\nids.high = ids.value >> 128";
        let mut vm = vm_with_range_check!();
        vm.segments = segments![
            ((1, 4), (2, 0)),
            ((1, 3), ("335438970432432812899076431678123043273", 10)),
            ((2, 1), 99)
        ];
        add_segments!(vm, 1);
        //Initialize fp
        vm.run_context.fp = 7;
        //Create ids_data & hint_data
        let ids_data = HashMap::from([
            ("value".to_string(), HintReference::new_simple(-4)),
            ("low".to_string(), HintReference::new(-3, 0, true, true)),
            ("high".to_string(), HintReference::new(-3, 1, true, true)),
        ]);
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(
                MemoryError::InconsistentMemory(
                    x,
                    y,
                    z
                )
            )) if x == Relocatable::from((2, 1)) &&
                    y == MaybeRelocatable::from(Felt252::new(99)) &&
                    z == MaybeRelocatable::from(Felt252::new(0))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_split_felt_value_is_not_integer() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128\nassert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW\nassert_integer(ids.value)\nids.low = ids.value & ((1 << 128) - 1)\nids.high = ids.value >> 128";
        let mut vm = vm_with_range_check!();
        vm.segments = segments![((1, 3), (1, 0)), ((1, 4), (2, 0))];
        //Initialize fp
        vm.run_context.fp = 7;
        //Create ids_data & hint_data
        let ids_data = HashMap::from([
            ("value".to_string(), HintReference::new_simple(-4)),
            ("low".to_string(), HintReference::new(-3, 0, true, true)),
            ("high".to_string(), HintReference::new(-3, 1, true, true)),
        ]);
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "value" && y == (1,3).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_lt_felt_ok() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert_integer(ids.b)\nassert (ids.a % PRIME) < (ids.b % PRIME), \\\n    f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids into memory
        vm.segments = segments![((1, 1), 1), ((1, 2), 2)];
        //Create ids
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(run_hint!(vm, ids_data, hint_code), Ok(()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_lt_felt_assert_fails() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert_integer(ids.b)\nassert (ids.a % PRIME) < (ids.b % PRIME), \\\n    f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 3;
        vm.segments = segments![((1, 1), 3), ((1, 2), 2)];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::AssertLtFelt252(x, y)) if x == Felt252::new(3) && y == Felt252::new(2)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_lt_felt_incorrect_ids() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert_integer(ids.b)\nassert (ids.a % PRIME) < (ids.b % PRIME), \\\n    f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 3;
        vm.segments = segments![((1, 1), 1), ((1, 2), 2)];
        //Create Incorrects ids
        let ids_data = ids_data!["a"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::UnknownIdentifier(x)) if x == "b"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_lt_felt_a_is_not_integer() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert_integer(ids.b)\nassert (ids.a % PRIME) < (ids.b % PRIME), \\\n    f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 3;
        vm.segments = segments![((1, 1), (1, 0)), ((1, 2), 2)];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "a" && y == (1,1).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_lt_felt_b_is_not_integer() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert_integer(ids.b)\nassert (ids.a % PRIME) < (ids.b % PRIME), \\\n    f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 3;
        vm.segments = segments![((1, 1), 1), ((1, 2), (1, 0))];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "b" && y == (1,2).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_assert_lt_felt_ok_failed_to_get_ids() {
        let hint_code =
        "from starkware.cairo.common.math_utils import assert_integer\nassert_integer(ids.a)\nassert_integer(ids.b)\nassert (ids.a % PRIME) < (ids.b % PRIME), \\\n    f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 3;
        //Insert ids.a into memory
        vm.segments = segments![((1, 1), 1)];
        let ids_data = ids_data!["a", "b"];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x, y
            )) if x == "b" && y == (1,2).into()
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    proptest! {
        #[test]
        // Proptest to check is_quad_residue hint function
        fn run_is_quad_residue(ref x in "([1-9][0-9]*)") {
            let mut vm = vm!();
            vm.run_context.fp = 2;
            vm.segments = segments![((1, 1), (&x[..], 10))];
            let ids_data = ids_data!["y", "x"];

            assert_matches!(run_hint!(vm, ids_data, hint_code::IS_QUAD_RESIDUE), Ok(()));

            let x = &Felt252::parse_bytes(x.as_bytes(), 10).unwrap();

            if x.is_zero() || x.is_one() {
                assert_eq!(vm.get_integer(Relocatable::from((1, 0))).unwrap().as_ref(), x);
            } else if x.pow(&(Felt252::max_value() >> 1)).is_one() {
                assert_eq!(vm.get_integer(Relocatable::from((1, 0))).unwrap().into_owned(), crate::math_utils::sqrt(x));
            } else {
                assert_eq!(vm.get_integer(Relocatable::from((1, 0))).unwrap().into_owned(), crate::math_utils::sqrt(&(x / Felt252::new(3))));
            }
        }
    }
}
