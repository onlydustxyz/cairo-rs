use crate::stdlib::{
    fmt::{self, Display},
    ops::{Add, AddAssign, Sub},
    prelude::*,
};

use crate::{
    relocatable, types::errors::math_errors::MathError, vm::errors::memory_errors::MemoryError,
};
use felt::Felt252;
use num_traits::{ToPrimitive, Zero};
use serde::{Deserialize, Serialize};

#[derive(Eq, Ord, Hash, PartialEq, PartialOrd, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Relocatable {
    pub segment_index: isize,
    pub offset: usize,
}

#[derive(Eq, Ord, Hash, PartialEq, PartialOrd, Clone, Debug, Serialize, Deserialize)]
pub enum MaybeRelocatable {
    RelocatableValue(Relocatable),
    Int(Felt252),
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
        MaybeRelocatable::Int(Felt252::new(num))
    }
}

impl From<Felt252> for MaybeRelocatable {
    fn from(num: Felt252) -> Self {
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

impl From<&Felt252> for MaybeRelocatable {
    fn from(val: &Felt252) -> Self {
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
            MaybeRelocatable::Int(num) => write!(f, "{num}"),
        }
    }
}

impl Display for Relocatable {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.segment_index, self.offset)
    }
}

impl Add<usize> for Relocatable {
    type Output = Result<Relocatable, MathError>;
    fn add(self, other: usize) -> Result<Self, MathError> {
        self.offset
            .checked_add(other)
            .map(|x| Relocatable::from((self.segment_index, x)))
            .ok_or(MathError::RelocatableAddUsizeOffsetExceeded(self, other))
    }
}

/// Warning: may panic if self.offset + rhs exceeds usize::MAX
impl AddAssign<usize> for Relocatable {
    fn add_assign(&mut self, rhs: usize) {
        self.offset += rhs
    }
}

impl Add<i32> for Relocatable {
    type Output = Result<Relocatable, MathError>;
    fn add(self, other: i32) -> Result<Self, MathError> {
        if other >= 0 {
            self + other as usize
        } else {
            self - other.unsigned_abs() as usize
        }
    }
}
impl Add<&Felt252> for Relocatable {
    type Output = Result<Relocatable, MathError>;
    fn add(self, other: &Felt252) -> Result<Relocatable, MathError> {
        let big_offset = other + self.offset;
        let new_offset = big_offset
            .to_usize()
            .ok_or_else(|| MathError::RelocatableAddFelt252OffsetExceeded(self, other.clone()))?;
        Ok(Relocatable {
            segment_index: self.segment_index,
            offset: new_offset,
        })
    }
}

/// Adds a MaybeRelocatable to self
/// Cant add two relocatable values
impl Add<&MaybeRelocatable> for Relocatable {
    type Output = Result<Relocatable, MathError>;
    fn add(self, other: &MaybeRelocatable) -> Result<Relocatable, MathError> {
        let num_ref = match other {
            MaybeRelocatable::RelocatableValue(rel) => {
                return Err(MathError::RelocatableAdd(self, *rel))
            }
            MaybeRelocatable::Int(num) => num,
        };
        self + num_ref
    }
}

impl Sub<usize> for Relocatable {
    type Output = Result<Relocatable, MathError>;
    fn sub(self, other: usize) -> Result<Self, MathError> {
        if self.offset < other {
            return Err(MathError::RelocatableSubNegOffset(self, other));
        }
        let new_offset = self.offset - other;
        Ok(relocatable!(self.segment_index, new_offset))
    }
}
impl Sub<Relocatable> for Relocatable {
    type Output = Result<usize, MathError>;
    fn sub(self, other: Self) -> Result<usize, MathError> {
        if self.segment_index != other.segment_index {
            return Err(MathError::RelocatableSubDiffIndex(self, other));
        }
        if self.offset < other.offset {
            return Err(MathError::RelocatableSubNegOffset(self, other.offset));
        }
        let result = self.offset - other.offset;
        Ok(result)
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
    type Error = MathError;
    fn try_from(other: &MaybeRelocatable) -> Result<Self, MathError> {
        match other {
            MaybeRelocatable::RelocatableValue(rel) => Ok(*rel),
            MaybeRelocatable::Int(num) => Err(MathError::Felt252ToRelocatable(num.clone())),
        }
    }
}

impl MaybeRelocatable {
    /// Adds a Felt252 to self
    pub fn add_int(&self, other: &Felt252) -> Result<MaybeRelocatable, MathError> {
        match *self {
            MaybeRelocatable::Int(ref value) => Ok(MaybeRelocatable::Int(value + other)),
            MaybeRelocatable::RelocatableValue(ref rel) => {
                let big_offset = other + rel.offset;
                let new_offset = big_offset.to_usize().ok_or_else(|| {
                    MathError::RelocatableAddFelt252OffsetExceeded(*rel, other.clone())
                })?;
                Ok(MaybeRelocatable::RelocatableValue(Relocatable {
                    segment_index: rel.segment_index,
                    offset: new_offset,
                }))
            }
        }
    }

