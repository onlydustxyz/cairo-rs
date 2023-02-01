use std::prelude::v1::*;

use crate::{
    hint_processor::builtin_hint_processor::hint_utils::{
        get_integer_from_var_name, get_relocatable_from_var_name, insert_value_from_var_name,
        insert_value_into_ap,
    },
    hint_processor::hint_processor_definition::HintReference,
    math_utils::isqrt,
    serde::deserialize_program::ApTracking,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use felt::Felt;
use num_integer::div_rem;
use num_traits::{One, Signed, Zero};
use std::{
    collections::HashMap,
    ops::{Shl, Shr},
};
/*
Implements hint:
%{
    sum_low = ids.a.low + ids.b.low
    ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
    sum_high = ids.a.high + ids.b.high + ids.carry_low
    ids.carry_high = 1 if sum_high >= ids.SHIFT else 0
%}
*/
pub fn uint256_add(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let shift = Felt::new(1_u32) << 128_u32;
    let a_relocatable = get_relocatable_from_var_name("a", vm, ids_data, ap_tracking)?;
    let b_relocatable = get_relocatable_from_var_name("b", vm, ids_data, ap_tracking)?;
    let a_low = vm.get_integer(&a_relocatable)?;
    let a_high = vm.get_integer(&(a_relocatable + 1_usize))?;
    let b_low = vm.get_integer(&b_relocatable)?;
    let b_high = vm.get_integer(&(b_relocatable + 1_usize))?;
    let a_low = a_low.as_ref();
    let a_high = a_high.as_ref();
    let b_low = b_low.as_ref();
    let b_high = b_high.as_ref();

    //Main logic
    //sum_low = ids.a.low + ids.b.low
    //ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
    //sum_high = ids.a.high + ids.b.high + ids.carry_low
    //ids.carry_high = 1 if sum_high >= ids.SHIFT else 0

    let carry_low = if a_low + b_low >= shift {
        Felt::one()
    } else {
        Felt::zero()
    };

    let carry_high = if a_high + b_high + &carry_low >= shift {
        Felt::one()
    } else {
        Felt::zero()
    };
    insert_value_from_var_name("carry_high", carry_high, vm, ids_data, ap_tracking)?;
    insert_value_from_var_name("carry_low", carry_low, vm, ids_data, ap_tracking)
}

/*
Implements hint:
%{
    ids.low = ids.a & ((1<<64) - 1)
    ids.high = ids.a >> 64
%}
*/
pub fn split_64(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a = get_integer_from_var_name("a", vm, ids_data, ap_tracking)?;
    let mut digits = a.iter_u64_digits();
    let low = Felt::new(digits.next().unwrap_or(0u64));
    let high = if digits.len() <= 1 {
        Felt::new(digits.next().unwrap_or(0u64))
    } else {
        a.as_ref().shr(64_u32)
    };
    insert_value_from_var_name("high", high, vm, ids_data, ap_tracking)?;
    insert_value_from_var_name("low", low, vm, ids_data, ap_tracking)
}

/*
Implements hint:
%{
    from starkware.python.math_utils import isqrt
    n = (ids.n.high << 128) + ids.n.low
    root = isqrt(n)
    assert 0 <= root < 2 ** 128
    ids.root.low = root
    ids.root.high = 0
%}
*/
pub fn uint256_sqrt(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let n_addr = get_relocatable_from_var_name("n", vm, ids_data, ap_tracking)?;
    let root_addr = get_relocatable_from_var_name("root", vm, ids_data, ap_tracking)?;
    let n_low = vm.get_integer(&n_addr)?;
    let n_high = vm.get_integer(&(n_addr + 1_usize))?;
    let n_low = n_low.as_ref();
    let n_high = n_high.as_ref();

    //Main logic
    //from starkware.python.math_utils import isqrt
    //n = (ids.n.high << 128) + ids.n.low
    //root = isqrt(n)
    //assert 0 <= root < 2 ** 128
    //ids.root.low = root
    //ids.root.high = 0

    let root = isqrt(&(&n_high.to_biguint().shl(128_u32) + n_low.to_biguint()))?;

    if root >= num_bigint::BigUint::one().shl(128_u32) {
        return Err(HintError::AssertionFailed(format!(
            "assert 0 <= {} < 2 ** 128",
            &root
        )));
    }
    vm.insert_value(&root_addr, Felt::new(root))?;
    vm.insert_value(&(root_addr + 1_i32), Felt::zero())
        .map_err(HintError::Internal)
}

/*
Implements hint:
%{ memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0 %}
*/
pub fn uint256_signed_nn(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a_addr = get_relocatable_from_var_name("a", vm, ids_data, ap_tracking)?;
    let a_high = vm.get_integer(&(a_addr + 1_usize))?;
    //Main logic
    //memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0
    let result: Felt = if !a_high.is_negative() && a_high.as_ref() <= &Felt::new(i128::MAX) {
        Felt::one()
    } else {
        Felt::zero()
    };
    insert_value_into_ap(vm, result)
}

/*
Implements hint:
%{
    a = (ids.a.high << 128) + ids.a.low
    div = (ids.div.high << 128) + ids.div.low
    quotient, remainder = divmod(a, div)

    ids.quotient.low = quotient & ((1 << 128) - 1)
    ids.quotient.high = quotient >> 128
    ids.remainder.low = remainder & ((1 << 128) - 1)
    ids.remainder.high = remainder >> 128
%}
*/
pub fn uint256_unsigned_div_rem(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a_addr = get_relocatable_from_var_name("a", vm, ids_data, ap_tracking)?;
    let div_addr = get_relocatable_from_var_name("div", vm, ids_data, ap_tracking)?;
    let quotient_addr = get_relocatable_from_var_name("quotient", vm, ids_data, ap_tracking)?;
    let remainder_addr = get_relocatable_from_var_name("remainder", vm, ids_data, ap_tracking)?;

    let a_low = vm.get_integer(&a_addr)?;
    let a_high = vm.get_integer(&(a_addr + 1_usize))?;
    let div_low = vm.get_integer(&div_addr)?;
    let div_high = vm.get_integer(&(div_addr + 1_usize))?;
    let a_low = a_low.as_ref();
    let a_high = a_high.as_ref();
    let div_low = div_low.as_ref();
    let div_high = div_high.as_ref();

    //Main logic
    //a = (ids.a.high << 128) + ids.a.low
    //div = (ids.div.high << 128) + ids.div.low
    //quotient, remainder = divmod(a, div)

    //ids.quotient.low = quotient & ((1 << 128) - 1)
    //ids.quotient.high = quotient >> 128
    //ids.remainder.low = remainder & ((1 << 128) - 1)
    //ids.remainder.high = remainder >> 128

    let a = &a_high.shl(128_usize) + a_low;
    let div = &div_high.shl(128_usize) + div_low;
    //a and div will always be positive numbers
    //Then, Rust div_rem equals Python divmod
    let (quotient, remainder) = div_rem(a, div);
    let quotient_low = &quotient & &Felt::new(u128::MAX);
    let quotient_high = quotient.shr(128);

    let remainder_low = &remainder & &Felt::new(u128::MAX);
    let remainder_high = remainder.shr(128);

    //Insert ids.quotient.low
    vm.insert_value(&quotient_addr, quotient_low)?;
    //Insert ids.quotient.high
    vm.insert_value(&(quotient_addr + 1_i32), quotient_high)?;
    //Insert ids.remainder.low
    vm.insert_value(&remainder_addr, remainder_low)?;
    //Insert ids.remainder.high
    vm.insert_value(&(remainder_addr + 1_i32), remainder_high)?;
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
        types::{
            exec_scope::ExecutionScopes,
            relocatable::{MaybeRelocatable, Relocatable},
        },
        utils::test_utils::*,
        vm::{
            errors::{memory_errors::MemoryError, vm_errors::VirtualMachineError},
            runners::builtin_runner::RangeCheckBuiltinRunner,
            vm_core::VirtualMachine,
            vm_memory::memory::Memory,
        },
    };
    use felt::felt_str;
    use std::any::Any;

    #[test]
    fn run_uint256_add_ok() {
        let hint_code = "sum_low = ids.a.low + ids.b.low\nids.carry_low = 1 if sum_low >= ids.SHIFT else 0\nsum_high = ids.a.high + ids.b.high + ids.carry_low\nids.carry_high = 1 if sum_high >= ids.SHIFT else 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -6), ("b", -4), ("carry_high", 3), ("carry_low", 2)];
        vm.memory = memory![
            ((1, 4), 2),
            ((1, 5), 3),
            ((1, 6), 4),
            ((1, 7), ("340282366920938463463374607431768211456", 10))
        ];
        //Execute the hint
        assert_eq!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        check_memory![&vm.memory, ((1, 12), 0), ((1, 13), 1)];
    }

    #[test]
    fn run_uint256_add_fail_inserts() {
        let hint_code = "sum_low = ids.a.low + ids.b.low\nids.carry_low = 1 if sum_low >= ids.SHIFT else 0\nsum_high = ids.a.high + ids.b.high + ids.carry_low\nids.carry_high = 1 if sum_high >= ids.SHIFT else 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -6), ("b", -4), ("carry_high", 3), ("carry_low", 2)];
        //Insert ids into memory
        vm.memory = memory![
            ((1, 4), 2),
            ((1, 5), 3),
            ((1, 6), 4),
            ((1, 7), 2),
            ((1, 12), 2)
        ];
        //Execute the hint
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(VirtualMachineError::MemoryError(
                MemoryError::InconsistentMemory(
                    MaybeRelocatable::from((1, 12)),
                    MaybeRelocatable::from(Felt::new(2)),
                    MaybeRelocatable::from(Felt::zero())
                )
            )))
        );
    }

    #[test]
    fn run_split_64_ok() {
        let hint_code = "ids.low = ids.a & ((1<<64) - 1)\nids.high = ids.a >> 64";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -3), ("high", 1), ("low", 0)];
        //Insert ids.a into memory
        vm.memory = memory![((1, 7), ("850981239023189021389081239089023", 10))];
        //Execute the hint
        assert_eq!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        //ids.low, ids.high
        check_memory![
            &vm.memory,
            ((1, 10), 7249717543555297151_u64),
            ((1, 11), 46131785404667_u64)
        ];
    }

    #[test]
    fn run_split_64_with_big_a() {
        let hint_code = "ids.low = ids.a & ((1<<64) - 1)\nids.high = ids.a >> 64";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create ids_data
        let ids_data = non_continuous_ids_data![("a", -3), ("high", 1), ("low", 0)];
        //Insert ids.a into memory
        vm.memory = memory![((1, 7), ("400066369019890261321163226850167045262", 10))];
        //Execute the hint
        assert_eq!(run_hint!(vm, ids_data, hint_code), Ok(()));

        //Check hint memory inserts
        //ids.low, ids.high
        check_memory![
            &vm.memory,
            ((1, 10), 2279400676465785998_u64),
            ((1, 11), 21687641321487626429_u128)
        ];
    }

    #[test]
    fn run_split_64_memory_error() {
        let hint_code = "ids.low = ids.a & ((1<<64) - 1)\nids.high = ids.a >> 64";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -3), ("high", 1), ("low", 0)];
        //Insert ids.a into memory
        vm.memory = memory![
            ((1, 7), ("850981239023189021389081239089023", 10)),
            ((1, 10), 0)
        ];
        //Execute the hint
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(VirtualMachineError::MemoryError(
                MemoryError::InconsistentMemory(
                    MaybeRelocatable::from((1, 10)),
                    MaybeRelocatable::from(Felt::zero()),
                    MaybeRelocatable::from(felt_str!("7249717543555297151"))
                )
            )))
        );
    }

    #[test]
    fn run_uint256_sqrt_ok() {
        let hint_code = "from starkware.python.math_utils import isqrt\nn = (ids.n.high << 128) + ids.n.low\nroot = isqrt(n)\nassert 0 <= root < 2 ** 128\nids.root.low = root\nids.root.high = 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("n", -5), ("root", 0)];
        vm.memory = memory![((1, 0), 17), ((1, 1), 7)];
        //Execute the hint
        assert_eq!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        //ids.root.low, ids.root.high
        check_memory![&vm.memory, ((1, 5), 48805497317890012913_u128), ((1, 6), 0)];
    }

    #[test]
    fn run_uint256_sqrt_assert_error() {
        let hint_code = "from starkware.python.math_utils import isqrt\nn = (ids.n.high << 128) + ids.n.low\nroot = isqrt(n)\nassert 0 <= root < 2 ** 128\nids.root.low = root\nids.root.high = 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("n", -5), ("root", 0)];
        vm.memory = memory![
            ((1, 0), 0),
            ((1, 1), ("340282366920938463463374607431768211458", 10))
        ];
        //Execute the hint
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::AssertionFailed(String::from(
                "assert 0 <= 340282366920938463463374607431768211456 < 2 ** 128"
            )))
        );
    }

    #[test]
    fn run_uint256_invalid_memory_insert() {
        let hint_code = "from starkware.python.math_utils import isqrt\nn = (ids.n.high << 128) + ids.n.low\nroot = isqrt(n)\nassert 0 <= root < 2 ** 128\nids.root.low = root\nids.root.high = 0";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 5;
        //Create hint_data
        let ids_data = non_continuous_ids_data![("n", -5), ("root", 0)];
        //Insert  ids.n.low into memory
        vm.memory = memory![((1, 0), 17), ((1, 1), 7), ((1, 5), 1)];
        //Execute the hint
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(VirtualMachineError::MemoryError(
                MemoryError::InconsistentMemory(
                    MaybeRelocatable::from((1, 5)),
                    MaybeRelocatable::from(Felt::one()),
                    MaybeRelocatable::from(felt_str!("48805497317890012913")),
                )
            )))
        );
    }

    #[test]
    fn run_signed_nn_ok_result_one() {
        let hint_code = "memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0";
        let mut vm = vm_with_range_check!();
        //Initialize run_context
        run_context!(vm, 0, 5, 4);
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -4)];
        //Insert ids.a.high into memory
        vm.memory = memory![(
            (1, 1),
            (
                "3618502788666131213697322783095070105793248398792065931704779359851756126208",
                10
            )
        )];
        //Execute the hint
        assert_eq!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory insert
        //memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0
        check_memory![&vm.memory, ((1, 5), 1)];
    }

    #[test]
    fn run_signed_nn_ok_result_zero() {
        let hint_code = "memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0";
        let mut vm = vm_with_range_check!();
        //Initialize run_context
        run_context!(vm, 0, 5, 4);
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -4)];
        //Insert ids.a.high into memory
        vm.memory = memory![(
            (1, 1),
            (
                "3618502788666131213697322783095070105793248398792065931704779359851756126209",
                10
            )
        )];
        //Execute the hint
        assert_eq!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory insert
        //memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0
        check_memory![&vm.memory, ((1, 5), 0)];
    }

    #[test]
    fn run_signed_nn_ok_invalid_memory_insert() {
        let hint_code = "memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0";
        let mut vm = vm_with_range_check!();
        //Initialize run_context
        run_context!(vm, 0, 5, 4);
        //Create hint_data
        let ids_data = non_continuous_ids_data![("a", -4)];
        vm.memory = memory![((1, 1), 1), ((1, 5), 55)];
        //Execute the hint
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(VirtualMachineError::MemoryError(
                MemoryError::InconsistentMemory(
                    MaybeRelocatable::from((1, 5)),
                    MaybeRelocatable::from(Felt::new(55)),
                    MaybeRelocatable::from(Felt::one()),
                )
            )))
        );
    }

    #[test]
    fn run_unsigned_div_rem_ok() {
        let hint_code = "a = (ids.a.high << 128) + ids.a.low\ndiv = (ids.div.high << 128) + ids.div.low\nquotient, remainder = divmod(a, div)\n\nids.quotient.low = quotient & ((1 << 128) - 1)\nids.quotient.high = quotient >> 128\nids.remainder.low = remainder & ((1 << 128) - 1)\nids.remainder.high = remainder >> 128";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -6), ("div", -4), ("quotient", 0), ("remainder", 2)];
        //Insert ids into memory
        vm.memory = memory![((1, 4), 89), ((1, 5), 72), ((1, 6), 3), ((1, 7), 7)];
        //Execute the hint
        assert_eq!(run_hint!(vm, ids_data, hint_code), Ok(()));
        //Check hint memory inserts
        //ids.quotient.low, ids.quotient.high, ids.remainder.low, ids.remainder.high
        check_memory![
            &vm.memory,
            ((1, 10), 10),
            ((1, 11), 0),
            ((1, 12), 59),
            ((1, 13), 2)
        ];
    }

    #[test]
    fn run_unsigned_div_rem_invalid_memory_insert() {
        let hint_code = "a = (ids.a.high << 128) + ids.a.low\ndiv = (ids.div.high << 128) + ids.div.low\nquotient, remainder = divmod(a, div)\n\nids.quotient.low = quotient & ((1 << 128) - 1)\nids.quotient.high = quotient >> 128\nids.remainder.low = remainder & ((1 << 128) - 1)\nids.remainder.high = remainder >> 128";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 10;
        //Create hint_data
        let ids_data =
            non_continuous_ids_data![("a", -6), ("div", -4), ("quotient", 0), ("remainder", 2)];
        //Insert ids into memory
        vm.memory = memory![
            ((1, 4), 89),
            ((1, 5), 72),
            ((1, 6), 3),
            ((1, 7), 7),
            ((1, 10), 0)
        ];
        //Execute the hint
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Internal(VirtualMachineError::MemoryError(
                MemoryError::InconsistentMemory(
                    MaybeRelocatable::from((1, 10)),
                    MaybeRelocatable::from(Felt::zero()),
                    MaybeRelocatable::from(Felt::new(10)),
                )
            )))
        );
    }
}
