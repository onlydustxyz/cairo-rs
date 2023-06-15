use super::secp::bigint_utils::Uint384;
use super::uint_utils::{pack, split};
use crate::stdlib::{borrow::Cow, collections::HashMap, prelude::*};
use crate::types::errors::math_errors::MathError;
use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::get_relocatable_from_var_name,
        hint_processor_definition::HintReference,
    },
    serde::deserialize_program::ApTracking,
    types::relocatable::Relocatable,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
};
use felt::Felt252;
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::Zero;

#[derive(Debug, PartialEq)]
pub(crate) struct Uint768<'a> {
    pub d0: Cow<'a, Felt252>,
    pub d1: Cow<'a, Felt252>,
    pub d2: Cow<'a, Felt252>,
    pub d3: Cow<'a, Felt252>,
    pub d4: Cow<'a, Felt252>,
    pub d5: Cow<'a, Felt252>,
}

impl Uint768<'_> {
    pub(crate) fn from_base_addr<'a>(
        addr: Relocatable,
        name: &str,
        vm: &'a VirtualMachine,
    ) -> Result<Uint768<'a>, HintError> {
        Ok(Uint768 {
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
            d5: vm.get_integer((addr + 5)?).map_err(|_| {
                HintError::IdentifierHasNoMember(name.to_string(), "d5".to_string())
            })?,
        })
    }

    pub(crate) fn from_var_name<'a>(
        name: &str,
        vm: &'a VirtualMachine,
        ids_data: &HashMap<String, HintReference>,
        ap_tracking: &ApTracking,
    ) -> Result<Uint768<'a>, HintError> {
        let base_addr = get_relocatable_from_var_name(name, vm, ids_data, ap_tracking)?;
        Uint768::from_base_addr(base_addr, name, vm)
    }

    pub(crate) fn from_values(limbs: [Felt252; 6]) -> Self {
        let [d0, d1, d2, d3, d4, d5] = limbs;
        Self {
            d0: Cow::Owned(d0),
            d1: Cow::Owned(d1),
            d2: Cow::Owned(d2),
            d3: Cow::Owned(d3),
            d4: Cow::Owned(d4),
            d5: Cow::Owned(d5),
        }
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
        vm.insert_value((addr + 3)?, self.d3.into_owned())?;
        vm.insert_value((addr + 4)?, self.d4.into_owned())?;
        vm.insert_value((addr + 5)?, self.d5.into_owned())?;

        Ok(())
    }

    pub(crate) fn pack(self) -> BigUint {
        let limbs = [self.d0, self.d1, self.d2, self.d3, self.d4, self.d5];
        pack(limbs, 128)
    }

    pub(crate) fn split(num: &BigUint) -> Self {
        let limbs = split(num, 128);
        Self::from_values(limbs)
    }
}

