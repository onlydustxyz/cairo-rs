use crate::stdlib::borrow::Cow;

use crate::{
    serde::deserialize_program::{ApTracking, OffsetValue},
    types::{
        errors::math_errors::MathError,
        instruction::Register,
        relocatable::{MaybeRelocatable, Relocatable},
    },
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};

use super::hint_processor_definition::HintReference;
use felt::Felt252;
use num_traits::ToPrimitive;

///Inserts value into the address of the given ids variable
pub fn insert_value_from_reference(
    value: impl Into<MaybeRelocatable>,
    vm: &mut VirtualMachine,
    hint_reference: &HintReference,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let var_addr = compute_addr_from_reference(hint_reference, vm, ap_tracking)
        .ok_or(HintError::UnknownIdentifierInternal)?;
    vm.insert_value(var_addr, value).map_err(HintError::Memory)
}

///Returns the Integer value stored in the given ids variable
/// Returns an internal error, users should map it into a more informative type
pub fn get_integer_from_reference<'a>(
    vm: &'a VirtualMachine,
    hint_reference: &'a HintReference,
    ap_tracking: &ApTracking,
) -> Result<Cow<'a, Felt252>, HintError> {
    // if the reference register is none, this means it is an immediate value and we
    // should return that value.

    if let (OffsetValue::Immediate(int_1), _) = (&hint_reference.offset1, &hint_reference.offset2) {
        return Ok(Cow::Borrowed(int_1));
    }

    let var_addr = compute_addr_from_reference(hint_reference, vm, ap_tracking)
        .ok_or(HintError::UnknownIdentifierInternal)?;
    vm.get_integer(var_addr)
        .map_err(|_| HintError::WrongIdentifierTypeInternal(var_addr))
}

///Returns the Relocatable value stored in the given ids variable
pub fn get_ptr_from_reference(
    vm: &VirtualMachine,
    hint_reference: &HintReference,
    ap_tracking: &ApTracking,
) -> Result<Relocatable, HintError> {
    let var_addr = compute_addr_from_reference(hint_reference, vm, ap_tracking)
        .ok_or(HintError::UnknownIdentifierInternal)?;
    if hint_reference.dereference {
        vm.get_relocatable(var_addr)
            .map_err(|_| HintError::WrongIdentifierTypeInternal(var_addr))
    } else {
        Ok(var_addr)
    }
}

///Returns the value given by a reference as [MaybeRelocatable]
pub fn get_maybe_relocatable_from_reference(
    vm: &VirtualMachine,
    hint_reference: &HintReference,
    ap_tracking: &ApTracking,
) -> Option<MaybeRelocatable> {
    //First handle case on only immediate
    if let OffsetValue::Immediate(num) = &hint_reference.offset1 {
        return Some(MaybeRelocatable::from(num));
    }
    //Then calculate address
    let var_addr = compute_addr_from_reference(hint_reference, vm, ap_tracking)?;
    if hint_reference.dereference {
        vm.get_maybe(&var_addr)
    } else {
        Some(MaybeRelocatable::from(var_addr))
    }
}

///Computes the memory address of the ids variable indicated by the HintReference as a [Relocatable]
pub fn compute_addr_from_reference(
    //Reference data of the ids variable
    hint_reference: &HintReference,
    vm: &VirtualMachine,
    //ApTracking of the Hint itself
    hint_ap_tracking: &ApTracking,
) -> Option<Relocatable> {
    let offset1 =
        if let OffsetValue::Reference(_register, _offset, _deref) = &hint_reference.offset1 {
            get_offset_value_reference(
                vm,
                hint_reference,
                hint_ap_tracking,
                &hint_reference.offset1,
            )?
            .get_relocatable()?
        } else {
            return None;
        };

    match &hint_reference.offset2 {
        OffsetValue::Reference(_register, _offset, _deref) => {
            // Cant add two relocatable values
            // So OffSet2 must be Bigint
            let value = get_offset_value_reference(
                vm,
                hint_reference,
                hint_ap_tracking,
                &hint_reference.offset2,
            )?;

            Some((offset1 + value.get_int_ref()?.to_usize()?).ok()?)
        }
        OffsetValue::Value(value) => Some((offset1 + *value).ok()?),
        _ => None,
    }
}

fn apply_ap_tracking_correction(
    ap: Relocatable,
    ref_ap_tracking: &ApTracking,
    hint_ap_tracking: &ApTracking,
) -> Option<Relocatable> {
    // check that both groups are the same
    if ref_ap_tracking.group != hint_ap_tracking.group {
        return None;
    }
    let ap_diff = hint_ap_tracking.offset - ref_ap_tracking.offset;
    (ap - ap_diff).ok()
}

//Tries to convert a Felt252 value to usize
pub fn felt_to_usize(felt: &Felt252) -> Result<usize, MathError> {
    felt.to_usize()
        .ok_or_else(|| MathError::Felt252ToUsizeConversion(felt.clone()))
}

///Tries to convert a Felt252 value to u32
pub fn felt_to_u32(felt: &Felt252) -> Result<u32, MathError> {
    felt.to_u32()
        .ok_or_else(|| MathError::Felt252ToU32Conversion(felt.clone()))
}

