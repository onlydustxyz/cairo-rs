use std::prelude::v1::*;

use felt::Felt;

use crate::hint_processor::hint_processor_definition::HintReference;
use crate::hint_processor::hint_processor_utils::compute_addr_from_reference;
use crate::hint_processor::hint_processor_utils::{
    get_integer_from_reference, get_maybe_relocatable_from_reference,
};
use crate::serde::deserialize_program::ApTracking;
use crate::types::relocatable::MaybeRelocatable;
use crate::types::relocatable::Relocatable;
use crate::vm::errors::hint_errors::HintError;
use crate::vm::vm_core::VirtualMachine;
use std::borrow::Cow;
use std::collections::HashMap;

//Inserts value into the address of the given ids variable
pub fn insert_value_from_var_name(
    var_name: &str,
    value: impl Into<MaybeRelocatable>,
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let var_address = get_relocatable_from_var_name(var_name, vm, ids_data, ap_tracking)?;
    vm.insert_value(&var_address, value)
        .map_err(HintError::Internal)
}

//Inserts value into ap
pub fn insert_value_into_ap(
    vm: &mut VirtualMachine,
    value: impl Into<MaybeRelocatable>,
) -> Result<(), HintError> {
    vm.insert_value(&vm.get_ap(), value)
        .map_err(HintError::Internal)
}

//Returns the Relocatable value stored in the given ids variable
pub fn get_ptr_from_var_name(
    var_name: &str,
    vm: &VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<Relocatable, HintError> {
    let var_addr = get_relocatable_from_var_name(var_name, vm, ids_data, ap_tracking)?;
    //Add immediate if present in reference
    let hint_reference = ids_data
        .get(&String::from(var_name))
        .ok_or(HintError::FailedToGetIds)?;
    if hint_reference.dereference {
        let value = vm.get_relocatable(&var_addr)?;
        Ok(value)
    } else {
        Ok(var_addr)
    }
}

//Gets the address, as a MaybeRelocatable of the variable given by the ids name
pub fn get_address_from_var_name(
    var_name: &str,
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<MaybeRelocatable, HintError> {
    Ok(MaybeRelocatable::from(compute_addr_from_reference(
        ids_data.get(var_name).ok_or(HintError::FailedToGetIds)?,
        vm,
        ap_tracking,
    )?))
}

//Gets the address, as a Relocatable of the variable given by the ids name
pub fn get_relocatable_from_var_name(
    var_name: &str,
    vm: &VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<Relocatable, HintError> {
    compute_addr_from_reference(
        ids_data.get(var_name).ok_or(HintError::FailedToGetIds)?,
        vm,
        ap_tracking,
    )
}

//Gets the value of a variable name.
//If the value is an MaybeRelocatable::Int(Bigint) return &Bigint
//else raises Err
pub fn get_integer_from_var_name<'a>(
    var_name: &str,
    vm: &'a VirtualMachine,
    ids_data: &'a HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<Cow<'a, Felt>, HintError> {
    let reference = get_reference_from_var_name(var_name, ids_data)?;
    get_integer_from_reference(vm, reference, ap_tracking)
}

//Gets the value of a variable name as a MaybeRelocatable
pub fn get_maybe_relocatable_from_var_name<'a>(
    var_name: &str,
    vm: &'a VirtualMachine,
    ids_data: &'a HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<MaybeRelocatable, HintError> {
    let reference = get_reference_from_var_name(var_name, ids_data)?;
    get_maybe_relocatable_from_reference(vm, reference, ap_tracking)
}

pub fn get_reference_from_var_name<'a>(
    var_name: &str,
    ids_data: &'a HashMap<String, HintReference>,
) -> Result<&'a HintReference, HintError> {
    ids_data.get(var_name).ok_or(HintError::FailedToGetIds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        hint_processor::hint_processor_definition::HintReference,
        relocatable,
        serde::deserialize_program::OffsetValue,
        utils::test_utils::*,
        vm::{
            errors::{memory_errors::MemoryError, vm_errors::VirtualMachineError},
            vm_core::VirtualMachine,
            vm_memory::memory::Memory,
        },
    };

    #[test]
    fn get_ptr_from_var_name_immediate_value() {
        let mut vm = vm!();
        vm.memory = memory![((1, 0), (0, 0))];
        let mut hint_ref = HintReference::new(0, 0, true, false);
        hint_ref.offset2 = OffsetValue::Value(2);
        let ids_data = HashMap::from([("imm".to_string(), hint_ref)]);

        assert_eq!(
            get_ptr_from_var_name("imm", &vm, &ids_data, &ApTracking::new()),
            Ok(relocatable!(0, 2))
        );
    }

    #[test]
    fn get_maybe_relocatable_from_var_name_valid() {
        let mut vm = vm!();
        vm.memory = memory![((1, 0), (0, 0))];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_eq!(
            get_maybe_relocatable_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Ok(mayberelocatable!(0, 0))
        );
    }

    #[test]
    fn get_maybe_relocatable_from_var_name_invalid() {
        let mut vm = vm!();
        vm.memory = Memory::new();
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_eq!(
            get_maybe_relocatable_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Err(HintError::FailedToGetIds)
        );
    }

    #[test]
    fn get_ptr_from_var_name_valid() {
        let mut vm = vm!();
        vm.memory = memory![((1, 0), (0, 0))];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_eq!(
            get_ptr_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Ok(relocatable!(0, 0))
        );
    }

    #[test]
    fn get_ptr_from_var_name_invalid() {
        let mut vm = vm!();
        vm.memory = memory![((1, 0), 0)];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_eq!(
            get_ptr_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Err(HintError::Internal(
                VirtualMachineError::ExpectedRelocatable(MaybeRelocatable::from((1, 0)))
            ))
        );
    }

    #[test]
    fn get_relocatable_from_var_name_valid() {
        let mut vm = vm!();
        vm.memory = memory![((1, 0), (0, 0))];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_eq!(
            get_relocatable_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Ok(relocatable!(1, 0))
        );
    }

    #[test]
    fn get_relocatable_from_var_name_invalid() {
        let mut vm = vm!();
        vm.memory = Memory::new();
        let hint_ref = HintReference::new_simple(-8);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_eq!(
            get_relocatable_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Err(HintError::FailedToGetIds)
        );
    }

    #[test]
    fn get_integer_from_var_name_valid() {
        let mut vm = vm!();
        vm.memory = memory![((1, 0), 1)];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_eq!(
            get_integer_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Ok(Cow::Borrowed(&Felt::new(1)))
        );
    }

    #[test]
    fn get_integer_from_var_name_invalid() {
        let mut vm = vm!();
        vm.memory = memory![((1, 0), (0, 0))];
        let hint_ref = HintReference::new_simple(0);
        let ids_data = HashMap::from([("value".to_string(), hint_ref)]);

        assert_eq!(
            get_integer_from_var_name("value", &vm, &ids_data, &ApTracking::new()),
            Err(HintError::Internal(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 0))
            )))
        );
    }
}