/* Implements Hint:
       %{
           def split(num: int, num_bits_shift: int, length: int):
               a = []
               for _ in range(length):
                   a.append( num & ((1 << num_bits_shift) - 1) )
                   num = num >> num_bits_shift
               return tuple(a)

           def pack(z, num_bits_shift: int) -> int:
               limbs = (z.d0, z.d1, z.d2)
               return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

           def pack_extended(z, num_bits_shift: int) -> int:
               limbs = (z.d0, z.d1, z.d2, z.d3, z.d4, z.d5)
               return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

           a = pack_extended(ids.a, num_bits_shift = 128)
           div = pack(ids.div, num_bits_shift = 128)

           quotient, remainder = divmod(a, div)

           quotient_split = split(quotient, num_bits_shift=128, length=6)

           ids.quotient.d0 = quotient_split[0]
           ids.quotient.d1 = quotient_split[1]
           ids.quotient.d2 = quotient_split[2]
           ids.quotient.d3 = quotient_split[3]
           ids.quotient.d4 = quotient_split[4]
           ids.quotient.d5 = quotient_split[5]

           remainder_split = split(remainder, num_bits_shift=128, length=3)
           ids.remainder.d0 = remainder_split[0]
           ids.remainder.d1 = remainder_split[1]
           ids.remainder.d2 = remainder_split[2]
       %}
*/
pub fn unsigned_div_rem_uint768_by_uint384(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let a = Uint768::from_var_name("a", vm, ids_data, ap_tracking)?.pack();
    let div = Uint384::from_var_name("div", vm, ids_data, ap_tracking)?.pack();

    if div.is_zero() {
        return Err(MathError::DividedByZero.into());
    }
    let (quotient, remainder) = a.div_mod_floor(&div);
    let quotient_split = Uint768::split(&quotient);
    quotient_split.insert_from_var_name("quotient", vm, ids_data, ap_tracking)?;
    let remainder_split = Uint384::split(&remainder);
    remainder_split.insert_from_var_name("remainder", vm, ids_data, ap_tracking)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::any_box;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::HintProcessorData;
    use crate::hint_processor::builtin_hint_processor::hint_code;
    use crate::hint_processor::hint_processor_definition::HintProcessor;
    use crate::types::exec_scope::ExecutionScopes;

    use crate::utils::test_utils::*;

    use assert_matches::assert_matches;

    use felt::felt_str;
    use num_traits::One;
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    fn get_uint768_from_base_addr_ok() {
        //Uint768(1,2,3,4,5,6)
        let mut vm = vm!();
        vm.segments = segments![
            ((1, 0), 1),
            ((1, 1), 2),
            ((1, 2), 3),
            ((1, 3), 4),
            ((1, 4), 5),
            ((1, 5), 6)
        ];
        let x = Uint768::from_base_addr((1, 0).into(), "x", &vm).unwrap();
        assert_eq!(x.d0.as_ref(), &Felt252::one());
        assert_eq!(x.d1.as_ref(), &Felt252::from(2));
        assert_eq!(x.d2.as_ref(), &Felt252::from(3));
    }

    #[test]
    fn get_uint768_from_base_addr_missing_member_d0() {
        //Uint768(x,2,x,x,x,x)
        let mut vm = vm!();
        vm.segments = segments![((0, 1), 2)];
        let r = Uint768::from_base_addr((0, 0).into(), "x", &vm);
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d0")
    }

    #[test]
    fn get_uint768_from_base_addr_missing_member_d1() {
        //Uint768(1,x,x,x,x,x)
        let mut vm = vm!();
        vm.segments = segments![((0, 0), 1)];
        let r = Uint768::from_base_addr((0, 0).into(), "x", &vm);
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d1")
    }

    #[test]
    fn get_uint768_from_base_addr_missing_member_d2() {
        //Uint768(1,2,x,x,x,x)
        let mut vm = vm!();
        vm.segments = segments![((0, 0), 1), ((0, 1), 2)];
        let r = Uint768::from_base_addr((0, 0).into(), "x", &vm);
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d2")
    }

    #[test]
    fn get_uint768_from_base_addr_missing_member_d3() {
        //Uint768(1,2,3,x,x,x)
        let mut vm = vm!();
        vm.segments = segments![((0, 0), 1), ((0, 1), 2), ((0, 2), 3)];
        let r = Uint768::from_base_addr((0, 0).into(), "x", &vm);
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d3")
    }

    #[test]
    fn get_uint768_from_base_addr_missing_member_d4() {
        //Uint768(1,2,3,4,x,x)
        let mut vm = vm!();
        vm.segments = segments![((0, 0), 1), ((0, 1), 2), ((0, 2), 3), ((0, 3), 4)];
        let r = Uint768::from_base_addr((0, 0).into(), "x", &vm);
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d4")
    }

    #[test]
    fn get_uint768_from_base_addr_missing_member_d5() {
        //Uint768(1,2,3,4,5,x)
        let mut vm = vm!();
        vm.segments = segments![
            ((0, 0), 1),
            ((0, 1), 2),
            ((0, 2), 3),
            ((0, 3), 4),
            ((0, 4), 5)
        ];
        let r = Uint768::from_base_addr((0, 0).into(), "x", &vm);
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d5")
    }

    #[test]
    fn get_uint768_from_var_name_ok() {
        //Uint768(1,2,3,4,5,6)
        let mut vm = vm!();
        vm.set_fp(1);
        vm.segments = segments![
            ((1, 0), 1),
            ((1, 1), 2),
            ((1, 2), 3),
            ((1, 3), 4),
            ((1, 4), 5),
            ((1, 5), 6)
        ];
        let ids_data = ids_data!["x"];
        let x = Uint768::from_var_name("x", &vm, &ids_data, &ApTracking::default()).unwrap();
        assert_eq!(x.d0.as_ref(), &Felt252::one());
        assert_eq!(x.d1.as_ref(), &Felt252::from(2));
        assert_eq!(x.d2.as_ref(), &Felt252::from(3));
    }

    #[test]
    fn get_uint768_from_var_name_missing_member() {
        //Uint768(1,2,x,x,x)
        let mut vm = vm!();
        vm.set_fp(1);
        vm.segments = segments![((1, 0), 1), ((1, 1), 2)];
        let ids_data = ids_data!["x"];
        let r = Uint768::from_var_name("x", &vm, &ids_data, &ApTracking::default());
        assert_matches!(r, Err(HintError::IdentifierHasNoMember(x, y)) if x == "x" && y == "d2")
    }

    #[test]
    fn get_uint768_from_var_name_invalid_reference() {
        let mut vm = vm!();
        vm.segments = segments![((1, 0), 1), ((1, 1), 2), ((1, 2), 3)];
        let ids_data = ids_data!["x"];
        let r = Uint768::from_var_name("x", &vm, &ids_data, &ApTracking::default());
        assert_matches!(r, Err(HintError::UnknownIdentifier(x)) if x == "x")
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_unsigned_div_rem_ok() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 17;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("a", -17),
            ("div", -11),
            ("quotient", -8),
            ("remainder", -2)
        ];
        //Insert ids into memory
        vm.segments = segments![
            //a
            ((1, 0), 1),
            ((1, 1), 2),
            ((1, 2), 3),
            ((1, 3), 4),
            ((1, 4), 5),
            ((1, 5), 6),
            //div
            ((1, 6), 6),
            ((1, 7), 7),
            ((1, 8), 8)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::UNSIGNED_DIV_REM_UINT768_BY_UINT384),
            Ok(())
        );
        //Check hint memory inserts
        check_memory![
            vm.segments.memory,
            // quotient
            //((1, 9), 328319314958874220607240343889245110272),
            //((1, 10), 329648542954659136480144150949525454847),
            //((1, 11), 255211775190703847597530955573826158591),
            ((1, 12), 0),
            ((1, 13), 0),
            ((1, 14), 0),
            // remainder
            ((1, 15), 71778311772385457136805581255138607105),
            ((1, 16), 147544307532125661892322583691118247938),
            ((1, 17), 3)
        ];
        assert_eq!(
            vm.segments
                .memory
                .get_integer((1, 9).into())
                .unwrap()
                .as_ref(),
            &felt_str!("328319314958874220607240343889245110272")
        );
        assert_eq!(
            vm.segments
                .memory
                .get_integer((1, 10).into())
                .unwrap()
                .as_ref(),
            &felt_str!("329648542954659136480144150949525454847")
        );
        assert_eq!(
            vm.segments
                .memory
                .get_integer((1, 11).into())
                .unwrap()
                .as_ref(),
            &felt_str!("255211775190703847597530955573826158591")
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn run_unsigned_div_rem_divide_by_zero() {
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = 17;
        //Create hint_data
        let ids_data = non_continuous_ids_data![
            ("a", -17),
            ("div", -11),
            ("quotient", -8),
            ("remainder", -2)
        ];
        //Insert ids into memory
        vm.segments = segments![
            //a
            ((1, 0), 1),
            ((1, 1), 2),
            ((1, 2), 3),
            ((1, 3), 4),
            ((1, 4), 5),
            ((1, 5), 6),
            //div
            ((1, 6), 0),
            ((1, 7), 0),
            ((1, 8), 0)
        ];
        //Execute the hint
        assert_matches!(
            run_hint!(vm, ids_data, hint_code::UNSIGNED_DIV_REM_UINT768_BY_UINT384),
            Err(HintError::Math(MathError::DividedByZero))
        );
    }
}
