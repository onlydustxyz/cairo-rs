use crate::any_box;
use crate::hint_processor::builtin_hint_processor::blake2s_utils::{
    blake2s_add_uint256, blake2s_add_uint256_bigend, compute_blake2s, finalize_blake2s,
};
use crate::hint_processor::builtin_hint_processor::dict_hint_utils::{
    default_dict_new, dict_new, dict_read, dict_squash_copy_dict, dict_squash_update_ptr,
    dict_update, dict_write,
};
use crate::hint_processor::builtin_hint_processor::find_element_hint::{
    find_element, search_sorted_lower,
};
use crate::hint_processor::builtin_hint_processor::hint_code;
use crate::hint_processor::builtin_hint_processor::keccak_utils::{
    unsafe_keccak, unsafe_keccak_finalize,
};
use crate::hint_processor::builtin_hint_processor::math_utils::*;
use crate::hint_processor::builtin_hint_processor::memcpy_hint_utils::{
    add_segment, enter_scope, exit_scope, memcpy_continue_copying, memcpy_enter_scope,
};
use crate::hint_processor::builtin_hint_processor::memset_utils::{
    memset_continue_loop, memset_enter_scope,
};
use crate::hint_processor::builtin_hint_processor::pow_utils::pow;
use crate::hint_processor::builtin_hint_processor::set::set_add;
use crate::hint_processor::builtin_hint_processor::squash_dict_utils::{
    squash_dict, squash_dict_inner_assert_len_keys, squash_dict_inner_check_access_index,
    squash_dict_inner_continue_loop, squash_dict_inner_first_iteration,
    squash_dict_inner_len_assert, squash_dict_inner_next_key, squash_dict_inner_skip_loop,
    squash_dict_inner_used_accesses_assert,
};
use crate::hint_processor::builtin_hint_processor::uint256_utils::{
    split_64, uint256_add, uint256_signed_nn, uint256_sqrt, uint256_unsigned_div_rem,
};
use crate::hint_processor::hint_processor_definition::{HintProcessor, HintReference};
use crate::serde::deserialize_program::ApTracking;
use crate::types::exec_scope::ExecutionScopes;
use crate::vm::errors::vm_errors::VirtualMachineError;
use crate::vm::vm_core::VirtualMachine;
use num_bigint::BigInt;
use std::any::Any;
use std::collections::HashMap;

use crate::hint_processor::builtin_hint_processor::cairo_keccak::keccak_hints::{
    block_permutation, cairo_keccak_finalize, compare_bytes_in_word_nondet,
    compare_keccak_full_rate_in_bytes_nondet, keccak_write_args,
};
use crate::hint_processor::builtin_hint_processor::secp::{
    bigint_utils::{bigint_to_uint256, nondet_bigint3},
    ec_utils::{
        compute_doubling_slope, compute_slope, ec_double_assign_new_x, ec_double_assign_new_y,
        ec_mul_inner, ec_negate, fast_ec_add_assign_new_x, fast_ec_add_assign_new_y,
    },
    field_utils::{
        is_zero_assign_scope_variables, is_zero_nondet, is_zero_pack, reduce, verify_zero,
    },
    signature::{div_mod_n_packed_divmod, div_mod_n_safe_div, get_point_from_x},
};
use crate::hint_processor::builtin_hint_processor::sha256_utils::{
    sha256_finalize, sha256_input, sha256_main,
};
use crate::hint_processor::builtin_hint_processor::usort::{
    usort_body, usort_enter_scope, verify_multiplicity_assert, verify_multiplicity_body,
    verify_usort,
};

pub struct HintProcessorData {
    pub code: String,
    pub ap_tracking: ApTracking,
    pub ids_data: HashMap<String, HintReference>,
}

impl HintProcessorData {
    pub fn new_default(code: String, ids_data: HashMap<String, HintReference>) -> Self {
        HintProcessorData {
            code,
            ap_tracking: ApTracking::default(),
            ids_data,
        }
    }
}

