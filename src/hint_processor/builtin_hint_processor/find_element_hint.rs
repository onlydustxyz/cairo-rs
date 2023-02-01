use std::prelude::v1::*;

use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{
            get_integer_from_var_name, get_ptr_from_var_name, get_relocatable_from_var_name,
            insert_value_from_var_name,
        },
        hint_processor_definition::HintReference,
        hint_processor_utils::felt_to_usize,
    },
    serde::deserialize_program::ApTracking,
    types::exec_scope::ExecutionScopes,
    vm::{
        errors::{hint_errors::HintError, vm_errors::VirtualMachineError},
        vm_core::VirtualMachine,
    },
};
use felt::Felt;
use num_traits::{Signed, ToPrimitive};
use std::collections::HashMap;

pub fn find_element(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let key = get_integer_from_var_name("key", vm, ids_data, ap_tracking)?;
    let elm_size_bigint = get_integer_from_var_name("elm_size", vm, ids_data, ap_tracking)?;
    let n_elms = get_integer_from_var_name("n_elms", vm, ids_data, ap_tracking)?;
    let array_start = get_ptr_from_var_name("array_ptr", vm, ids_data, ap_tracking)?;
    let find_element_index = exec_scopes.get::<Felt>("find_element_index").ok();
    let elm_size = elm_size_bigint
        .to_usize()
        .ok_or_else(|| HintError::ValueOutOfRange(elm_size_bigint.as_ref().clone()))?;
    if elm_size == 0 {
        return Err(HintError::ValueOutOfRange(elm_size_bigint.into_owned()));
    }

    if let Some(find_element_index_value) = find_element_index {
        let find_element_index_usize = felt_to_usize(&find_element_index_value)?;
        let found_key = vm
            .get_integer(&(array_start + (elm_size * find_element_index_usize)))
            .map_err(|_| HintError::KeyNotFound)?;

        if found_key.as_ref() != key.as_ref() {
            return Err(HintError::InvalidIndex(
                find_element_index_value,
                key.into_owned(),
                found_key.into_owned(),
            ));
        }
        insert_value_from_var_name("index", find_element_index_value, vm, ids_data, ap_tracking)?;
        exec_scopes.delete_variable("find_element_index");
        Ok(())
    } else {
        if n_elms.is_negative() {
            return Err(HintError::ValueOutOfRange(n_elms.into_owned()));
        }

        if let Ok(find_element_max_size) = exec_scopes.get_ref::<Felt>("find_element_max_size") {
            if n_elms.as_ref() > find_element_max_size {
                return Err(HintError::FindElemMaxSize(
                    find_element_max_size.clone(),
                    n_elms.into_owned(),
                ));
            }
        }
        let n_elms_iter: i32 = n_elms
            .to_i32()
            .ok_or_else(|| VirtualMachineError::OffsetExceeded(n_elms.into_owned()))?;

        for i in 0..n_elms_iter {
            let iter_key = vm
                .get_integer(&(array_start + (elm_size * i as usize)))
                .map_err(|_| HintError::KeyNotFound)?;

            if iter_key.as_ref() == key.as_ref() {
                return insert_value_from_var_name(
                    "index",
                    Felt::new(i),
                    vm,
                    ids_data,
                    ap_tracking,
                );
            }
        }

        Err(HintError::NoValueForKeyFindElement(key.into_owned()))
    }
}

