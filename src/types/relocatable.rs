use std::prelude::v1::*;

use crate::{
    relocatable,
    vm::errors::{memory_errors::MemoryError, vm_errors::VirtualMachineError},
};
use felt::Felt;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self, Display},
    ops::Add,
};

#[derive(Eq, Hash, PartialEq, PartialOrd, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Relocatable {
    pub segment_index: isize,
    pub offset: usize,
}

#[derive(Eq, Hash, PartialEq, PartialOrd, Clone, Debug, Serialize, Deserialize)]
pub enum MaybeRelocatable {
    RelocatableValue(Relocatable),
    Int(Felt),
}

impl From<(isize, usize)> for Relocatable {
    fn from(index_offset: (isize, usize)) -> Self {
        Relocatable {
            segment_index: index_offset.0,
            offset: index_offset.1,
        }
    }
}

impl From<(isize, usize)> for MaybeRelocatable {
    fn from(index_offset: (isize, usize)) -> Self {
        MaybeRelocatable::RelocatableValue(Relocatable::from(index_offset))
    }
}

impl From<usize> for MaybeRelocatable {
    fn from(num: usize) -> Self {
        MaybeRelocatable::Int(Felt::new(num))
    }
}

impl From<Felt> for MaybeRelocatable {
    fn from(num: Felt) -> Self {
        MaybeRelocatable::Int(num)
    }
}

impl From<&Relocatable> for MaybeRelocatable {
    fn from(rel: &Relocatable) -> Self {
        MaybeRelocatable::RelocatableValue(*rel)
    }
}

impl From<&Relocatable> for Relocatable {
    fn from(other: &Relocatable) -> Self {
        *other
    }
}

impl From<&Felt> for MaybeRelocatable {
    fn from(val: &Felt) -> Self {
        MaybeRelocatable::Int(val.clone())
    }
}

impl From<Relocatable> for MaybeRelocatable {
    fn from(rel: Relocatable) -> Self {
        MaybeRelocatable::RelocatableValue(rel)
    }
}

impl Display for MaybeRelocatable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MaybeRelocatable::RelocatableValue(rel) => rel.fmt(f),
            MaybeRelocatable::Int(num) => write!(f, "{}", num),
        }
    }
}

impl Display for Relocatable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.segment_index, self.offset)
    }
}

impl Add<usize> for Relocatable {
    type Output = Relocatable;
    fn add(self, other: usize) -> Self {
        relocatable!(self.segment_index, self.offset + other)
    }
}

impl Add<i32> for Relocatable {
    type Output = Relocatable;
    fn add(self, other: i32) -> Self {
        if other >= 0 {
            relocatable!(self.segment_index, self.offset + other as usize)
        } else {
            relocatable!(
                self.segment_index,
                self.offset - other.unsigned_abs() as usize
            )
        }
    }
}

impl Add<i32> for &Relocatable {
    type Output = Relocatable;
    fn add(self, other: i32) -> Relocatable {
        if other >= 0 {
            relocatable!(self.segment_index, self.offset + other as usize)
        } else {
            relocatable!(
                self.segment_index,
                self.offset - other.unsigned_abs() as usize
            )
        }
    }
}

impl TryInto<Relocatable> for MaybeRelocatable {
    type Error = MemoryError;
    fn try_into(self) -> Result<Relocatable, MemoryError> {
        match self {
            MaybeRelocatable::RelocatableValue(rel) => Ok(rel),
            _ => Err(MemoryError::AddressNotRelocatable),
        }
    }
}

impl From<&MaybeRelocatable> for MaybeRelocatable {
    fn from(other: &MaybeRelocatable) -> Self {
        other.clone()
    }
}

impl TryFrom<&MaybeRelocatable> for Relocatable {
    type Error = MemoryError;
    fn try_from(other: &MaybeRelocatable) -> Result<Self, MemoryError> {
        match other {
            MaybeRelocatable::RelocatableValue(rel) => Ok(*rel),
            _ => Err(MemoryError::AddressNotRelocatable),
        }
    }
}

impl Relocatable {
    pub fn sub_usize(&self, other: usize) -> Result<Self, VirtualMachineError> {
        if self.offset < other {
            return Err(VirtualMachineError::CantSubOffset(self.offset, other));
        }
        let new_offset = self.offset - other;
        Ok(relocatable!(self.segment_index, new_offset))
    }