    /// Adds a usize to self
    pub fn add_usize(&self, other: usize) -> Result<MaybeRelocatable, MathError> {
        Ok(match *self {
            MaybeRelocatable::Int(ref value) => MaybeRelocatable::Int(value + other),
            MaybeRelocatable::RelocatableValue(rel) => (rel + other)?.into(),
        })
    }

    /// Adds a MaybeRelocatable to self
    /// Cant add two relocatable values
    pub fn add(&self, other: &MaybeRelocatable) -> Result<MaybeRelocatable, MathError> {
        match (self, other) {
            (MaybeRelocatable::Int(num_a_ref), MaybeRelocatable::Int(num_b)) => {
                Ok(MaybeRelocatable::Int(num_a_ref + num_b))
            }
            (
                &MaybeRelocatable::RelocatableValue(rel_a),
                &MaybeRelocatable::RelocatableValue(rel_b),
            ) => Err(MathError::RelocatableAdd(rel_a, rel_b)),
            (&MaybeRelocatable::RelocatableValue(rel), &MaybeRelocatable::Int(ref num_ref))
            | (&MaybeRelocatable::Int(ref num_ref), &MaybeRelocatable::RelocatableValue(rel)) => {
                Ok((rel + num_ref)?.into())
            }
        }
    }

    /// Substracts two MaybeRelocatable values and returns the result as a MaybeRelocatable value.
    /// Only values of the same type may be substracted.
    /// Relocatable values can only be substracted if they belong to the same segment.
    pub fn sub(&self, other: &MaybeRelocatable) -> Result<MaybeRelocatable, MathError> {
        match (self, other) {
            (MaybeRelocatable::Int(num_a), MaybeRelocatable::Int(num_b)) => {
                Ok(MaybeRelocatable::Int(num_a - num_b))
            }
            (
                MaybeRelocatable::RelocatableValue(rel_a),
                MaybeRelocatable::RelocatableValue(rel_b),
            ) => {
                if rel_a.segment_index == rel_b.segment_index {
                    return Ok(MaybeRelocatable::from(Felt252::new((*rel_a - *rel_b)?)));
                }
                Err(MathError::RelocatableSubDiffIndex(*rel_a, *rel_b))
            }
            (MaybeRelocatable::RelocatableValue(rel_a), MaybeRelocatable::Int(ref num_b)) => {
                Ok(MaybeRelocatable::from((
                    rel_a.segment_index,
                    (rel_a.offset - num_b).to_usize().ok_or_else(|| {
                        MathError::RelocatableAddFelt252OffsetExceeded(*rel_a, num_b.clone())
                    })?,
                )))
            }
            (MaybeRelocatable::Int(int), MaybeRelocatable::RelocatableValue(rel)) => {
                Err(MathError::SubRelocatableFromInt(int.clone(), *rel))
            }
        }
    }

    /// Performs integer division and module on a MaybeRelocatable::Int by another
    /// MaybeRelocatable::Int and returns the quotient and reminder.
    pub fn divmod(
        &self,
        other: &MaybeRelocatable,
    ) -> Result<(MaybeRelocatable, MaybeRelocatable), MathError> {
        match (self, other) {
            (MaybeRelocatable::Int(val), MaybeRelocatable::Int(div)) => Ok((
                MaybeRelocatable::from(val / div),
                // NOTE: elements on a field element always have multiplicative inverse
                MaybeRelocatable::from(Felt252::zero()),
            )),
            _ => Err(MathError::DivModWrongType(self.clone(), other.clone())),
        }
    }

    /// Returns a reference to the inner value if it is a Felt252, returns None otherwise.
    pub fn get_int_ref(&self) -> Option<&Felt252> {
        match self {
            MaybeRelocatable::Int(num) => Some(num),
            MaybeRelocatable::RelocatableValue(_) => None,
        }
    }