#[allow(clippy::type_complexity)]
pub struct HintFunc(
    pub  Box<
        dyn Fn(
                &mut VirtualMachine,
                &mut ExecutionScopes,
                &HashMap<String, HintReference>,
                &ApTracking,
                &HashMap<String, BigInt>,
            ) -> Result<(), VirtualMachineError>
            + Sync,
    >,
);
pub struct BuiltinHintProcessor {
    pub extra_hints: HashMap<String, HintFunc>,
}
impl BuiltinHintProcessor {
    pub fn new_empty() -> Self {
        BuiltinHintProcessor {
            extra_hints: HashMap::new(),
        }
    }

    pub fn new(extra_hints: HashMap<String, HintFunc>) -> Self {
        BuiltinHintProcessor { extra_hints }
    }

    pub fn add_hint(&mut self, hint_code: String, hint_func: HintFunc) {
        self.extra_hints.insert(hint_code, hint_func);
    }
}

impl HintProcessor for BuiltinHintProcessor {
    fn execute_hint(
        &self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
        constants: &HashMap<String, BigInt>,
    ) -> Result<(), VirtualMachineError> {
        let hint_data = hint_data
            .downcast_ref::<HintProcessorData>()
            .ok_or(VirtualMachineError::WrongHintData)?;

        if let Some(hint_func) = self.extra_hints.get(&hint_data.code) {
            return hint_func.0(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            );
        }

        match &*hint_data.code {
            hint_code::ADD_SEGMENT => add_segment(vm),
            hint_code::IS_NN => is_nn(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::IS_NN_OUT_OF_RANGE => {
                is_nn_out_of_range(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::IS_LE_FELT => is_le_felt(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::ASSERT_LE_FELT => {
                assert_le_felt(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::ASSERT_250_BITS => {
                assert_250_bit(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::IS_POSITIVE => is_positive(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::SPLIT_INT_ASSERT_RANGE => {
                split_int_assert_range(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::SPLIT_INT => split_int(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::ASSERT_NOT_EQUAL => {
                assert_not_equal(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::ASSERT_NN => assert_nn(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::SQRT => sqrt(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::ASSERT_NOT_ZERO => {
                assert_not_zero(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::VM_EXIT_SCOPE => exit_scope(exec_scopes),
            hint_code::MEMCPY_ENTER_SCOPE => {
                memcpy_enter_scope(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::MEMSET_ENTER_SCOPE => {
                memset_enter_scope(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::MEMCPY_CONTINUE_COPYING => memcpy_continue_copying(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::MEMSET_CONTINUE_LOOP => {
                memset_continue_loop(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::SPLIT_FELT => split_felt(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::UNSIGNED_DIV_REM => {
                unsigned_div_rem(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::SIGNED_DIV_REM => {
                signed_div_rem(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::ASSERT_LT_FELT => {
                assert_lt_felt(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::FIND_ELEMENT => {
                find_element(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::SEARCH_SORTED_LOWER => {
                search_sorted_lower(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::POW => pow(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::SET_ADD => set_add(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::DICT_NEW => dict_new(vm, exec_scopes),
            hint_code::DICT_READ => {
                dict_read(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::DICT_WRITE => {
                dict_write(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::DEFAULT_DICT_NEW => {
                default_dict_new(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::SQUASH_DICT_INNER_FIRST_ITERATION => squash_dict_inner_first_iteration(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::USORT_ENTER_SCOPE => usort_enter_scope(exec_scopes),
            hint_code::USORT_BODY => {
                usort_body(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::USORT_VERIFY => {
                verify_usort(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::USORT_VERIFY_MULTIPLICITY_ASSERT => verify_multiplicity_assert(exec_scopes),
            hint_code::USORT_VERIFY_MULTIPLICITY_BODY => verify_multiplicity_body(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::BLAKE2S_COMPUTE => {
                compute_blake2s(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::VERIFY_ZERO => {
                verify_zero(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::NONDET_BIGINT3 => nondet_bigint3(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::REDUCE => reduce(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::BLAKE2S_FINALIZE => {
                finalize_blake2s(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::BLAKE2S_ADD_UINT256 => {
                blake2s_add_uint256(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::BLAKE2S_ADD_UINT256_BIGEND => {
                blake2s_add_uint256_bigend(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UNSAFE_KECCAK => {
                unsafe_keccak(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UNSAFE_KECCAK_FINALIZE => {
                unsafe_keccak_finalize(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::SQUASH_DICT_INNER_SKIP_LOOP => squash_dict_inner_skip_loop(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::SQUASH_DICT_INNER_CHECK_ACCESS_INDEX => {
                squash_dict_inner_check_access_index(
                    vm,
                    exec_scopes,
                    &hint_data.ids_data,
                    &hint_data.ap_tracking,
                )
            }
            hint_code::SQUASH_DICT_INNER_CONTINUE_LOOP => squash_dict_inner_continue_loop(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::SQUASH_DICT_INNER_ASSERT_LEN_KEYS => {
                squash_dict_inner_assert_len_keys(exec_scopes)
            }
            hint_code::SQUASH_DICT_INNER_LEN_ASSERT => squash_dict_inner_len_assert(exec_scopes),
            hint_code::SQUASH_DICT_INNER_USED_ACCESSES_ASSERT => {
                squash_dict_inner_used_accesses_assert(
                    vm,
                    exec_scopes,
                    &hint_data.ids_data,
                    &hint_data.ap_tracking,
                )
            }
            hint_code::SQUASH_DICT_INNER_NEXT_KEY => squash_dict_inner_next_key(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::SQUASH_DICT => {
                squash_dict(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::VM_ENTER_SCOPE => enter_scope(exec_scopes),
            hint_code::DICT_UPDATE => {
                dict_update(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::DICT_SQUASH_COPY_DICT => {
                dict_squash_copy_dict(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::DICT_SQUASH_UPDATE_PTR => {
                dict_squash_update_ptr(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UINT256_ADD => uint256_add(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::SPLIT_64 => split_64(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::UINT256_SQRT => {
                uint256_sqrt(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UINT256_SIGNED_NN => {
                uint256_signed_nn(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UINT256_UNSIGNED_DIV_REM => {
                uint256_unsigned_div_rem(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::BIGINT_TO_UINT256 => {
                bigint_to_uint256(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::IS_ZERO_PACK => is_zero_pack(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::IS_ZERO_NONDET => is_zero_nondet(vm, exec_scopes),
            hint_code::IS_ZERO_ASSIGN_SCOPE_VARS => {
                is_zero_assign_scope_variables(exec_scopes, constants)
            }
            hint_code::DIV_MOD_N_PACKED_DIVMOD => div_mod_n_packed_divmod(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::DIV_MOD_N_SAFE_DIV => div_mod_n_safe_div(exec_scopes, constants),
            hint_code::GET_POINT_FROM_X => get_point_from_x(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::EC_NEGATE => ec_negate(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::EC_DOUBLE_SCOPE => compute_doubling_slope(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::COMPUTE_SLOPE => compute_slope(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::EC_DOUBLE_ASSIGN_NEW_X => ec_double_assign_new_x(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::EC_DOUBLE_ASSIGN_NEW_Y => ec_double_assign_new_y(exec_scopes, constants),
            hint_code::KECCAK_WRITE_ARGS => {
                keccak_write_args(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::COMPARE_BYTES_IN_WORD_NONDET => compare_bytes_in_word_nondet(
                vm,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::SHA256_MAIN => sha256_main(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::SHA256_INPUT => {
                sha256_input(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::SHA256_FINALIZE => {
                sha256_finalize(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::COMPARE_KECCAK_FULL_RATE_IN_BYTES_NONDET => {
                compare_keccak_full_rate_in_bytes_nondet(
                    vm,
                    &hint_data.ids_data,
                    &hint_data.ap_tracking,
                    constants,
                )
            }
            hint_code::BLOCK_PERMUTATION => {
                block_permutation(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::CAIRO_KECCAK_FINALIZE => {
                cairo_keccak_finalize(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::FAST_EC_ADD_ASSIGN_NEW_X => fast_ec_add_assign_new_x(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::FAST_EC_ADD_ASSIGN_NEW_Y => fast_ec_add_assign_new_y(exec_scopes, constants),
            hint_code::EC_MUL_INNER => {
                ec_mul_inner(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            code => Err(VirtualMachineError::UnknownHint(code.to_string())),
        }
    }

    fn compile_hint(
        &self,
        code: &str,
        ap_tracking: &ApTracking,
        reference_ids: &HashMap<String, usize>,
        references: &HashMap<usize, HintReference>,
    ) -> Result<Box<dyn Any>, VirtualMachineError> {
        Ok(any_box!(HintProcessorData {
            code: code.to_string(),
            ap_tracking: ap_tracking.clone(),
            ids_data: get_ids_data(reference_ids, references)?,
        }))
    }
}

fn get_ids_data(
    reference_ids: &HashMap<String, usize>,
    references: &HashMap<usize, HintReference>,
) -> Result<HashMap<String, HintReference>, VirtualMachineError> {
    let mut ids_data = HashMap::<String, HintReference>::new();
    for (path, ref_id) in reference_ids {
        let name = path
            .rsplit('.')
            .next()
            .ok_or(VirtualMachineError::FailedToGetIds)?;
        ids_data.insert(
            name.to_string(),
            references
                .get(ref_id)
                .ok_or(VirtualMachineError::FailedToGetIds)?
                .clone(),
        );
    }
    Ok(ids_data)
}

#[cfg(test)]
mod tests {
    use crate::types::exec_scope::ExecutionScopes;
    use crate::types::relocatable::MaybeRelocatable;
    use crate::utils::test_utils::*;
    use crate::vm::errors::{exec_scope_errors::ExecScopeError, memory_errors::MemoryError};
    use crate::vm::vm_core::VirtualMachine;
    use crate::vm::vm_memory::memory::Memory;
    use crate::{any_box, bigint};
    use num_bigint::{BigInt, Sign};
    use std::any::Any;

    use super::*;
    use crate::hint_processor::hint_processor_definition::HintProcessor;

    #[test]
    fn run_alloc_hint_empty_memory() {
        let hint_code = "memory[ap] = segments.add()";
        let mut vm = vm!();
        add_segments!(vm, 1);
        //ids and references are not needed for this test
        run_hint!(vm, HashMap::new(), hint_code).expect("Error while executing hint");
        //first new segment is added
        assert_eq!(vm.segments.num_segments, 2);
        //new segment base (1,0) is inserted into ap (1,0)
        check_memory![vm.memory, ((1, 0), (1, 0))];
    }

    #[test]
    fn run_alloc_hint_preset_memory() {
        let hint_code = "memory[ap] = segments.add()";
        let mut vm = vm!();
        //Add 3 segments to the memory
        add_segments!(vm, 3);
        vm.run_context.ap = 6;
        //ids and references are not needed for this test
        run_hint!(vm, HashMap::new(), hint_code).expect("Error while executing hint");
        //Segment N°4 is added
        assert_eq!(vm.segments.num_segments, 4);
        //new segment base (3,0) is inserted into ap (1,6)
        check_memory![vm.memory, ((1, 6), (3, 0))];
    }

    #[test]
    fn run_alloc_hint_ap_is_not_empty() {
        let hint_code = "memory[ap] = segments.add()";
        let mut vm = vm!();
        //Add 3 segments to the memory
        add_segments!(vm, 3);
        vm.run_context.ap = 6;
        //Insert something into ap
        vm.memory = memory![((1, 6), (1, 6))];
        //ids and references are not needed for this test
        assert_eq!(
            run_hint!(vm, HashMap::new(), hint_code),
            Err(VirtualMachineError::MemoryError(
                MemoryError::InconsistentMemory(
                    MaybeRelocatable::from((1, 6)),
                    MaybeRelocatable::from((1, 6)),
                    MaybeRelocatable::from((3, 0))
                )
            ))
        );
    }

    #[test]
    fn run_unknown_hint() {
        let hint_code = "random_invalid_code";
        let mut vm = vm!();
        assert_eq!(
            run_hint!(vm, HashMap::new(), hint_code),
            Err(VirtualMachineError::UnknownHint(hint_code.to_string())),
        );
    }

    #[test]
    fn memcpy_enter_scope_valid() {
        let hint_code = "vm_enter_scope({'n': ids.len})";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 2;
        // insert ids.len into memory
        vm.memory = memory![((1, 1), 5)];
        let ids_data = ids_data!["len"];
        assert!(run_hint!(vm, ids_data, hint_code).is_ok());
    }

    #[test]
    fn memcpy_enter_scope_invalid() {
        let hint_code = "vm_enter_scope({'n': ids.len})";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 2;
        // insert ids.len into memory
        // we insert a relocatable value in the address of ids.len so that it raises an error.
        vm.memory = memory![((1, 1), (1, 0))];

        let ids_data = ids_data!["len"];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 1))
            ))
        );
    }

    #[test]
    fn memcpy_continue_copying_valid() {
        let hint_code = "n -= 1\nids.continue_copying = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 2;
        // initialize vm scope with variable `n`
        let mut exec_scopes = scope![("n", bigint!(1))];
        // initialize ids.continue_copying
        // we create a memory gap so that there is None in (1, 0), the actual addr of continue_copying
        vm.memory = memory![((1, 2), 5)];
        let ids_data = ids_data!["continue_copying"];
        assert!(run_hint!(vm, ids_data, hint_code, &mut exec_scopes).is_ok());
    }

    #[test]
    fn memcpy_continue_copying_variable_not_in_scope_error() {
        let hint_code = "n -= 1\nids.continue_copying = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 1);
        // initialize fp
        vm.run_context.fp = 3;
        // we don't initialize `n` now:
        // initialize ids
        vm.memory = memory![((0, 2), 5)];
        let ids_data = ids_data!["continue_copying"];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(VirtualMachineError::VariableNotInScopeError(
                "n".to_string()
            ))
        );
    }

    #[test]
    fn memcpy_continue_copying_insert_error() {
        let hint_code = "n -= 1\nids.continue_copying = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 2;
        // initialize with variable `n`
        let mut exec_scopes = scope![("n", bigint!(1))];
        // initialize ids.continue_copying
        // a value is written in the address so the hint cant insert value there
        vm.memory = memory![((1, 1), 5)];

        let ids_data = ids_data!["continue_copying"];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code, &mut exec_scopes),
            Err(VirtualMachineError::MemoryError(
                MemoryError::InconsistentMemory(
                    MaybeRelocatable::from((1, 1)),
                    MaybeRelocatable::from(bigint!(5)),
                    MaybeRelocatable::from(bigint!(0))
                )
            ))
        );
    }

    #[test]
    fn exit_scope_valid() {
        let hint_code = "vm_exit_scope()";
        let mut vm = vm!();
        // Create new vm scope with dummy variable
        let mut exec_scopes = ExecutionScopes::new();
        let a_value: Box<dyn Any> = Box::new(bigint!(1));
        exec_scopes.enter_scope(HashMap::from([(String::from("a"), a_value)]));
        // Initialize memory segments
        add_segments!(vm, 1);
        assert!(run_hint!(vm, HashMap::new(), hint_code, &mut exec_scopes).is_ok());
    }

    #[test]
    fn exit_scope_invalid() {
        let hint_code = "vm_exit_scope()";
        let mut vm = vm!();
        // new vm scope is not created so that the hint raises an error:
        // initialize memory segments
        add_segments!(vm, 1);
        assert_eq!(
            run_hint!(vm, HashMap::new(), hint_code),
            Err(VirtualMachineError::MainScopeError(
                ExecScopeError::ExitMainScopeError
            ))
        );
    }

    #[test]
    fn run_enter_scope() {
        let hint_code = "vm_enter_scope()";
        //Create vm
        let mut vm = vm!();
        let mut exec_scopes = ExecutionScopes::new();
        //Execute the hint
        assert_eq!(
            run_hint!(vm, HashMap::new(), hint_code, &mut exec_scopes),
            Ok(())
        );
        //Check exec_scopes
        assert_eq!(exec_scopes.data.len(), 2);
        assert!(exec_scopes.data[0].is_empty());
        assert!(exec_scopes.data[1].is_empty());
    }

    #[test]
    fn unsafe_keccak_valid() {
        let hint_code = "from eth_hash.auto import keccak\n\ndata, length = ids.data, ids.length\n\nif '__keccak_max_size' in globals():\n    assert length <= __keccak_max_size, \\\n        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \\\n        f'Got: length={length}.'\n\nkeccak_input = bytearray()\nfor word_i, byte_i in enumerate(range(0, length, 16)):\n    word = memory[data + word_i]\n    n_bytes = min(16, length - byte_i)\n    assert 0 <= word < 2 ** (8 * n_bytes)\n    keccak_input += word.to_bytes(n_bytes, 'big')\n\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 5;
        // insert ids into memory
        vm.memory = memory![
            ((1, 1), 3),
            ((2, 0), 1),
            ((2, 1), 1),
            ((2, 2), 1),
            ((1, 2), (2, 0)),
            ((1, 5), 0)
        ];
        let ids_data = ids_data!["length", "data", "high", "low"];
        let mut exec_scopes = scope![("__keccak_max_size", bigint!(500))];
        assert!(run_hint!(vm, ids_data, hint_code, &mut exec_scopes).is_ok());
    }

    #[test]
    fn unsafe_keccak_max_size() {
        let hint_code = "from eth_hash.auto import keccak\n\ndata, length = ids.data, ids.length\n\nif '__keccak_max_size' in globals():\n    assert length <= __keccak_max_size, \\\n        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \\\n        f'Got: length={length}.'\n\nkeccak_input = bytearray()\nfor word_i, byte_i in enumerate(range(0, length, 16)):\n    word = memory[data + word_i]\n    n_bytes = min(16, length - byte_i)\n    assert 0 <= word < 2 ** (8 * n_bytes)\n    keccak_input += word.to_bytes(n_bytes, 'big')\n\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 5;
        // insert ids into memory
        vm.memory = memory![
            ((1, 1), 5),
            ((2, 0), 1),
            ((2, 1), 1),
            ((2, 2), 1),
            ((1, 2), (2, 0))
        ];
        let ids_data = ids_data!["length", "data", "high", "low"];
        let mut exec_scopes = scope![("__keccak_max_size", bigint!(2))];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code, &mut exec_scopes),
            Err(VirtualMachineError::KeccakMaxSize(bigint!(5), bigint!(2)))
        );
    }

    #[test]
    fn unsafe_keccak_invalid_input_length() {
        let hint_code = "from eth_hash.auto import keccak\n\ndata, length = ids.data, ids.length\n\nif '__keccak_max_size' in globals():\n    assert length <= __keccak_max_size, \\\n        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \\\n        f'Got: length={length}.'\n\nkeccak_input = bytearray()\nfor word_i, byte_i in enumerate(range(0, length, 16)):\n    word = memory[data + word_i]\n    n_bytes = min(16, length - byte_i)\n    assert 0 <= word < 2 ** (8 * n_bytes)\n    keccak_input += word.to_bytes(n_bytes, 'big')\n\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 4;
        // insert ids into memory
        vm.memory = memory![
            ((1, 1), 18446744073709551616_i128),
            ((1, 5), 0),
            ((2, 0), 1),
            ((2, 1), 1),
            ((2, 2), 1),
            ((1, 2), (2, 0))
        ];
        let ids_data = ids_data!["length", "data", "high", "low"];
        assert!(run_hint!(vm, ids_data, hint_code).is_err());
    }

    #[test]
    fn unsafe_keccak_invalid_word_size() {
        let hint_code = "from eth_hash.auto import keccak\n\ndata, length = ids.data, ids.length\n\nif '__keccak_max_size' in globals():\n    assert length <= __keccak_max_size, \\\n        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \\\n        f'Got: length={length}.'\n\nkeccak_input = bytearray()\nfor word_i, byte_i in enumerate(range(0, length, 16)):\n    word = memory[data + word_i]\n    n_bytes = min(16, length - byte_i)\n    assert 0 <= word < 2 ** (8 * n_bytes)\n    keccak_input += word.to_bytes(n_bytes, 'big')\n\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 5;
        // insert ids into memory
        vm.memory = memory![
            ((1, 1), 3),
            ((1, 5), 0),
            ((2, 0), (-1)),
            ((2, 1), 1),
            ((2, 2), 1),
            ((1, 2), (2, 0))
        ];
        let ids_data = ids_data!["length", "data", "high", "low"];
        let mut exec_scopes = scope![("__keccak_max_size", bigint!(10))];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code, &mut exec_scopes),
            Err(VirtualMachineError::InvalidWordSize(bigint!(-1)))
        );
    }

    #[test]
    fn unsafe_keccak_finalize_valid() {
        let hint_code = "from eth_hash.auto import keccak\nkeccak_input = bytearray()\nn_elms = ids.keccak_state.end_ptr - ids.keccak_state.start_ptr\nfor word in memory.get_range(ids.keccak_state.start_ptr, n_elms):\n    keccak_input += word.to_bytes(16, 'big')\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 9;
        vm.memory = memory![
            ((1, 1), (1, 2)),
            ((1, 2), (1, 4)),
            ((1, 3), (1, 5)),
            ((1, 4), 1),
            ((1, 5), 2),
            ((1, 8), 0)
        ];
        let ids_data = non_continuous_ids_data![("keccak_state", -7), ("high", -3), ("low", -2)];
        assert!(run_hint!(vm, ids_data, hint_code).is_ok());
    }

    #[test]
    fn unsafe_keccak_finalize_nones_in_range() {
        let hint_code = "from eth_hash.auto import keccak\nkeccak_input = bytearray()\nn_elms = ids.keccak_state.end_ptr - ids.keccak_state.start_ptr\nfor word in memory.get_range(ids.keccak_state.start_ptr, n_elms):\n    keccak_input += word.to_bytes(16, 'big')\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 9;
        vm.memory = memory![
            ((1, 1), (1, 2)),
            ((1, 2), (1, 4)),
            ((1, 3), (1, 5)),
            ((1, 5), 2),
            ((1, 8), 0)
        ];
        let ids_data = non_continuous_ids_data![("keccak_state", -7), ("high", -3), ("low", -2)];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code),
            Err(VirtualMachineError::NoneInMemoryRange)
        );
    }

    #[test]
    fn unsafe_keccak_finalize_expected_integer_at_range() {
        let hint_code = "from eth_hash.auto import keccak\nkeccak_input = bytearray()\nn_elms = ids.keccak_state.end_ptr - ids.keccak_state.start_ptr\nfor word in memory.get_range(ids.keccak_state.start_ptr, n_elms):\n    keccak_input += word.to_bytes(16, 'big')\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 9;
        vm.memory = memory![
            ((1, 1), (1, 2)),
            ((1, 2), (1, 4)),
            ((1, 3), (1, 5)),
            ((1, 4), (1, 5)),
            ((1, 5), 2),
            ((1, 8), 0)
        ];
        let ids_data = non_continuous_ids_data![("keccak_state", -7), ("high", -3), ("low", -2)];
        assert!(run_hint!(vm, ids_data, hint_code).is_err());
    }
}