    ///Adds a Felt to self
    pub fn add_int(&self, other: &Felt) -> Result<Relocatable, VirtualMachineError> {
        let big_offset = other + self.offset;
        let new_offset = big_offset
            .to_usize()
            .ok_or(VirtualMachineError::OffsetExceeded(big_offset))?;
        Ok(Relocatable {
            segment_index: self.segment_index,
            offset: new_offset,
        })
    }

    /// Adds a MaybeRelocatable to self
    /// Cant add two relocatable values
    pub fn add_maybe(&self, other: &MaybeRelocatable) -> Result<Relocatable, VirtualMachineError> {
        let num_ref = other
            .get_int_ref()
            .map_err(|_| VirtualMachineError::RelocatableAdd)?;

        let big_offset: Felt = num_ref + self.offset;
        let new_offset = big_offset
            .to_usize()
            .ok_or(VirtualMachineError::OffsetExceeded(big_offset))?;
        Ok(Relocatable {
            segment_index: self.segment_index,
            offset: new_offset,
        })
    }

    pub fn sub(&self, other: &Self) -> Result<usize, VirtualMachineError> {
        if self.segment_index != other.segment_index {
            return Err(VirtualMachineError::DiffIndexSub);
        }
        if self.offset < other.offset {
            return Err(VirtualMachineError::CantSubOffset(
                self.offset,
                other.offset,
            ));
        }
        let result = self.offset - other.offset;
        Ok(result)
    }
}

impl MaybeRelocatable {
    /// Adds a Felt to self
    pub fn add_int(&self, other: &Felt) -> Result<MaybeRelocatable, VirtualMachineError> {
        match *self {
            MaybeRelocatable::Int(ref value) => Ok(MaybeRelocatable::Int(value + other)),
            MaybeRelocatable::RelocatableValue(ref rel) => {
                let big_offset = other + rel.offset;
                let new_offset = big_offset
                    .to_usize()
                    .ok_or(VirtualMachineError::OffsetExceeded(big_offset))?;
                Ok(MaybeRelocatable::RelocatableValue(Relocatable {
                    segment_index: rel.segment_index,
                    offset: new_offset,
                }))
            }
        }
    }

    /// Adds a usize to self
    pub fn add_usize(&self, other: usize) -> MaybeRelocatable {
        match *self {
            MaybeRelocatable::Int(ref value) => MaybeRelocatable::Int(value + other),
            MaybeRelocatable::RelocatableValue(ref rel) => {
                MaybeRelocatable::RelocatableValue(Relocatable {
                    segment_index: rel.segment_index,
                    offset: rel.offset + other,
                })
            }
        }
    }

    /// Adds a MaybeRelocatable to self
    /// Cant add two relocatable values
    pub fn add(&self, other: &MaybeRelocatable) -> Result<MaybeRelocatable, VirtualMachineError> {
        match (self, other) {
            (&MaybeRelocatable::Int(ref num_a_ref), MaybeRelocatable::Int(num_b)) => {
                Ok(MaybeRelocatable::Int(num_a_ref + num_b))
            }
            (&MaybeRelocatable::RelocatableValue(_), &MaybeRelocatable::RelocatableValue(_)) => {
                Err(VirtualMachineError::RelocatableAdd)
            }
            (&MaybeRelocatable::RelocatableValue(ref rel), &MaybeRelocatable::Int(ref num_ref))
            | (&MaybeRelocatable::Int(ref num_ref), &MaybeRelocatable::RelocatableValue(ref rel)) =>
            {
                let big_offset: Felt = num_ref + rel.offset;
                let new_offset = big_offset
                    .to_usize()
                    .ok_or(VirtualMachineError::OffsetExceeded(big_offset))?;
                Ok(MaybeRelocatable::RelocatableValue(Relocatable {
                    segment_index: rel.segment_index,
                    offset: new_offset,
                }))
            }
        }
    }

