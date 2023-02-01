use std::prelude::v1::*;

use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{
            get_integer_from_var_name, get_ptr_from_var_name, insert_value_from_var_name,
        },
        hint_processor_utils::felt_to_u32,
    },
    serde::deserialize_program::ApTracking,
    types::relocatable::MaybeRelocatable,
    vm::errors::{hint_errors::HintError, vm_errors::VirtualMachineError},
    vm::vm_core::VirtualMachine,
};
use felt::Felt;
use generic_array::GenericArray;
use num_traits::{One, Zero};
use sha2::compress256;
use std::collections::HashMap;

use crate::hint_processor::hint_processor_definition::HintReference;

const SHA256_INPUT_CHUNK_SIZE_FELTS: usize = 16;
const SHA256_STATE_SIZE_FELTS: usize = 8;
const BLOCK_SIZE: usize = 7;
const IV: [u32; SHA256_STATE_SIZE_FELTS] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

pub fn sha256_input(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let n_bytes = get_integer_from_var_name("n_bytes", vm, ids_data, ap_tracking)?;
    let n_bytes = n_bytes.as_ref();

    insert_value_from_var_name(
        "full_word",
        if n_bytes >= &Felt::new(4_i32) {
            Felt::one()
        } else {
            Felt::zero()
        },
        vm,
        ids_data,
        ap_tracking,
    )
}

pub fn sha256_main(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let input_ptr = get_ptr_from_var_name("sha256_start", vm, ids_data, ap_tracking)?;

    let mut message: Vec<u8> = Vec::with_capacity(4 * SHA256_INPUT_CHUNK_SIZE_FELTS);

    for i in 0..SHA256_INPUT_CHUNK_SIZE_FELTS {
        let input_element = vm.get_integer(&(input_ptr + i))?;
        let bytes = felt_to_u32(input_element.as_ref())?.to_be_bytes();
        message.extend(bytes);
    }

    let mut iv = IV;
    let new_message = GenericArray::clone_from_slice(&message);
    compress256(&mut iv, &[new_message]);

    let mut output: Vec<MaybeRelocatable> = Vec::with_capacity(SHA256_STATE_SIZE_FELTS);

    for new_state in iv {
        output.push(Felt::new(new_state).into());
    }

    let output_base = get_ptr_from_var_name("output", vm, ids_data, ap_tracking)?;

    vm.write_arg(&output_base, &output)
        .map_err(VirtualMachineError::MemoryError)?;
    Ok(())
}

pub fn sha256_finalize(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let message: Vec<u8> = vec![0; 64];

    let mut iv = IV;

    let iv_static: Vec<MaybeRelocatable> = iv.iter().map(|n| Felt::new(*n).into()).collect();

    let new_message = GenericArray::clone_from_slice(&message);
    compress256(&mut iv, &[new_message]);

    let mut output: Vec<MaybeRelocatable> = Vec::with_capacity(SHA256_STATE_SIZE_FELTS);

    for new_state in iv {
        output.push(Felt::new(new_state).into());
    }

    let sha256_ptr_end = get_ptr_from_var_name("sha256_ptr_end", vm, ids_data, ap_tracking)?;

    let mut padding: Vec<MaybeRelocatable> = Vec::new();
    let zero_vector_message: Vec<MaybeRelocatable> = vec![Felt::zero().into(); 16];

    for _ in 0..BLOCK_SIZE - 1 {
        padding.extend_from_slice(zero_vector_message.as_slice());
        padding.extend_from_slice(iv_static.as_slice());
        padding.extend_from_slice(output.as_slice());
    }

    vm.write_arg(&sha256_ptr_end, &padding)
        .map_err(VirtualMachineError::MemoryError)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hint_processor::hint_processor_definition::HintReference,
        types::relocatable::MaybeRelocatable,
        utils::test_utils::*,
        vm::{
            errors::memory_errors::MemoryError, runners::builtin_runner::RangeCheckBuiltinRunner,
            vm_core::VirtualMachine, vm_memory::memory::Memory,
        },
    };

    #[test]
    fn sha256_input_one() {
        let mut vm = vm_with_range_check!();
        vm.memory = memory![((1, 1), 7)];
        vm.run_context.fp = 2;
        let ids_data = ids_data!["full_word", "n_bytes"];
        assert_eq!(sha256_input(&mut vm, &ids_data, &ApTracking::new()), Ok(()));

        check_memory![&vm.memory, ((1, 0), 1)];
    }

    #[test]
    fn sha256_input_zero() {
        let mut vm = vm_with_range_check!();
        vm.memory = memory![((1, 1), 3)];
        vm.run_context.fp = 2;
        let ids_data = ids_data!["full_word", "n_bytes"];
        assert_eq!(sha256_input(&mut vm, &ids_data, &ApTracking::new()), Ok(()));

        check_memory![&vm.memory, ((1, 0), 0)];
    }

    #[test]
    fn sha256_ok() {
        let mut vm = vm_with_range_check!();

        vm.memory = memory![
            ((1, 0), (2, 0)),
            ((1, 1), (3, 0)),
            ((2, 0), 22),
            ((2, 1), 22),
            ((2, 2), 22),
            ((2, 3), 22),
            ((2, 4), 22),
            ((2, 5), 22),
            ((2, 6), 22),
            ((2, 7), 22),
            ((2, 8), 22),
            ((2, 9), 22),
            ((2, 10), 22),
            ((2, 11), 22),
            ((2, 12), 22),
            ((2, 13), 22),
            ((2, 14), 22),
            ((2, 15), 22),
            ((3, 9), 0)
        ];
        vm.run_context.fp = 2;
        let ids_data = ids_data!["sha256_start", "output"];
        assert_eq!(sha256_main(&mut vm, &ids_data, &ApTracking::new()), Ok(()));

        check_memory![
            &vm.memory,
            ((3, 0), 3704205499_u32),
            ((3, 1), 2308112482_u32),
            ((3, 2), 3022351583_u32),
            ((3, 3), 174314172_u32),
            ((3, 4), 1762869695_u32),
            ((3, 5), 1649521060_u32),
            ((3, 6), 2811202336_u32),
            ((3, 7), 4231099170_u32)
        ];
    }
}
