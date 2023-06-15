use super::{
    ec_recover::{ec_recover_divmod_n_packed, ec_recover_sub_a_b},
    field_arithmetic::uint384_div,
    vrf::{fq::uint512_unsigned_div_rem, inv_mod_p_uint512::inv_mod_p_uint512},
};
use crate::{
    hint_processor::{
        builtin_hint_processor::{
            bigint::{bigint_pack_div_mod_hint, bigint_safe_div_hint},
            blake2s_utils::{
                blake2s_add_uint256, blake2s_add_uint256_bigend, compute_blake2s, finalize_blake2s,
            },
            cairo_keccak::keccak_hints::{
                block_permutation, cairo_keccak_finalize_v1, cairo_keccak_finalize_v2,
                compare_bytes_in_word_nondet, compare_keccak_full_rate_in_bytes_nondet,
                keccak_write_args,
            },
            dict_hint_utils::{
                default_dict_new, dict_new, dict_read, dict_squash_copy_dict,
                dict_squash_update_ptr, dict_update, dict_write,
            },
            ec_utils::{chained_ec_op_random_ec_point_hint, random_ec_point_hint, recover_y_hint},
            field_arithmetic::get_square_root,
            find_element_hint::{find_element, search_sorted_lower},
            garaga::get_felt_bitlenght,
            hint_code,
            keccak_utils::{
                split_input, split_n_bytes, split_output, split_output_mid_low_high, unsafe_keccak,
                unsafe_keccak_finalize,
            },
            math_utils::*,
            memcpy_hint_utils::{
                add_segment, enter_scope, exit_scope, memcpy_continue_copying, memcpy_enter_scope,
            },
            memset_utils::{memset_continue_loop, memset_enter_scope},
            poseidon_utils::{n_greater_than_10, n_greater_than_2},
            pow_utils::pow,
            secp::{
                bigint_utils::{bigint_to_uint256, hi_max_bitlen, nondet_bigint3},
                ec_utils::{
                    compute_doubling_slope, compute_slope, compute_slope_secp_p, di_bit,
                    ec_double_assign_new_x, ec_double_assign_new_y, ec_mul_inner, ec_negate,
                    fast_ec_add_assign_new_x, fast_ec_add_assign_new_y, import_secp256r1_p,
                    quad_bit,
                },
                field_utils::{
                    is_zero_assign_scope_variables, is_zero_assign_scope_variables_external_const,
                    is_zero_nondet, is_zero_pack, is_zero_pack_external_secp, reduce, verify_zero,
                    verify_zero_with_external_const,
                },
                signature::{
                    div_mod_n_packed_divmod, div_mod_n_packed_external_n, div_mod_n_safe_div,
                    get_point_from_x, pack_modn_div_modn,
                },
            },
            segments::{relocate_segment, temporary_array},
            set::set_add,
            sha256_utils::{sha256_finalize, sha256_input, sha256_main},
            signature::verify_ecdsa_signature,
            squash_dict_utils::{
                squash_dict, squash_dict_inner_assert_len_keys,
                squash_dict_inner_check_access_index, squash_dict_inner_continue_loop,
                squash_dict_inner_first_iteration, squash_dict_inner_len_assert,
                squash_dict_inner_next_key, squash_dict_inner_skip_loop,
                squash_dict_inner_used_accesses_assert,
            },
            uint256_utils::{
                split_64, uint128_add, uint256_add, uint256_expanded_unsigned_div_rem,
                uint256_mul_div_mod, uint256_signed_nn, uint256_sqrt, uint256_sub,
                uint256_unsigned_div_rem,
            },
            uint384::{
                add_no_uint384_check, uint384_signed_nn, uint384_split_128, uint384_sqrt,
                uint384_unsigned_div_rem, uint384_unsigned_div_rem_expanded,
            },
            uint384_extension::unsigned_div_rem_uint768_by_uint384,
            usort::{
                usort_body, usort_enter_scope, verify_multiplicity_assert,
                verify_multiplicity_body, verify_usort,
            },
        },
        hint_processor_definition::{HintProcessor, HintReference},
    },
    serde::deserialize_program::ApTracking,
    stdlib::{any::Any, cell::RefCell, collections::HashMap, prelude::*, rc::Rc},
    types::exec_scope::ExecutionScopes,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use felt::Felt252;
use rand::{rngs::SmallRng, SeedableRng};

#[cfg(feature = "skip_next_instruction_hint")]
use crate::hint_processor::builtin_hint_processor::skip_next_instruction::skip_next_instruction;

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
                &HashMap<String, Felt252>,
            ) -> Result<(), HintError>
            + Sync,
    >,
);
pub struct BuiltinHintProcessor {
    pub extra_hints: HashMap<String, Rc<HintFunc>>,
}
impl BuiltinHintProcessor {
    pub fn new_empty() -> Self {
        BuiltinHintProcessor {
            extra_hints: HashMap::new(),
        }
    }