fn get_offset_value_reference(
    vm: &VirtualMachine,
    hint_reference: &HintReference,
    hint_ap_tracking: &ApTracking,
    offset_value: &OffsetValue,
) -> Option<MaybeRelocatable> {
    let (register, offset, deref) = match offset_value {
        OffsetValue::Reference(register, offset, deref) => (register, offset, deref),
        _ => return None,
    };

    let base_addr = if register == &Register::FP {
        vm.get_fp()
    } else {
        let var_ap_trackig = hint_reference.ap_tracking_data.as_ref()?;

        apply_ap_tracking_correction(vm.get_ap(), var_ap_trackig, hint_ap_tracking)?
    };

    if offset.is_negative() && base_addr.offset < offset.unsigned_abs() as usize {
        return None;
    }

    if *deref {
        vm.get_maybe(&(base_addr + *offset).ok()?)
    } else {
        Some((base_addr + *offset).ok()?.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stdlib::collections::HashMap;

    use crate::{
        relocatable,
        utils::test_utils::*,
        vm::{vm_core::VirtualMachine, vm_memory::memory::Memory},
    };
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_integer_from_reference_with_immediate_value() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), 0)];
        let mut hint_ref = HintReference::new(0, 0, false, true);
        hint_ref.offset1 = OffsetValue::Immediate(Felt252::new(2));

        assert_eq!(
            get_integer_from_reference(&vm, &hint_ref, &ApTracking::new())
                .expect("Unexpected get integer fail")
                .into_owned(),
            Felt252::new(2)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_offset_value_reference_valid() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), 0)];
        let mut hint_ref = HintReference::new(0, 0, false, true);
        hint_ref.offset1 = OffsetValue::Reference(Register::FP, 2_i32, false);

        assert_matches!(
            get_offset_value_reference(&vm, &hint_ref, &ApTracking::new(), &hint_ref.offset1),
            Some(x) if x == mayberelocatable!(1, 2)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_offset_value_reference_invalid() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), 0)];
        let mut hint_ref = HintReference::new(0, 0, false, true);
        hint_ref.offset1 = OffsetValue::Reference(Register::FP, -2_i32, false);

        assert_matches!(
            get_offset_value_reference(&vm, &hint_ref, &ApTracking::new(), &hint_ref.offset1),
            None
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_ptr_from_reference_short_path() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (2, 0))];

        assert_matches!(
            get_ptr_from_reference(
                &vm,
                &HintReference::new(0, 0, false, false),
                &ApTracking::new()
            ),
            Ok(x) if x == relocatable!(1, 0)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_ptr_from_reference_with_dereference() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (3, 0))];

        assert_matches!(
            get_ptr_from_reference(
                &vm,
                &HintReference::new(0, 0, false, true),
                &ApTracking::new()
            ),
            Ok(x) if x == relocatable!(3, 0)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_ptr_from_reference_with_dereference_and_imm() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (4, 0))];
        let mut hint_ref = HintReference::new(0, 0, true, false);
        hint_ref.offset2 = OffsetValue::Value(2);

        assert_matches!(
            get_ptr_from_reference(&vm, &hint_ref, &ApTracking::new()),
            Ok(x) if x == relocatable!(4, 2)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_addr_from_reference_no_regiter_in_reference() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (4, 0))];
        let mut hint_reference = HintReference::new(0, 0, false, false);
        hint_reference.offset1 = OffsetValue::Immediate(Felt252::new(2_i32));

        assert!(compute_addr_from_reference(&hint_reference, &vm, &ApTracking::new()).is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_addr_from_reference_failed_to_get_ids() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), 4)];
        // vm.run_context.fp = -1;
        let mut hint_reference = HintReference::new(0, 0, false, false);
        hint_reference.offset1 = OffsetValue::Reference(Register::FP, -1, true);

        assert_matches!(
            compute_addr_from_reference(&hint_reference, &vm, &ApTracking::new()),
            None
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn tracking_correction_valid() {
        let mut ref_ap_tracking = ApTracking::new();
        ref_ap_tracking.group = 1;
        let mut hint_ap_tracking = ApTracking::new();
        hint_ap_tracking.group = 1;

        assert_matches!(
            apply_ap_tracking_correction(relocatable!(1, 0), &ref_ap_tracking, &hint_ap_tracking),
            Some(relocatable!(1, 0))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn tracking_correction_invalid_group() {
        let mut ref_ap_tracking = ApTracking::new();
        ref_ap_tracking.group = 1;
        let mut hint_ap_tracking = ApTracking::new();
        hint_ap_tracking.group = 2;

        assert!(apply_ap_tracking_correction(
            relocatable!(1, 0),
            &ref_ap_tracking,
            &hint_ap_tracking
        )
        .is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_maybe_relocatable_from_reference_valid() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), (0, 0))];
        let hint_ref = HintReference::new_simple(0);
        assert_matches!(
            get_maybe_relocatable_from_reference(&vm, &hint_ref, &ApTracking::new()),
            Some(x) if x == mayberelocatable!(0, 0)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_maybe_relocatable_from_reference_invalid() {
        let mut vm = vm!();
        vm.segments.memory = Memory::new();
        let hint_ref = HintReference::new_simple(0);
        assert_matches!(
            get_maybe_relocatable_from_reference(&vm, &hint_ref, &ApTracking::new()),
            None
        );
    }
}