    /// Substracts two MaybeRelocatable values and returns the result as a MaybeRelocatable value.
    /// Only values of the same type may be substracted.
    /// Relocatable values can only be substracted if they belong to the same segment.
    pub fn sub(&self, other: &MaybeRelocatable) -> Result<MaybeRelocatable, VirtualMachineError> {
        match (self, other) {
            (&MaybeRelocatable::Int(ref num_a), &MaybeRelocatable::Int(ref num_b)) => {
                Ok(MaybeRelocatable::Int(num_a - num_b))
            }
            (
                MaybeRelocatable::RelocatableValue(rel_a),
                MaybeRelocatable::RelocatableValue(rel_b),
            ) => {
                if rel_a.segment_index == rel_b.segment_index {
                    return Ok(MaybeRelocatable::from(Felt::new(
                        rel_a.offset - rel_b.offset,
                    )));
                }
                Err(VirtualMachineError::DiffIndexSub)
            }
            (MaybeRelocatable::RelocatableValue(rel_a), MaybeRelocatable::Int(ref num_b)) => {
                Ok(MaybeRelocatable::from((
                    rel_a.segment_index,
                    (rel_a.offset - num_b)
                        .to_usize()
                        .ok_or_else(|| VirtualMachineError::OffsetExceeded(rel_a.offset - num_b))?,
                )))
            }
            _ => Err(VirtualMachineError::NotImplemented),
        }
    }

    /// Performs integer division and module on a MaybeRelocatable::Int by another
    /// MaybeRelocatable::Int and returns the quotient and reminder.
    pub fn divmod(
        &self,
        other: &MaybeRelocatable,
    ) -> Result<(MaybeRelocatable, MaybeRelocatable), VirtualMachineError> {
        match (self, other) {
            (&MaybeRelocatable::Int(ref val), &MaybeRelocatable::Int(ref div)) => Ok((
                MaybeRelocatable::from(val / div),
                // NOTE: elements on a field element always have multiplicative inverse
                MaybeRelocatable::from(Felt::zero()),
            )),
            _ => Err(VirtualMachineError::NotImplemented),
        }
    }

    //Returns reference to Felt inside self if Int variant or Error if RelocatableValue variant
    pub fn get_int_ref(&self) -> Result<&Felt, VirtualMachineError> {
        match self {
            MaybeRelocatable::Int(num) => Ok(num),
            MaybeRelocatable::RelocatableValue(_) => {
                Err(VirtualMachineError::ExpectedInteger(self.clone()))
            }
        }
    }

    //Returns reference to Relocatable inside self if Relocatable variant or Error if Int variant
    pub fn get_relocatable(&self) -> Result<Relocatable, VirtualMachineError> {
        match self {
            MaybeRelocatable::RelocatableValue(rel) => Ok(*rel),
            MaybeRelocatable::Int(_) => Err(VirtualMachineError::ExpectedRelocatable(self.clone())),
        }
    }
}

impl<'a> Add<usize> for &'a Relocatable {
    type Output = Relocatable;

    fn add(self, other: usize) -> Self::Output {
        Relocatable {
            segment_index: self.segment_index,
            offset: self.offset + other,
        }
    }
}

/// Turns a MaybeRelocatable into a Felt value.
/// If the value is an Int, it will extract the Felt value from it.
/// If the value is Relocatable, it will return an error since it should've already been relocated.
pub fn relocate_value(
    value: MaybeRelocatable,
    relocation_table: &Vec<usize>,
) -> Result<Felt, MemoryError> {
    match value {
        MaybeRelocatable::Int(num) => Ok(num),
        MaybeRelocatable::RelocatableValue(relocatable) => {
            Felt::from_usize(relocate_address(relocatable, relocation_table)?)
                .ok_or(MemoryError::Relocation)
        }
    }
}