    pub fn new(extra_hints: HashMap<String, Rc<HintFunc>>) -> Self {
        BuiltinHintProcessor { extra_hints }
    }

    pub fn add_hint(&mut self, hint_code: String, hint_func: Rc<HintFunc>) {
        self.extra_hints.insert(hint_code, hint_func);
    }
}

impl HintProcessor for BuiltinHintProcessor {
    fn execute_hint(
        &mut self,
        vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        hint_data: &Box<dyn Any>,
        constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        let hint_data = hint_data
            .downcast_ref::<HintProcessorData>()
            .ok_or(HintError::WrongHintData)?;

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
            hint_code::ASSERT_LE_FELT => assert_le_felt(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::ASSERT_LE_FELT_EXCLUDED_2 => assert_le_felt_excluded_2(exec_scopes),
            hint_code::ASSERT_LE_FELT_EXCLUDED_1 => assert_le_felt_excluded_1(vm, exec_scopes),
            hint_code::ASSERT_LE_FELT_EXCLUDED_0 => assert_le_felt_excluded_0(vm, exec_scopes),
            hint_code::IS_LE_FELT => is_le_felt(vm, &hint_data.ids_data, &hint_data.ap_tracking),
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
            hint_code::IS_QUAD_RESIDUE => {
                is_quad_residue(vm, &hint_data.ids_data, &hint_data.ap_tracking)
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
            hint_code::VERIFY_ZERO_V1 | hint_code::VERIFY_ZERO_V2 => {
                verify_zero(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::VERIFY_ZERO_EXTERNAL_SECP => verify_zero_with_external_const(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::NONDET_BIGINT3 => {
                nondet_bigint3(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::REDUCE => {
                reduce(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
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
            hint_code::UINT128_ADD => uint128_add(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::UINT256_SUB => uint256_sub(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::SPLIT_64 => split_64(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::UINT256_SQRT => {
                uint256_sqrt(vm, &hint_data.ids_data, &hint_data.ap_tracking, false)
            }
            hint_code::UINT256_SQRT_FELT => {
                uint256_sqrt(vm, &hint_data.ids_data, &hint_data.ap_tracking, true)
            }
            hint_code::UINT256_SIGNED_NN => {
                uint256_signed_nn(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UINT256_UNSIGNED_DIV_REM => {
                uint256_unsigned_div_rem(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UINT256_EXPANDED_UNSIGNED_DIV_REM => {
                uint256_expanded_unsigned_div_rem(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::BIGINT_TO_UINT256 => {
                bigint_to_uint256(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::IS_ZERO_PACK => {
                is_zero_pack(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::IS_ZERO_NONDET | hint_code::IS_ZERO_INT => is_zero_nondet(vm, exec_scopes),
            hint_code::IS_ZERO_PACK_EXTERNAL_SECP => is_zero_pack_external_secp(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::IS_ZERO_ASSIGN_SCOPE_VARS => is_zero_assign_scope_variables(exec_scopes),
            hint_code::IS_ZERO_ASSIGN_SCOPE_VARS_EXTERNAL_SECP => {
                is_zero_assign_scope_variables_external_const(exec_scopes)
            }
            hint_code::DIV_MOD_N_PACKED_DIVMOD_V1 => div_mod_n_packed_divmod(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::GET_FELT_BIT_LENGTH => {
                get_felt_bitlenght(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::BIGINT_PACK_DIV_MOD => bigint_pack_div_mod_hint(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::BIGINT_SAFE_DIV => {
                bigint_safe_div_hint(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::DIV_MOD_N_PACKED_DIVMOD_EXTERNAL_N => div_mod_n_packed_external_n(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::DIV_MOD_N_SAFE_DIV => div_mod_n_safe_div(exec_scopes, "a", "b", 0),
            hint_code::DIV_MOD_N_SAFE_DIV_PLUS_ONE => div_mod_n_safe_div(exec_scopes, "a", "b", 1),
            hint_code::GET_POINT_FROM_X => get_point_from_x(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                constants,
            ),
            hint_code::EC_NEGATE => {
                ec_negate(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::EC_DOUBLE_SCOPE => compute_doubling_slope(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                "point",
            ),
            hint_code::EC_DOUBLE_SCOPE_WHITELIST => compute_doubling_slope(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                "pt",
            ),
            hint_code::COMPUTE_SLOPE => compute_slope_secp_p(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                "point0",
                "point1",
            ),
            hint_code::COMPUTE_SLOPE_SECP256R1 => compute_slope(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                "point0",
                "point1",
            ),
            hint_code::IMPORT_SECP256R1_P => import_secp256r1_p(exec_scopes),
            hint_code::COMPUTE_SLOPE_WHITELIST => compute_slope_secp_p(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
                "pt0",
                "pt1",
            ),
            hint_code::EC_DOUBLE_ASSIGN_NEW_X_V1 | hint_code::EC_DOUBLE_ASSIGN_NEW_X_V2 => {
                ec_double_assign_new_x(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::EC_DOUBLE_ASSIGN_NEW_Y => ec_double_assign_new_y(exec_scopes),
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
            hint_code::BLOCK_PERMUTATION | hint_code::BLOCK_PERMUTATION_WHITELIST => {
                block_permutation(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::CAIRO_KECCAK_FINALIZE_V1 => {
                cairo_keccak_finalize_v1(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::CAIRO_KECCAK_FINALIZE_V2 => {
                cairo_keccak_finalize_v2(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::FAST_EC_ADD_ASSIGN_NEW_X => fast_ec_add_assign_new_x(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::FAST_EC_ADD_ASSIGN_NEW_Y => fast_ec_add_assign_new_y(exec_scopes),
            hint_code::EC_MUL_INNER => {
                ec_mul_inner(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::RELOCATE_SEGMENT => {
                relocate_segment(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::TEMPORARY_ARRAY => {
                temporary_array(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::VERIFY_ECDSA_SIGNATURE => {
                verify_ecdsa_signature(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::SPLIT_OUTPUT_0 => {
                split_output(vm, &hint_data.ids_data, &hint_data.ap_tracking, 0)
            }
            hint_code::SPLIT_OUTPUT_1 => {
                split_output(vm, &hint_data.ids_data, &hint_data.ap_tracking, 1)
            }
            hint_code::SPLIT_INPUT_3 => {
                split_input(vm, &hint_data.ids_data, &hint_data.ap_tracking, 3, 1)
            }
            hint_code::SPLIT_INPUT_6 => {
                split_input(vm, &hint_data.ids_data, &hint_data.ap_tracking, 6, 2)
            }
            hint_code::SPLIT_INPUT_9 => {
                split_input(vm, &hint_data.ids_data, &hint_data.ap_tracking, 9, 3)
            }
            hint_code::SPLIT_INPUT_12 => {
                split_input(vm, &hint_data.ids_data, &hint_data.ap_tracking, 12, 4)
            }
            hint_code::SPLIT_INPUT_15 => {
                split_input(vm, &hint_data.ids_data, &hint_data.ap_tracking, 15, 5)
            }
            hint_code::SPLIT_N_BYTES => {
                split_n_bytes(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::SPLIT_OUTPUT_MID_LOW_HIGH => {
                split_output_mid_low_high(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::NONDET_N_GREATER_THAN_10 => {
                n_greater_than_10(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::NONDET_N_GREATER_THAN_2 => {
                n_greater_than_2(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::RANDOM_EC_POINT => {
                random_ec_point_hint(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::CHAINED_EC_OP_RANDOM_EC_POINT => {
                chained_ec_op_random_ec_point_hint(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::RECOVER_Y => recover_y_hint(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::PACK_MODN_DIV_MODN => {
                pack_modn_div_modn(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::XS_SAFE_DIV => div_mod_n_safe_div(exec_scopes, "x", "s", 0),
            hint_code::UINT384_UNSIGNED_DIV_REM => {
                uint384_unsigned_div_rem(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UINT384_SPLIT_128 => {
                uint384_split_128(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::ADD_NO_UINT384_CHECK => {
                add_no_uint384_check(vm, &hint_data.ids_data, &hint_data.ap_tracking, constants)
            }
            hint_code::UINT384_UNSIGNED_DIV_REM_EXPANDED => {
                uint384_unsigned_div_rem_expanded(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UINT384_SQRT => {
                uint384_sqrt(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UNSIGNED_DIV_REM_UINT768_BY_UINT384 => {
                unsigned_div_rem_uint768_by_uint384(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::GET_SQUARE_ROOT => {
                let rng = Rc::new(RefCell::new(SmallRng::seed_from_u64(42)));
                get_square_root(vm, &hint_data.ids_data, &hint_data.ap_tracking, rng)
            }
            hint_code::UINT384_SIGNED_NN => {
                uint384_signed_nn(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UINT384_DIV => uint384_div(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::UINT256_MUL_DIV_MOD => {
                uint256_mul_div_mod(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::UINT512_UNSIGNED_DIV_REM => {
                uint512_unsigned_div_rem(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::HI_MAX_BITLEN => {
                hi_max_bitlen(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::QUAD_BIT => quad_bit(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::INV_MOD_P_UINT512 => {
                inv_mod_p_uint512(vm, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            hint_code::DI_BIT => di_bit(vm, &hint_data.ids_data, &hint_data.ap_tracking),
            hint_code::EC_RECOVER_DIV_MOD_N_PACKED => ec_recover_divmod_n_packed(
                vm,
                exec_scopes,
                &hint_data.ids_data,
                &hint_data.ap_tracking,
            ),
            hint_code::EC_RECOVER_SUB_A_B => {
                ec_recover_sub_a_b(vm, exec_scopes, &hint_data.ids_data, &hint_data.ap_tracking)
            }
            #[cfg(feature = "skip_next_instruction_hint")]
            hint_code::SKIP_NEXT_INSTRUCTION => skip_next_instruction(vm),
            code => Err(HintError::UnknownHint(code.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::any::Any;
    use crate::types::relocatable::Relocatable;

    use crate::{
        any_box,
        hint_processor::hint_processor_definition::HintProcessor,
        types::{exec_scope::ExecutionScopes, relocatable::MaybeRelocatable},
        utils::test_utils::*,
        vm::{
            errors::{exec_scope_errors::ExecScopeError, memory_errors::MemoryError},
            vm_core::VirtualMachine,
        },
    };
    use assert_matches::assert_matches;
    use num_traits::{One, Zero};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_alloc_hint_empty_memory() {
        let hint_code = "memory[ap] = segments.add()";
        let mut vm = vm!();
        add_segments!(vm, 1);
        //ids and references are not needed for this test
        run_hint!(vm, HashMap::new(), hint_code).expect("Error while executing hint");
        //first new segment is added
        assert_eq!(vm.segments.num_segments(), 2);
        //new segment base (1,0) is inserted into ap (1,0)
        check_memory![vm.segments.memory, ((1, 0), (1, 0))];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_alloc_hint_preset_memory() {
        let hint_code = "memory[ap] = segments.add()";
        let mut vm = vm!();
        //Add 3 segments to the memory
        add_segments!(vm, 3);
        vm.run_context.ap = 6;
        //ids and references are not needed for this test
        run_hint!(vm, HashMap::new(), hint_code).expect("Error while executing hint");
        //Segment N°4 is added
        assert_eq!(vm.segments.num_segments(), 4);
        //new segment base (3,0) is inserted into ap (1,6)
        check_memory![vm.segments.memory, ((1, 6), (3, 0))];
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_alloc_hint_ap_is_not_empty() {
        let hint_code = "memory[ap] = segments.add()";
        let mut vm = vm!();
        vm.run_context.ap = 6;
        //Insert something into ap
        vm.segments = segments![((1, 6), (1, 6))];
        //Add 1 extra segment to the memory
        add_segments!(vm, 1);
        //ids and references are not needed for this test
        assert_matches!(
                    run_hint!(vm, HashMap::new(), hint_code),
                    Err(HintError::Memory(
                        MemoryError::InconsistentMemory(
                            x,
                            y,
                            z
                        )
                    )) if x ==
        Relocatable::from((1, 6)) &&
                            y == MaybeRelocatable::from((1, 6)) &&
                            z == MaybeRelocatable::from((3, 0))
                );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_unknown_hint() {
        let hint_code = "random_invalid_code";
        let mut vm = vm!();
        assert_matches!(
            run_hint!(vm, HashMap::new(), hint_code),
            Err(HintError::UnknownHint(x)) if x == *hint_code.to_string()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn memcpy_enter_scope_valid() {
        let hint_code = "vm_enter_scope({'n': ids.len})";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 2;
        // insert ids.len into memory
        vm.segments = segments![((1, 1), 5)];
        let ids_data = ids_data!["len"];
        assert!(run_hint!(vm, ids_data, hint_code).is_ok());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn memcpy_enter_scope_invalid() {
        let hint_code = "vm_enter_scope({'n': ids.len})";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 2;
        // insert ids.len into memory
        // we insert a relocatable value in the address of ids.len so that it raises an error.
        vm.segments = segments![((1, 1), (1, 0))];

        let ids_data = ids_data!["len"];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::IdentifierNotInteger(x, y))
            if x == "len" && y == (1,1).into()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn memcpy_continue_copying_valid() {
        let hint_code = "n -= 1\nids.continue_copying = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 2;
        // initialize vm scope with variable `n`
        let mut exec_scopes = scope![("n", Felt252::one())];
        // initialize ids.continue_copying
        // we create a memory gap so that there is None in (1, 0), the actual addr of continue_copying
        vm.segments = segments![((1, 2), 5)];
        let ids_data = ids_data!["continue_copying"];
        assert!(run_hint!(vm, ids_data, hint_code, &mut exec_scopes).is_ok());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn memcpy_continue_copying_variable_not_in_scope_error() {
        let hint_code = "n -= 1\nids.continue_copying = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 1);
        // initialize fp
        vm.run_context.fp = 3;
        // we don't initialize `n` now:
        // initialize ids
        vm.segments = segments![((0, 2), 5)];
        let ids_data = ids_data!["continue_copying"];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::VariableNotInScopeError(x)) if x == *"n".to_string()
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn memcpy_continue_copying_insert_error() {
        let hint_code = "n -= 1\nids.continue_copying = 1 if n > 0 else 0";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 2;
        // initialize with variable `n`
        let mut exec_scopes = scope![("n", Felt252::one())];
        // initialize ids.continue_copying
        // a value is written in the address so the hint cant insert value there
        vm.segments = segments![((1, 1), 5)];

        let ids_data = ids_data!["continue_copying"];
        assert_matches!(
                    run_hint!(vm, ids_data, hint_code, &mut exec_scopes),
                    Err(HintError::Memory(
                        MemoryError::InconsistentMemory(
                            x,
                            y,
                            z
                        )
                    )) if x ==
        Relocatable::from((1, 1)) &&
                            y == MaybeRelocatable::from(Felt252::new(5)) &&
                            z == MaybeRelocatable::from(Felt252::zero())
                );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn exit_scope_valid() {
        let hint_code = "vm_exit_scope()";
        let mut vm = vm!();
        // Create new vm scope with dummy variable
        let mut exec_scopes = ExecutionScopes::new();
        let a_value: Box<dyn Any> = Box::new(Felt252::one());
        exec_scopes.enter_scope(HashMap::from([(String::from("a"), a_value)]));
        // Initialize memory segments
        add_segments!(vm, 1);
        assert!(run_hint!(vm, HashMap::new(), hint_code, &mut exec_scopes).is_ok());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn exit_scope_invalid() {
        let hint_code = "vm_exit_scope()";
        let mut vm = vm!();
        // new vm scope is not created so that the hint raises an error:
        // initialize memory segments
        add_segments!(vm, 1);
        assert_matches!(
            run_hint!(vm, HashMap::new(), hint_code),
            Err(HintError::FromScopeError(
                ExecScopeError::ExitMainScopeError
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_enter_scope() {
        let hint_code = "vm_enter_scope()";
        //Create vm
        let mut vm = vm!();
        let mut exec_scopes = ExecutionScopes::new();
        //Execute the hint
        assert_matches!(
            run_hint!(vm, HashMap::new(), hint_code, &mut exec_scopes),
            Ok(())
        );
        //Check exec_scopes
        assert_eq!(exec_scopes.data.len(), 2);
        assert!(exec_scopes.data[0].is_empty());
        assert!(exec_scopes.data[1].is_empty());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsafe_keccak_valid() {
        let hint_code = "from eth_hash.auto import keccak\n\ndata, length = ids.data, ids.length\n\nif '__keccak_max_size' in globals():\n    assert length <= __keccak_max_size, \\\n        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \\\n        f'Got: length={length}.'\n\nkeccak_input = bytearray()\nfor word_i, byte_i in enumerate(range(0, length, 16)):\n    word = memory[data + word_i]\n    n_bytes = min(16, length - byte_i)\n    assert 0 <= word < 2 ** (8 * n_bytes)\n    keccak_input += word.to_bytes(n_bytes, 'big')\n\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 5;
        // insert ids into memory
        vm.segments = segments![
            ((1, 1), 3),
            ((2, 0), 1),
            ((2, 1), 1),
            ((2, 2), 1),
            ((1, 2), (2, 0)),
            ((1, 5), 0)
        ];
        let ids_data = ids_data!["length", "data", "high", "low"];
        let mut exec_scopes = scope![("__keccak_max_size", Felt252::new(500))];
        assert!(run_hint!(vm, ids_data, hint_code, &mut exec_scopes).is_ok());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsafe_keccak_max_size() {
        let hint_code = "from eth_hash.auto import keccak\n\ndata, length = ids.data, ids.length\n\nif '__keccak_max_size' in globals():\n    assert length <= __keccak_max_size, \\\n        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \\\n        f'Got: length={length}.'\n\nkeccak_input = bytearray()\nfor word_i, byte_i in enumerate(range(0, length, 16)):\n    word = memory[data + word_i]\n    n_bytes = min(16, length - byte_i)\n    assert 0 <= word < 2 ** (8 * n_bytes)\n    keccak_input += word.to_bytes(n_bytes, 'big')\n\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 5;
        // insert ids into memory
        vm.segments = segments![
            ((1, 1), 5),
            ((2, 0), 1),
            ((2, 1), 1),
            ((2, 2), 1),
            ((1, 2), (2, 0))
        ];
        let ids_data = ids_data!["length", "data", "high", "low"];
        let mut exec_scopes = scope![("__keccak_max_size", Felt252::new(2))];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code, &mut exec_scopes),
            Err(HintError::KeccakMaxSize(x, y)) if x == Felt252::new(5) && y == Felt252::new(2)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsafe_keccak_invalid_input_length() {
        let hint_code = "from eth_hash.auto import keccak\n\ndata, length = ids.data, ids.length\n\nif '__keccak_max_size' in globals():\n    assert length <= __keccak_max_size, \\\n        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \\\n        f'Got: length={length}.'\n\nkeccak_input = bytearray()\nfor word_i, byte_i in enumerate(range(0, length, 16)):\n    word = memory[data + word_i]\n    n_bytes = min(16, length - byte_i)\n    assert 0 <= word < 2 ** (8 * n_bytes)\n    keccak_input += word.to_bytes(n_bytes, 'big')\n\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 4;
        // insert ids into memory
        vm.segments = segments![
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsafe_keccak_invalid_word_size() {
        let hint_code = "from eth_hash.auto import keccak\n\ndata, length = ids.data, ids.length\n\nif '__keccak_max_size' in globals():\n    assert length <= __keccak_max_size, \\\n        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \\\n        f'Got: length={length}.'\n\nkeccak_input = bytearray()\nfor word_i, byte_i in enumerate(range(0, length, 16)):\n    word = memory[data + word_i]\n    n_bytes = min(16, length - byte_i)\n    assert 0 <= word < 2 ** (8 * n_bytes)\n    keccak_input += word.to_bytes(n_bytes, 'big')\n\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 3);
        // initialize fp
        vm.run_context.fp = 5;
        // insert ids into memory
        vm.segments = segments![
            ((1, 1), 3),
            ((1, 5), 0),
            ((2, 0), (-1)),
            ((2, 1), 1),
            ((2, 2), 1),
            ((1, 2), (2, 0))
        ];
        let ids_data = ids_data!["length", "data", "high", "low"];
        let mut exec_scopes = scope![("__keccak_max_size", Felt252::new(10))];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code, &mut exec_scopes),
            Err(HintError::InvalidWordSize(x)) if x == Felt252::new(-1)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsafe_keccak_finalize_valid() {
        let hint_code = "from eth_hash.auto import keccak\nkeccak_input = bytearray()\nn_elms = ids.keccak_state.end_ptr - ids.keccak_state.start_ptr\nfor word in memory.get_range(ids.keccak_state.start_ptr, n_elms):\n    keccak_input += word.to_bytes(16, 'big')\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 9;
        vm.segments = segments![
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsafe_keccak_finalize_nones_in_range() {
        let hint_code = "from eth_hash.auto import keccak\nkeccak_input = bytearray()\nn_elms = ids.keccak_state.end_ptr - ids.keccak_state.start_ptr\nfor word in memory.get_range(ids.keccak_state.start_ptr, n_elms):\n    keccak_input += word.to_bytes(16, 'big')\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 9;
        vm.segments = segments![
            ((1, 1), (1, 2)),
            ((1, 2), (1, 4)),
            ((1, 3), (1, 5)),
            ((1, 5), 2),
            ((1, 8), 0)
        ];
        let ids_data = non_continuous_ids_data![("keccak_state", -7), ("high", -3), ("low", -2)];
        assert_matches!(
            run_hint!(vm, ids_data, hint_code),
            Err(HintError::Memory(MemoryError::UnknownMemoryCell(_)))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn unsafe_keccak_finalize_expected_integer_at_range() {
        let hint_code = "from eth_hash.auto import keccak\nkeccak_input = bytearray()\nn_elms = ids.keccak_state.end_ptr - ids.keccak_state.start_ptr\nfor word in memory.get_range(ids.keccak_state.start_ptr, n_elms):\n    keccak_input += word.to_bytes(16, 'big')\nhashed = keccak(keccak_input)\nids.high = int.from_bytes(hashed[:16], 'big')\nids.low = int.from_bytes(hashed[16:32], 'big')";
        let mut vm = vm!();
        // initialize memory segments
        add_segments!(vm, 2);
        // initialize fp
        vm.run_context.fp = 9;
        vm.segments = segments![
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

    fn enter_scope(
        _vm: &mut VirtualMachine,
        exec_scopes: &mut ExecutionScopes,
        _ids_data: &HashMap<String, HintReference>,
        _ap_tracking: &ApTracking,
        _constants: &HashMap<String, Felt252>,
    ) -> Result<(), HintError> {
        exec_scopes.enter_scope(HashMap::new());
        Ok(())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_hint_add_same_hint_twice() {
        let mut hint_processor = BuiltinHintProcessor::new_empty();
        let hint_func = Rc::new(HintFunc(Box::new(enter_scope)));
        hint_processor.add_hint(String::from("enter_scope_custom_a"), Rc::clone(&hint_func));
        hint_processor.add_hint(String::from("enter_scope_custom_b"), hint_func);
        let mut vm = vm!();
        let exec_scopes = exec_scopes_ref!();
        assert_eq!(exec_scopes.data.len(), 1);
        let hint_data =
            HintProcessorData::new_default(String::from("enter_scope_custom_a"), HashMap::new());
        assert_matches!(
            hint_processor.execute_hint(
                &mut vm,
                exec_scopes,
                &any_box!(hint_data),
                &HashMap::new()
            ),
            Ok(())
        );
        assert_eq!(exec_scopes.data.len(), 2);
        let hint_data =
            HintProcessorData::new_default(String::from("enter_scope_custom_a"), HashMap::new());
        assert_matches!(
            hint_processor.execute_hint(
                &mut vm,
                exec_scopes,
                &any_box!(hint_data),
                &HashMap::new()
            ),
            Ok(())
        );
        assert_eq!(exec_scopes.data.len(), 3);
    }
}