    /// Returns the inner value if it is a Relocatable, returns None otherwise.
    pub fn get_relocatable(&self) -> Option<Relocatable> {
        match self {
            MaybeRelocatable::RelocatableValue(rel) => Some(*rel),
            MaybeRelocatable::Int(_) => None,
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

/// Turns a MaybeRelocatable into a Felt252 value.
/// If the value is an Int, it will extract the Felt252 value from it.
/// If the value is RelocatableValue, it will relocate it according to the relocation_table
pub fn relocate_value(
    value: MaybeRelocatable,
    relocation_table: &Vec<usize>,
) -> Result<Felt252, MemoryError> {
    match value {
        MaybeRelocatable::Int(num) => Ok(num),
        MaybeRelocatable::RelocatableValue(relocatable) => Ok(Felt252::from(relocate_address(
            relocatable,
            relocation_table,
        )?)),
    }
}

// Relocates a Relocatable value according to the relocation_table
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

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_bigint_to_int() {
        let addr = MaybeRelocatable::from(Felt252::new(7i32));
        let added_addr = addr.add_int(&Felt252::new(2i32));
        assert_eq!(added_addr, Ok(MaybeRelocatable::Int(Felt252::new(9))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_usize_to_int() {
        let addr = MaybeRelocatable::from(Felt252::new(7_i32));
        let added_addr = addr.add_usize(2).unwrap();
        assert_eq!(MaybeRelocatable::Int(Felt252::new(9)), added_addr);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_bigint_to_relocatable() {
        let addr = MaybeRelocatable::RelocatableValue(relocatable!(7, 65));
        let added_addr = addr.add_int(&Felt252::new(2));
        assert_eq!(
            added_addr,
            Ok(MaybeRelocatable::RelocatableValue(Relocatable {
                segment_index: 7,
                offset: 67
            }))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_int_mod_offset_exceeded() {
        let addr = MaybeRelocatable::from((0, 0));
        let error = addr.add_int(&felt_str!("18446744073709551616"));
        assert_eq!(
            error,
            Err(MathError::RelocatableAddFelt252OffsetExceeded(
                relocatable!(0, 0),
                felt_str!("18446744073709551616")
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_usize_to_relocatable() {
        let addr = MaybeRelocatable::RelocatableValue(relocatable!(7, 65));
        let added_addr = addr.add_usize(2);
        assert_eq!(
            added_addr,
            Ok(MaybeRelocatable::RelocatableValue(Relocatable {
                segment_index: 7,
                offset: 67
            }))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_bigint_to_int_prime_mod() {
        let addr = MaybeRelocatable::Int(felt_str!(
            "800000000000011000000000000000000000000000000000000000000000004",
            16
        ));
        let added_addr = addr.add_int(&Felt252::one());
        assert_eq!(added_addr, Ok(MaybeRelocatable::Int(Felt252::new(4))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_bigint_to_relocatable_prime() {
        let addr = MaybeRelocatable::from((1, 9));
        let added_addr = addr.add_int(&felt_str!(
            "3618502788666131213697322783095070105623107215331596699973092056135872020481"
        ));
        assert_eq!(
            added_addr,
            Ok(MaybeRelocatable::RelocatableValue(Relocatable {
                segment_index: 1,
                offset: 9
            }))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_int_to_int() {
        let addr_a = &MaybeRelocatable::from(felt_str!(
            "3618502788666131213697322783095070105623107215331596699973092056135872020488"
        ));
        let addr_b = &MaybeRelocatable::from(Felt252::new(17_i32));
        let added_addr = addr_a.add(addr_b);
        assert_eq!(added_addr, Ok(MaybeRelocatable::Int(Felt252::new(24))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_relocatable_to_relocatable_should_fail() {
        let addr_a = &MaybeRelocatable::from((7, 5));
        let addr_b = &MaybeRelocatable::RelocatableValue(relocatable!(7, 10));
        let error = addr_a.add(addr_b);
        assert_eq!(
            error,
            Err(MathError::RelocatableAdd(
                relocatable!(7, 5),
                relocatable!(7, 10)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_int_to_relocatable() {
        let addr_a = &MaybeRelocatable::from((7, 7));
        let addr_b = &MaybeRelocatable::from(Felt252::new(10));
        let added_addr = addr_a.add(addr_b);
        assert_eq!(
            added_addr,
            Ok(MaybeRelocatable::RelocatableValue(Relocatable {
                segment_index: 7,
                offset: 17
            }))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_relocatable_to_int() {
        let addr_a = &MaybeRelocatable::from(Felt252::new(10_i32));
        let addr_b = &MaybeRelocatable::RelocatableValue(relocatable!(7, 7));
        let added_addr = addr_a.add(addr_b);
        assert_eq!(
            added_addr,
            Ok(MaybeRelocatable::RelocatableValue(Relocatable {
                segment_index: 7,
                offset: 17
            }))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_int_to_relocatable_prime() {
        let addr_a = &MaybeRelocatable::from((7, 14));
        let addr_b = &MaybeRelocatable::Int(felt_str!(
            "800000000000011000000000000000000000000000000000000000000000001",
            16
        ));
        let added_addr = addr_a.add(addr_b);
        assert_eq!(
            added_addr,
            Ok(MaybeRelocatable::RelocatableValue(Relocatable {
                segment_index: 7,
                offset: 14
            }))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_int_rel_int_offset_exceeded() {
        let addr = MaybeRelocatable::from((0, 0));
        let error = addr.add(&MaybeRelocatable::from(felt_str!("18446744073709551616")));
        assert_eq!(
            error,
            Err(MathError::RelocatableAddFelt252OffsetExceeded(
                relocatable!(0, 0),
                felt_str!("18446744073709551616")
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_int_int_rel_offset_exceeded() {
        let addr = MaybeRelocatable::Int(felt_str!("18446744073709551616"));
        let relocatable = Relocatable {
            offset: 0,
            segment_index: 0,
        };
        let error = addr.add(&MaybeRelocatable::RelocatableValue(relocatable));
        assert_eq!(
            error,
            Err(MathError::RelocatableAddFelt252OffsetExceeded(
                relocatable!(0, 0),
                felt_str!("18446744073709551616")
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sub_int_from_int() {
        let addr_a = &MaybeRelocatable::from(Felt252::new(7));
        let addr_b = &MaybeRelocatable::from(Felt252::new(5));
        let sub_addr = addr_a.sub(addr_b);
        assert_eq!(sub_addr, Ok(MaybeRelocatable::Int(Felt252::new(2))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sub_relocatable_from_relocatable_same_offset() {
        let addr_a = &MaybeRelocatable::from((7, 17));
        let addr_b = &MaybeRelocatable::from((7, 7));
        let sub_addr = addr_a.sub(addr_b);
        assert_eq!(sub_addr, Ok(MaybeRelocatable::Int(Felt252::new(10))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sub_relocatable_from_relocatable_diff_offset() {
        let addr_a = &MaybeRelocatable::from((7, 17));
        let addr_b = &MaybeRelocatable::from((8, 7));
        let error = addr_a.sub(addr_b);
        assert_eq!(
            error,
            Err(MathError::RelocatableSubDiffIndex(
                relocatable!(7, 17),
                relocatable!(8, 7)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sub_int_addr_ref_from_relocatable_addr_ref() {
        let addr_a = &MaybeRelocatable::from((7, 17));
        let addr_b = &MaybeRelocatable::from(Felt252::new(5_i32));
        let addr_c = addr_a.sub(addr_b);
        assert_eq!(addr_c, Ok(MaybeRelocatable::from((7, 12))));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sub_rel_to_int_error() {
        assert_eq!(
            MaybeRelocatable::from(Felt252::new(7_i32)).sub(&MaybeRelocatable::from((7, 10))),
            Err(MathError::SubRelocatableFromInt(
                Felt252::new(7_i32),
                Relocatable::from((7, 10))
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn divmod_working() {
        let value = &MaybeRelocatable::from(Felt252::new(10));
        let div = &MaybeRelocatable::from(Felt252::new(3));
        let (q, r) = value.divmod(div).expect("Unexpected error in divmod");
        assert_eq!(
            q,
            MaybeRelocatable::from(Felt252::new(10) / Felt252::new(3))
        );
        assert_eq!(r, MaybeRelocatable::from(Felt252::zero()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn divmod_bad_type() {
        let value = &MaybeRelocatable::from(Felt252::new(10));
        let div = &MaybeRelocatable::from((2, 7));
        assert_eq!(
            value.divmod(div),
            Err(MathError::DivModWrongType(
                MaybeRelocatable::from(Felt252::new(10)),
                MaybeRelocatable::from((2, 7))
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocate_relocatable_value() {
        let value = MaybeRelocatable::from((2, 7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Ok(Felt252::new(12))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocate_relocatable_in_temp_segment_value() {
        let value = MaybeRelocatable::from((-1, 7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Err(MemoryError::TemporarySegmentInRelocation(-1)),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocate_relocatable_in_temp_segment_value_with_offset() {
        let value = MaybeRelocatable::from((-1, 7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Err(MemoryError::TemporarySegmentInRelocation(-1)),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocate_relocatable_in_temp_segment_value_error() {
        let value = MaybeRelocatable::from((-1, 7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Err(MemoryError::TemporarySegmentInRelocation(-1))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocate_int_value() {
        let value = MaybeRelocatable::from(Felt252::new(7));
        let relocation_table = vec![1, 2, 5];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Ok(Felt252::new(7))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocate_relocatable_value_no_relocation() {
        let value = MaybeRelocatable::from((2, 7));
        let relocation_table = vec![1, 2];
        assert_eq!(
            relocate_value(value, &relocation_table),
            Err(MemoryError::Relocation)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocatable_add_int() {
        assert_eq!(
            relocatable!(1, 2) + &Felt252::new(4),
            Ok(relocatable!(1, 6))
        );
        assert_eq!(
            relocatable!(3, 2) + &Felt252::zero(),
            Ok(relocatable!(3, 2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocatable_add_int_mod_offset_exceeded_error() {
        assert_eq!(
            relocatable!(0, 0) + &(Felt252::new(usize::MAX) + 1_usize),
            Err(MathError::RelocatableAddFelt252OffsetExceeded(
                relocatable!(0, 0),
                Felt252::new(usize::MAX) + 1_usize
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocatable_add_i32() {
        let reloc = relocatable!(1, 5);

        assert_eq!(reloc + 3, Ok(relocatable!(1, 8)));
        assert_eq!(reloc + (-3), Ok(relocatable!(1, 2)));
    }

    #[test]
    fn relocatable_add_i32_with_overflow() {
        let reloc = relocatable!(1, 1);

        assert_eq!(
            reloc + (-3),
            Err(MathError::RelocatableSubNegOffset(relocatable!(1, 1), 3))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn mayberelocatable_try_into_reloctable() {
        let address = mayberelocatable!(1, 2);
        assert_eq!(Ok(relocatable!(1, 2)), address.try_into());

        let value = mayberelocatable!(1);
        let err: Result<Relocatable, _> = value.try_into();
        assert_eq!(Err(MemoryError::AddressNotRelocatable), err)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocatable_sub_rel_test() {
        let reloc = relocatable!(7, 6);
        assert_eq!(reloc - relocatable!(7, 5), Ok(1));
        assert_eq!(
            reloc - relocatable!(7, 9),
            Err(MathError::RelocatableSubNegOffset(relocatable!(7, 6), 9))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sub_rel_different_indexes() {
        let a = relocatable!(7, 6);
        let b = relocatable!(8, 6);
        assert_eq!(a - b, Err(MathError::RelocatableSubDiffIndex(a, b)));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_maybe_mod_ok() {
        assert_eq!(
            relocatable!(1, 0) + &mayberelocatable!(2),
            Ok(relocatable!(1, 2))
        );
        assert_eq!(
            relocatable!(0, 29) + &mayberelocatable!(100),
            Ok(relocatable!(0, 129))
        );
        assert_eq!(
            relocatable!(2, 12) + &mayberelocatable!(104),
            Ok(relocatable!(2, 116))
        );
        assert_eq!(
            relocatable!(1, 0) + &mayberelocatable!(0),
            Ok(relocatable!(1, 0))
        );
        assert_eq!(
            relocatable!(1, 2) + &mayberelocatable!(71),
            Ok(relocatable!(1, 73))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_maybe_mod_add_two_relocatable_error() {
        assert_eq!(
            relocatable!(1, 0) + &mayberelocatable!(1, 2),
            Err(MathError::RelocatableAdd(
                relocatable!(1, 0),
                relocatable!(1, 2)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_maybe_mod_offset_exceeded_error() {
        assert_eq!(
            relocatable!(1, 0) + &mayberelocatable!(usize::MAX as i128 + 1),
            Err(MathError::RelocatableAddFelt252OffsetExceeded(
                relocatable!(1, 0),
                Felt252::new(usize::MAX) + 1_usize
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_relocatable_test() {
        assert_eq!(
            mayberelocatable!(1, 2).get_relocatable(),
            Some(relocatable!(1, 2))
        );
        assert_eq!(mayberelocatable!(3).get_relocatable(), None)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn relocatable_display() {
        assert_eq!(
            format!("{}", Relocatable::from((1, 0))),
            String::from("1:0")
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn maybe_relocatable_relocatable_display() {
        assert_eq!(
            format!("{}", MaybeRelocatable::from((1, 0))),
            String::from("1:0")
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn maybe_relocatable_int_display() {
        assert_eq!(
            format!("{}", MaybeRelocatable::from(Felt252::new(6))),
            String::from("6")
        )
    }

    #[test]
    fn relocatable_add_assign_usize() {
        let mut addr = Relocatable::from((1, 0));
        addr += 1;
        assert_eq!(addr, Relocatable::from((1, 1)))
    }
}