pub fn relocate_address(
    relocatable: Relocatable,
    relocation_table: &Vec<usize>,
) -> Result<usize, MemoryError> {
    let (segment_index, offset) = if relocatable.segment_index >= 0 {
        (relocatable.segment_index as usize, relocatable.offset)
    } else {
        return Err(MemoryError::TemporarySegmentInRelocation(
            relocatable.segment_index,
        ));
    };

    if relocation_table.len() <= segment_index {
        return Err(MemoryError::Relocation);
    }

    Ok(relocation_table[segment_index] + offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{relocatable, utils::test_utils::mayberelocatable};
    use felt::felt_str;
    use num_traits::{One, Zero};

    #[test]
    fn add_bigint_to_int() {
        let addr = MaybeRelocatable::from(Felt::new(7i32));
        let added_addr = addr.add_int(&Felt::new(2i32));
        assert_eq!(Ok(MaybeRelocatable::Int(Felt::new(9i32))), added_addr);
    }

    #[test]
    fn add_usize_to_int() {
        let addr = MaybeRelocatable::from(Felt::new(7_i32));
        let added_addr = addr.add_usize(2);
        assert_eq!(MaybeRelocatable::Int(Felt::new(9_i32)), added_addr);
    }

    #[test]
    fn add_bigint_to_relocatable() {
        let addr = MaybeRelocatable::RelocatableValue(relocatable!(7, 65));
        let added_addr = addr.add_int(&Felt::new(2));
        assert_eq!(Ok(MaybeRelocatable::from((7, 67))), added_addr);
    }

    #[test]
    fn add_int_mod_offset_exceeded() {
        let addr = MaybeRelocatable::from((0, 0));
        let error = addr.add_int(&felt_str!("18446744073709551616"));
        assert_eq!(
            error,
            Err(VirtualMachineError::OffsetExceeded(felt_str!(
                "18446744073709551616"
            )))
        );
        assert_eq!(
            error.unwrap_err().to_string(),
            "Offset 18446744073709551616 exceeds maximum offset value"
        );
    }

    #[test]
    fn add_usize_to_relocatable() {
        let addr = MaybeRelocatable::RelocatableValue(relocatable!(7, 65));
        let added_addr = addr.add_int(&Felt::new(2));
        assert_eq!(Ok(MaybeRelocatable::from((7, 67))), added_addr);
    }

    #[test]
    fn add_bigint_to_int_prime_mod() {
        let addr = MaybeRelocatable::Int(felt_str!(
            "800000000000011000000000000000000000000000000000000000000000004",
            16
        ));
        let added_addr = addr.add_int(&Felt::one());
        assert_eq!(Ok(MaybeRelocatable::Int(Felt::new(4_i32))), added_addr);
    }

    #[test]
    fn add_bigint_to_relocatable_prime() {
        let addr = MaybeRelocatable::RelocatableValue(relocatable!(1, 9));
        let added_addr = addr.add_int(&felt_str!(
            "3618502788666131213697322783095070105623107215331596699973092056135872020481"
        ));
        assert_eq!(
            Ok(MaybeRelocatable::RelocatableValue(relocatable!(1, 9))),
            added_addr
        );
    }

    #[test]
    fn add_int_to_int() {
        let addr_a = &MaybeRelocatable::from(felt_str!(
            "3618502788666131213697322783095070105623107215331596699973092056135872020488"
        ));
        let addr_b = &MaybeRelocatable::from(Felt::new(17_i32));
        let added_addr = addr_a.add(addr_b);
        assert_eq!(Ok(MaybeRelocatable::from(Felt::new(24_i32))), added_addr);
    }

    #[test]
    fn add_relocatable_to_relocatable_should_fail() {
        let addr_a = &MaybeRelocatable::from((7, 5));
        let addr_b = &MaybeRelocatable::RelocatableValue(relocatable!(7, 10));
        let error = addr_a.add(addr_b);
        assert_eq!(error, Err(VirtualMachineError::RelocatableAdd));
    }

    #[test]
    fn add_int_to_relocatable() {
        let addr_a = &MaybeRelocatable::from((7, 7));
        let addr_b = &MaybeRelocatable::from(Felt::new(10));
        let added_addr = addr_a.add(addr_b);
        assert_eq!(
            Ok(MaybeRelocatable::RelocatableValue(relocatable!(7, 17))),
            added_addr
        );
    }

    #[test]
    fn add_relocatable_to_int() {
        let addr_a = &MaybeRelocatable::from(Felt::new(10_i32));
        let addr_b = &MaybeRelocatable::RelocatableValue(relocatable!(7, 7));
        let added_addr = addr_a.add(addr_b);
        assert_eq!(
            Ok(MaybeRelocatable::RelocatableValue(relocatable!(7, 17))),
            added_addr
        );
    }

    #[test]
    fn add_int_to_relocatable_prime() {
        let addr_a = &MaybeRelocatable::from((7, 14));
        let addr_b = &MaybeRelocatable::Int(felt_str!(
            "800000000000011000000000000000000000000000000000000000000000001",
            16
        ));
        let added_addr = addr_a.add(addr_b);
        assert_eq!(
            Ok(MaybeRelocatable::RelocatableValue(relocatable!(7, 14))),
            added_addr
        );
    }

    #[test]
    fn add_int_rel_int_offset_exceeded() {
        let addr = MaybeRelocatable::from((0, 0));
        let error = addr.add(&MaybeRelocatable::from(felt_str!("18446744073709551616")));
        assert_eq!(
            error,
            Err(VirtualMachineError::OffsetExceeded(felt_str!(
                "18446744073709551616"
            )))
        );
    }

    #[test]
    fn add_int_int_rel_offset_exceeded() {
        let addr = MaybeRelocatable::Int(felt_str!("18446744073709551616"));
        let relocatable = Relocatable {
            offset: 0,
            segment_index: 0,
        };
        let error = addr.add(&MaybeRelocatable::RelocatableValue(relocatable));
        assert_eq!(
            error,
            Err(VirtualMachineError::OffsetExceeded(felt_str!(
                "18446744073709551616"
            )))
        );
    }

    #[test]
    fn sub_int_from_int() {
        let addr_a = &MaybeRelocatable::from(Felt::new(7));
        let addr_b = &MaybeRelocatable::from(Felt::new(5));
        let sub_addr = addr_a.sub(addr_b);
        assert_eq!(Ok(MaybeRelocatable::from(Felt::new(2))), sub_addr);
    }

    #[test]
    fn sub_relocatable_from_relocatable_same_offset() {
        let addr_a = &MaybeRelocatable::from((7, 17));
        let addr_b = &MaybeRelocatable::from((7, 7));
        let sub_addr = addr_a.sub(addr_b);
        assert_eq!(Ok(MaybeRelocatable::from(Felt::new(10))), sub_addr);
    }

    #[test]
    fn sub_relocatable_from_relocatable_diff_offset() {
        let addr_a = &MaybeRelocatable::from((7, 17));
        let addr_b = &MaybeRelocatable::from((8, 7));
        let error = addr_a.sub(addr_b);
        assert_eq!(error, Err(VirtualMachineError::DiffIndexSub));
        assert_eq!(
            error.unwrap_err().to_string(),
            "Can only subtract two relocatable values of the same segment"
        );
    }

    #[test]
    fn sub_int_addr_ref_from_relocatable_addr_ref() {
        let addr_a = &MaybeRelocatable::from((7, 17));
        let addr_b = &MaybeRelocatable::from(Felt::new(5_i32));
        let addr_c = addr_a.sub(addr_b);
        assert_eq!(addr_c, Ok(MaybeRelocatable::from((7, 12))));
    }

    #[test]
    fn sub_rel_to_int_error() {
        let a = &MaybeRelocatable::from(Felt::new(7_i32));
        let b = &MaybeRelocatable::from((7, 10));
        assert_eq!(Err(VirtualMachineError::NotImplemented), a.sub(b));
    }

    #[test]
    fn divmod_working() {
        let value = &MaybeRelocatable::from(Felt::new(10));
        let div = &MaybeRelocatable::from(Felt::new(3));
        let (q, r) = value.divmod(div).expect("Unexpected error in divmod");
        assert_eq!(q, MaybeRelocatable::from(Felt::new(10) / Felt::new(3)));
        assert_eq!(r, MaybeRelocatable::from(Felt::zero()));
    }

    #[test]
    fn divmod_bad_type() {
        let value = &MaybeRelocatable::from(Felt::new(10));
        let div = &MaybeRelocatable::from((2, 7));
        assert_eq!(value.divmod(div), Err(VirtualMachineError::NotImplemented));
    }

    #[test]
    fn relocate_relocatable_value() {
        let value = MaybeRelocatable::from((2, 7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(relocate_value(value, &relocation_table), Ok(Felt::new(12)));
    }

    #[test]
    fn relocate_relocatable_in_temp_segment_value() {
        let value = MaybeRelocatable::from((-1, 7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Err(MemoryError::TemporarySegmentInRelocation(-1)),
        );
    }

    #[test]
    fn relocate_relocatable_in_temp_segment_value_with_offset() {
        let value = MaybeRelocatable::from((-1, 7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Err(MemoryError::TemporarySegmentInRelocation(-1)),
        );
    }

    #[test]
    fn relocate_relocatable_in_temp_segment_value_error() {
        let value = MaybeRelocatable::from((-1, 7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Err(MemoryError::TemporarySegmentInRelocation(-1))
        );
    }

    #[test]
    fn relocate_int_value() {
        let value = MaybeRelocatable::from(Felt::new(7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(relocate_value(value, &relocation_table), Ok(Felt::new(7)));
    }

    #[test]
    fn relocate_relocatable_value_no_relocation() {
        let value = MaybeRelocatable::from((2, 7));
        let relocation_table = vec![1, 2];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Err(MemoryError::Relocation)
        );
    }

    #[test]
    fn relocatable_add_int() {
        assert_eq!(
            Ok(relocatable!(1, 6)),
            relocatable!(1, 2).add_int(&Felt::new(4))
        );
        assert_eq!(
            Ok(relocatable!(3, 2)),
            relocatable!(3, 2).add_int(&Felt::zero())
        );
    }

    #[test]
    fn relocatable_add_int_mod_offset_exceeded_error() {
        assert_eq!(
            Err(VirtualMachineError::OffsetExceeded(
                Felt::new(usize::MAX) + 1_usize
            )),
            relocatable!(0, 0).add_int(&(Felt::new(usize::MAX) + 1_usize))
        );
    }

    #[test]
    fn relocatable_add_i32() {
        let reloc = relocatable!(1, 5);

        assert_eq!(reloc + 3, relocatable!(1, 8));
        assert_eq!(reloc + (-3), relocatable!(1, 2));
    }

    #[test]
    #[should_panic]
    fn relocatable_add_i32_with_overflow() {
        let reloc = relocatable!(1, 1);

        let _panic = reloc + (-3);
    }

    #[test]
    fn mayberelocatable_try_into_reloctable() {
        let address = mayberelocatable!(1, 2);
        assert_eq!(Ok(relocatable!(1, 2)), address.try_into());

        let value = mayberelocatable!(1);
        let err: Result<Relocatable, _> = value.try_into();
        assert_eq!(Err(MemoryError::AddressNotRelocatable), err)
    }

    #[test]
    fn relocatable_sub_rel_test() {
        let reloc = relocatable!(7, 6);

        assert_eq!(Ok(1), reloc.sub(&relocatable!(7, 5)));
        assert_eq!(
            Err(VirtualMachineError::CantSubOffset(6, 9)),
            reloc.sub(&relocatable!(7, 9))
        );
    }

    #[test]
    fn sub_rel_different_indexes() {
        let a = relocatable!(7, 6);
        let b = relocatable!(8, 6);

        assert_eq!(Err(VirtualMachineError::DiffIndexSub), a.sub(&b));
    }

    #[test]
    fn add_maybe_mod_ok() {
        assert_eq!(
            Ok(relocatable!(1, 2)),
            relocatable!(1, 0).add_maybe(&mayberelocatable!(2))
        );
        assert_eq!(
            Ok(relocatable!(0, 129)),
            relocatable!(0, 29).add_maybe(&mayberelocatable!(100))
        );
        assert_eq!(
            Ok(relocatable!(2, 116)),
            relocatable!(2, 12).add_maybe(&mayberelocatable!(104))
        );

        assert_eq!(
            Ok(relocatable!(1, 0)),
            relocatable!(1, 0).add_maybe(&mayberelocatable!(0))
        );
        assert_eq!(
            Ok(relocatable!(1, 73)),
            relocatable!(1, 2).add_maybe(&mayberelocatable!(71))
        );
    }

    #[test]
    fn add_maybe_mod_add_two_relocatable_error() {
        assert_eq!(
            Err(VirtualMachineError::RelocatableAdd),
            relocatable!(1, 0).add_maybe(&mayberelocatable!(1, 2))
        );
    }

    #[test]
    fn add_maybe_mod_offset_exceeded_error() {
        assert_eq!(
            Err(VirtualMachineError::OffsetExceeded(
                Felt::new(usize::MAX) + 1_usize
            )),
            relocatable!(1, 0).add_maybe(&mayberelocatable!(usize::MAX as i128 + 1),)
        );
    }

    #[test]
    fn get_relocatable_test() {
        assert_eq!(
            Ok(relocatable!(1, 2)),
            mayberelocatable!(1, 2).get_relocatable()
        );
        assert_eq!(
            Err(VirtualMachineError::ExpectedRelocatable(mayberelocatable!(
                3
            ))),
            mayberelocatable!(3).get_relocatable()
        )
    }

    #[test]
    fn relocatable_display() {
        assert_eq!(
            format!("{}", Relocatable::from((1, 0))),
            String::from("1:0")
        )
    }

    #[test]
    fn maybe_relocatable_relocatable_display() {
        assert_eq!(
            format!("{}", MaybeRelocatable::from((1, 0))),
            String::from("1:0")
        )
    }

    #[test]
    fn maybe_relocatable_int_display() {
        assert_eq!(
            format!("{}", MaybeRelocatable::from(Felt::new(6))),
            String::from("6")
        )
    }
}
