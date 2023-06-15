use crate::{
    hint_processor::hint_processor_definition::HintReference,
    serde::deserialize_program::ApTracking,
    stdlib::{borrow::Cow, collections::HashMap, prelude::*},
    types::relocatable::Relocatable,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use felt::Felt252;
use num_bigint::BigUint;

use super::{
    hint_utils::get_relocatable_from_var_name,
    uint_utils::{pack, split},
};

pub(crate) struct Uint512<'a> {
    pub d0: Cow<'a, Felt252>,
    pub d1: Cow<'a, Felt252>,
    pub d2: Cow<'a, Felt252>,
    pub d3: Cow<'a, Felt252>,
}

impl<'a> Uint512<'a> {
    pub(crate) fn from_base_addr(
        addr: Relocatable,
        name: &str,
        vm: &'a VirtualMachine,
    ) -> Result<Self, HintError> {
        Ok(Self {
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
        })
    }

    pub(crate) fn from_var_name(
        name: &str,
        vm: &'a VirtualMachine,
        ids_data: &HashMap<String, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Result<Self, HintError> {
        let base_addr = get_relocatable_from_var_name(name, vm, ids_data, ap_tracking)?;
        Self::from_base_addr(base_addr, name, vm)
    }

    pub(crate) fn from_values(limbs: [Felt252; 4]) -> Self {
        let [d0, d1, d2, d3] = limbs;
        let d0 = Cow::Owned(d0);
        let d1 = Cow::Owned(d1);
        let d2 = Cow::Owned(d2);
        let d3 = Cow::Owned(d3);
        Self { d0, d1, d2, d3 }
    }

    pub(crate) fn insert_from_var_name(
        self,
        var_name: &str,
        vm: &mut VirtualMachine,
        ids_data: &HashMap<String, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Result<(), HintError> {
        let addr = get_relocatable_from_var_name(var_name, vm, ids_data, ap_tracking)?;

        [self.d0, self.d1, self.d2, self.d3]
            .into_iter()
            .enumerate()
            .try_for_each(|(i, limb)| vm.insert_value((addr + i)?, limb.into_owned()))?;

        Ok(())
    }

    pub(crate) fn pack(self) -> BigUint {
        pack([self.d0, self.d1, self.d2, self.d3], 128)
    }

    pub(crate) fn split(num: &'a BigUint) -> Uint512 {
        let limbs = split(num, 128);
        Self::from_values(limbs)
    }
}

impl<'a> From<&'a BigUint> for Uint512<'a> {
    fn from(value: &'a BigUint) -> Self {
        Self::split(value)
    }
}