pub fn search_sorted_lower(
    vm: &mut VirtualMachine,
    exec_scopes: &mut ExecutionScopes,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let find_element_max_size = exec_scopes.get::<Felt>("find_element_max_size");
    let n_elms = get_integer_from_var_name("n_elms", vm, ids_data, ap_tracking)?;
    let rel_array_ptr = get_relocatable_from_var_name("array_ptr", vm, ids_data, ap_tracking)?;
    let elm_size = get_integer_from_var_name("elm_size", vm, ids_data, ap_tracking)?;
    let key = get_integer_from_var_name("key", vm, ids_data, ap_tracking)?;

    if !elm_size.is_positive() {
        return Err(HintError::ValueOutOfRange(elm_size.into_owned()));
    }

    if n_elms.is_negative() {
        return Err(HintError::ValueOutOfRange(n_elms.into_owned()));
    }

    if let Ok(find_element_max_size) = find_element_max_size {
        if n_elms.as_ref() > &find_element_max_size {
            return Err(HintError::FindElemMaxSize(
                find_element_max_size,
                n_elms.into_owned(),
            ));
        }
    }

    let mut array_iter = vm.get_relocatable(&rel_array_ptr)?;
    let n_elms_usize = n_elms.to_usize().ok_or(HintError::KeyNotFound)?;
    let elm_size_usize = elm_size.to_usize().ok_or(HintError::KeyNotFound)?;

    for i in 0..n_elms_usize {
        let value = vm.get_integer(&array_iter)?;
        if value.as_ref() >= key.as_ref() {
            return insert_value_from_var_name("index", Felt::new(i), vm, ids_data, ap_tracking);
        }
        array_iter.offset += elm_size_usize;
    }
    insert_value_from_var_name("index", n_elms.into_owned(), vm, ids_data, ap_tracking)
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
        types::relocatable::MaybeRelocatable,
        utils::test_utils::*,
        vm::vm_core::VirtualMachine,
    };
    use num_traits::{One, Zero};
    use std::any::Any;

    fn init_vm_ids_data(
        values_to_override: HashMap<String, MaybeRelocatable>,
    ) -> (VirtualMachine, HashMap<String, HintReference>) {
        let mut vm = vm!();

        const FP_OFFSET_START: usize = 4;
        vm.run_context.fp = FP_OFFSET_START;

        for _ in 0..3 {
            vm.segments.add(&mut vm.memory);
        }

        let addresses = vec![
            MaybeRelocatable::from((1, 0)),
            MaybeRelocatable::from((1, 1)),
            MaybeRelocatable::from((1, 2)),
            MaybeRelocatable::from((1, 4)),
            MaybeRelocatable::from((2, 0)),
            MaybeRelocatable::from((2, 1)),
            MaybeRelocatable::from((2, 2)),
            MaybeRelocatable::from((2, 3)),
        ];

        let default_values = vec![
            ("array_ptr", MaybeRelocatable::from((2, 0))),
            ("elm_size", MaybeRelocatable::from(Felt::new(2_i32))),
            ("n_elms", MaybeRelocatable::from(Felt::new(2_i32))),
            ("key", MaybeRelocatable::from(Felt::new(3_i32))),
            ("arr[0].a", MaybeRelocatable::from(Felt::one())),
            ("arr[0].b", MaybeRelocatable::from(Felt::new(2_i32))),
            ("arr[1].a", MaybeRelocatable::from(Felt::new(3_i32))),
            ("arr[1].b", MaybeRelocatable::from(Felt::new(4_i32))),
        ];

        /* array_ptr = (1,0) -> [Struct{1, 2}, Struct{3, 4}]
          elm_size = 2
          n_elms = 2
          index = None. Should become 1
          key = 3
        */

        // Build memory
        // default_values[i].0 -> contains name
        // default_values[i].1 -> contains maybe relocatable
        for (i, memory_cell) in addresses.iter().enumerate() {
            let value_to_insert = values_to_override
                .get(default_values[i].0)
                .unwrap_or(&default_values[i].1);
            vm.memory
                .insert(memory_cell, value_to_insert)
                .expect("Unexpected memory insert fail");
        }
        let mut ids_data = HashMap::<String, HintReference>::new();
        for (i, name) in ["array_ptr", "elm_size", "n_elms", "index", "key"]
            .iter()
            .enumerate()
        {
            ids_data.insert(
                name.to_string(),
                HintReference::new_simple(i as i32 - FP_OFFSET_START as i32),
            );
        }

        (vm, ids_data)
    }

    #[test]
    fn element_found_by_search() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT.to_string()),
            Ok(())
        );
        check_memory![vm.memory, ((1, 3), 1)];
    }

    #[test]
    fn element_found_by_oracle() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        let mut exec_scopes = scope![("find_element_index", Felt::one())];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT, &mut exec_scopes),
            Ok(())
        );
        check_memory![vm.memory, ((1, 3), 1)];
    }

    #[test]
    fn element_not_found_search() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "key".to_string(),
            MaybeRelocatable::from(Felt::new(7)),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::NoValueForKeyFindElement(Felt::new(7)))
        );
    }

    #[test]
    fn element_not_found_oracle() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        let mut exec_scopes = scope![("find_element_index", Felt::new(2))];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT, &mut exec_scopes),
            Err(HintError::KeyNotFound)
        );
    }

    #[test]
    fn find_elm_failed_ids_get_from_mem() {
        let mut vm = vm!();
        vm.run_context.fp = 5;
        let ids_data = ids_data!["array_ptr", "elm_size", "n_elms", "index", "key"];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::Internal(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 4))
            )))
        );
    }

    #[test]
    fn find_elm_not_int_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::from((7, 8)),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::Internal(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 1))
            )))
        );
    }

    #[test]
    fn find_elm_zero_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::Int(Felt::zero()),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::ValueOutOfRange(Felt::zero()))
        );
    }

    #[test]
    fn find_elm_negative_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::Int(Felt::new(-1)),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::ValueOutOfRange(Felt::new(-1)))
        );
    }

    #[test]
    fn find_elm_not_int_n_elms() {
        let relocatable = MaybeRelocatable::from((1, 2));
        let (mut vm, ids_data) =
            init_vm_ids_data(HashMap::from([("n_elms".to_string(), relocatable.clone())]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::Internal(VirtualMachineError::ExpectedInteger(
                relocatable
            )))
        );
    }

    #[test]
    fn find_elm_negative_n_elms() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "n_elms".to_string(),
            MaybeRelocatable::Int(Felt::new(-1)),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::ValueOutOfRange(Felt::new(-1)))
        );
    }

    #[test]
    fn find_elm_empty_scope() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        assert_eq!(run_hint!(vm, ids_data, hint_code::FIND_ELEMENT), Ok(()));
    }

    #[test]
    fn find_elm_n_elms_gt_max_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        let mut exec_scopes = scope![("find_element_max_size", Felt::one())];
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT, &mut exec_scopes),
            Err(HintError::FindElemMaxSize(Felt::one(), Felt::new(2)))
        );
    }

    #[test]
    fn find_elm_key_not_int() {
        let relocatable = MaybeRelocatable::from((1, 4));
        let (mut vm, ids_data) =
            init_vm_ids_data(HashMap::from([("key".to_string(), relocatable.clone())]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::FIND_ELEMENT),
            Err(HintError::Internal(VirtualMachineError::ExpectedInteger(
                relocatable
            )))
        );
    }

    #[test]
    fn search_sorted_lower() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Ok(())
        );

        check_memory![vm.memory, ((1, 3), 1)];
    }

    #[test]
    fn search_sorted_lower_no_matches() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "key".to_string(),
            MaybeRelocatable::Int(Felt::new(7)),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Ok(())
        );
        check_memory![vm.memory, ((1, 3), 2)];
    }

    #[test]
    fn search_sorted_lower_not_int_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::from((7, 8)),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Err(HintError::Internal(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 1))
            )))
        );
    }

    #[test]
    fn search_sorted_lower_zero_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::Int(Felt::zero()),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Err(HintError::ValueOutOfRange(Felt::zero()))
        );
    }

    #[test]
    fn search_sorted_lower_negative_elm_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "elm_size".to_string(),
            MaybeRelocatable::Int(Felt::new(-1)),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Err(HintError::ValueOutOfRange(Felt::new(-1)))
        );
    }

    #[test]
    fn search_sorted_lower_not_int_n_elms() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "n_elms".to_string(),
            MaybeRelocatable::from((2, 2)),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Err(HintError::Internal(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 2))
            )))
        );
    }

    #[test]
    fn search_sorted_lower_negative_n_elms() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::from([(
            "n_elms".to_string(),
            MaybeRelocatable::Int(Felt::new(-1)),
        )]));
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Err(HintError::ValueOutOfRange(Felt::new(-1)))
        );
    }

    #[test]
    fn search_sorted_lower_empty_scope() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        assert_eq!(
            run_hint!(vm, ids_data, hint_code::SEARCH_SORTED_LOWER),
            Ok(())
        );
    }

    #[test]
    fn search_sorted_lower_n_elms_gt_max_size() {
        let (mut vm, ids_data) = init_vm_ids_data(HashMap::new());
        let mut exec_scopes = scope![("find_element_max_size", Felt::one())];
        assert_eq!(
            run_hint!(
                vm,
                ids_data,
                hint_code::SEARCH_SORTED_LOWER,
                &mut exec_scopes
            ),
            Err(HintError::FindElemMaxSize(Felt::one(), Felt::new(2)))
        );
    }
}
