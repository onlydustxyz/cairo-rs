#![cfg_attr(not(feature = "std"), no_std)]
#[allow(unused_imports)]
#[macro_use]
#[cfg(all(not(feature = "std"), feature = "alloc"))]
pub extern crate alloc;

mod bigint_felt;

use bigint_felt::{FeltBigInt, FIELD_HIGH, FIELD_LOW};
use num_bigint::{BigInt, BigUint, U64Digits};
use num_integer::Integer;
use num_traits::{Bounded, FromPrimitive, Num, One, Pow, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};

use core::{
    convert::Into,
    fmt,
    iter::Sum,
    ops::{
        Add, AddAssign, BitAnd, BitOr, BitXor, Div, Mul, MulAssign, Neg, Rem, Shl, Shr, ShrAssign,
        Sub, SubAssign,
    },
};

#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{string::String, vec::Vec};

pub const PRIME_STR: &str = "0x800000000000011000000000000000000000000000000000000000000000001"; // in decimal, this is equal to 3618502788666131213697322783095070105623107215331596699973092056135872020481

pub(crate) trait FeltOps {
    fn new<T: Into<FeltBigInt<FIELD_HIGH, FIELD_LOW>>>(value: T) -> Self;

    fn modpow(
        &self,
        exponent: &FeltBigInt<FIELD_HIGH, FIELD_LOW>,
        modulus: &FeltBigInt<FIELD_HIGH, FIELD_LOW>,
    ) -> Self;

    fn iter_u64_digits(&self) -> U64Digits;

    #[cfg(any(feature = "std", feature = "alloc"))]
    fn to_signed_bytes_le(&self) -> Vec<u8>;

    #[cfg(any(feature = "std", feature = "alloc"))]
    fn to_bytes_be(&self) -> Vec<u8>;

    fn parse_bytes(buf: &[u8], radix: u32) -> Option<FeltBigInt<FIELD_HIGH, FIELD_LOW>>;

    fn from_bytes_be(bytes: &[u8]) -> Self;

    #[cfg(any(feature = "std", feature = "alloc"))]
    fn to_str_radix(&self, radix: u32) -> String;

    #[deprecated]
    /// Converts [`Felt252`] into a [`BigInt`] number in the range: `(- FIELD / 2, FIELD / 2)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::cairo_felt::Felt252;
    /// # use num_bigint::BigInt;
    /// # use num_traits::Bounded;
    /// let positive = Felt252::new(5);
    /// assert_eq!(positive.to_bigint(), Into::<num_bigint::BigInt>::into(5));
    ///
    /// let negative = Felt252::max_value();
    /// assert_eq!(negative.to_bigint(), Into::<num_bigint::BigInt>::into(-1));
    /// ```
    fn to_bigint(&self) -> BigInt;

    #[deprecated]
    /// Converts [`Felt252`] into a [`BigUint`] number.
    ///
    /// # Examples
    ///
    /// ```
    /// # use crate::cairo_felt::Felt252;
    /// # use num_bigint::BigUint;
    /// # use num_traits::{Num, Bounded};
    /// let positive = Felt252::new(5);
    /// assert_eq!(positive.to_biguint(), Into::<num_bigint::BigUint>::into(5_u32));
    ///
    /// let negative = Felt252::max_value();
    /// assert_eq!(negative.to_biguint(), BigUint::from_str_radix("800000000000011000000000000000000000000000000000000000000000000", 16).unwrap());
    /// ```
    fn to_biguint(&self) -> BigUint;

    fn sqrt(&self) -> Self;

    fn bits(&self) -> u64;

    fn prime() -> BigUint;
}

#[macro_export]
macro_rules! felt_str {
    ($val: expr) => {
        $crate::Felt252::parse_bytes($val.as_bytes(), 10_u32).expect("Couldn't parse bytes")
    };
    ($val: expr, $opt: expr) => {
        $crate::Felt252::parse_bytes($val.as_bytes(), $opt as u32).expect("Couldn't parse bytes")
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseFeltError;

#[derive(Eq, Hash, PartialEq, PartialOrd, Ord, Clone, Deserialize, Default, Serialize)]
pub struct Felt252 {
    value: FeltBigInt<FIELD_HIGH, FIELD_LOW>,
}

macro_rules! from_num {
    ($type:ty) => {
        impl From<$type> for Felt252 {
            fn from(value: $type) -> Self {
                Self {
                    value: value.into(),
                }
            }
        }
    };
}

from_num!(i8);
from_num!(i16);
from_num!(i32);
from_num!(i64);
from_num!(i128);
from_num!(isize);
from_num!(u8);
from_num!(u16);
from_num!(u32);
from_num!(u64);
from_num!(u128);
from_num!(usize);
from_num!(BigInt);
from_num!(&BigInt);
from_num!(BigUint);
from_num!(&BigUint);

impl Felt252 {
    pub fn new<T: Into<Felt252>>(value: T) -> Self {
        value.into()
    }
    pub fn modpow(&self, exponent: &Felt252, modulus: &Felt252) -> Self {
        Self {
            value: self.value.modpow(&exponent.value, &modulus.value),
        }
    }
    pub fn iter_u64_digits(&self) -> U64Digits {
        self.value.iter_u64_digits()
    }

    pub fn to_le_bytes(&self) -> [u8; 32] {
        let mut res = [0u8; 32];
        let mut iter = self.iter_u64_digits();
        let (d0, d1, d2, d3) = (
            iter.next().unwrap_or_default().to_le_bytes(),
            iter.next().unwrap_or_default().to_le_bytes(),
            iter.next().unwrap_or_default().to_le_bytes(),
            iter.next().unwrap_or_default().to_le_bytes(),
        );
        res[..8].copy_from_slice(&d0);
        res[8..16].copy_from_slice(&d1);
        res[16..24].copy_from_slice(&d2);
        res[24..].copy_from_slice(&d3);
        res
    }

    pub fn to_be_bytes(&self) -> [u8; 32] {
        let mut bytes = self.to_le_bytes();
        bytes.reverse();
        bytes
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn to_signed_bytes_le(&self) -> Vec<u8> {
        self.value.to_signed_bytes_le()
    }
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn to_bytes_be(&self) -> Vec<u8> {
        self.value.to_bytes_be()
    }
    pub fn parse_bytes(buf: &[u8], radix: u32) -> Option<Self> {
        Some(Self {
            value: FeltBigInt::parse_bytes(buf, radix)?,
        })
    }
    pub fn from_bytes_be(bytes: &[u8]) -> Self {
        Self {
            value: FeltBigInt::from_bytes_be(bytes),
        }
    }
    #[cfg(any(feature = "std", feature = "alloc"))]
    pub fn to_str_radix(&self, radix: u32) -> String {
        self.value.to_str_radix(radix)
    }
    pub fn to_bigint(&self) -> BigInt {
        #[allow(deprecated)]
        self.value.to_bigint()
    }
    pub fn to_biguint(&self) -> BigUint {
        #[allow(deprecated)]
        self.value.to_biguint()
    }
    pub fn sqrt(&self) -> Self {
        Self {
            value: self.value.sqrt(),
        }
    }
    pub fn bits(&self) -> u64 {
        self.value.bits()
    }

    pub fn prime() -> BigUint {
        FeltBigInt::prime()
    }
}

impl Add for Felt252 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            value: self.value + rhs.value,
        }
    }
}

impl<'a> Add for &'a Felt252 {
    type Output = Felt252;
    fn add(self, rhs: Self) -> Self::Output {
        Self::Output {
            value: &self.value + &rhs.value,
        }
    }
}

impl<'a> Add<&'a Felt252> for Felt252 {
    type Output = Self;
    fn add(self, rhs: &Self) -> Self::Output {
        Self::Output {
            value: self.value + &rhs.value,
        }
    }
}

impl Add<u32> for Felt252 {
    type Output = Self;
    fn add(self, rhs: u32) -> Self {
        Self {
            value: self.value + rhs,
        }
    }
}

impl Add<usize> for Felt252 {
    type Output = Self;
    fn add(self, rhs: usize) -> Self {
        Self {
            value: self.value + rhs,
        }
    }
}

impl<'a> Add<usize> for &'a Felt252 {
    type Output = Felt252;
    fn add(self, rhs: usize) -> Self::Output {
        Self::Output {
            value: &self.value + rhs,
        }
    }
}

impl AddAssign for Felt252 {
    fn add_assign(&mut self, rhs: Self) {
        self.value += rhs.value;
    }
}

impl<'a> AddAssign<&'a Felt252> for Felt252 {
    fn add_assign(&mut self, rhs: &Self) {
        self.value += &rhs.value;
    }
}

impl Sum for Felt252 {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(Felt252::zero(), |mut acc, x| {
            acc += x;
            acc
        })
    }
}

impl Neg for Felt252 {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            value: self.value.neg(),
        }
    }
}

impl<'a> Neg for &'a Felt252 {
    type Output = Felt252;
    fn neg(self) -> Self::Output {
        Self::Output {
            value: (&self.value).neg(),
        }
    }
}

impl Sub for Felt252 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            value: self.value - rhs.value,
        }
    }
}

impl<'a> Sub for &'a Felt252 {
    type Output = Felt252;
    fn sub(self, rhs: Self) -> Self::Output {
        Self::Output {
            value: &self.value - &rhs.value,
        }
    }
}

impl<'a> Sub<&'a Felt252> for Felt252 {
    type Output = Self;
    fn sub(self, rhs: &Self) -> Self {
        Self {
            value: self.value - &rhs.value,
        }
    }
}

impl Sub<&Felt252> for usize {
    type Output = Felt252;
    fn sub(self, rhs: &Self::Output) -> Self::Output {
        Self::Output {
            value: self - &rhs.value,
        }
    }
}

impl SubAssign for Felt252 {
    fn sub_assign(&mut self, rhs: Self) {
        self.value -= rhs.value
    }
}

impl<'a> SubAssign<&'a Felt252> for Felt252 {
    fn sub_assign(&mut self, rhs: &Self) {
        self.value -= &rhs.value;
    }
}

impl Sub<u32> for Felt252 {
    type Output = Self;
    fn sub(self, rhs: u32) -> Self {
        Self {
            value: self.value - rhs,
        }
    }
}

impl<'a> Sub<u32> for &'a Felt252 {
    type Output = Felt252;
    fn sub(self, rhs: u32) -> Self::Output {
        Self::Output {
            value: &self.value - rhs,
        }
    }
}

impl Sub<usize> for Felt252 {
    type Output = Self;
    fn sub(self, rhs: usize) -> Self {
        Self {
            value: self.value - rhs,
        }
    }
}

impl Mul for Felt252 {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self {
            value: self.value * rhs.value,
        }
    }
}

impl<'a> Mul for &'a Felt252 {
    type Output = Felt252;
    fn mul(self, rhs: Self) -> Self::Output {
        Self::Output {
            value: &self.value * &rhs.value,
        }
    }
}

impl<'a> Mul<&'a Felt252> for Felt252 {
    type Output = Self;
    fn mul(self, rhs: &Self) -> Self {
        Self {
            value: self.value * &rhs.value,
        }
    }
}

impl<'a> MulAssign<&'a Felt252> for Felt252 {
    fn mul_assign(&mut self, rhs: &Self) {
        self.value *= &rhs.value;
    }
}

impl Pow<u32> for Felt252 {
    type Output = Self;
    fn pow(self, rhs: u32) -> Self {
        Self {
            value: self.value.pow(rhs),
        }
    }
}

impl<'a> Pow<u32> for &'a Felt252 {
    type Output = Felt252;
    fn pow(self, rhs: u32) -> Self::Output {
        Self::Output {
            value: (&self.value).pow(rhs),
        }
    }
}

impl<'a> Pow<&'a Felt252> for &'a Felt252 {
    type Output = Felt252;
    fn pow(self, rhs: &'a Felt252) -> Self::Output {
        Self::Output {
            value: (&self.value).pow(&rhs.value),
        }
    }
}

impl Div for Felt252 {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        Self {
            value: self.value / rhs.value,
        }
    }
}

impl<'a> Div for &'a Felt252 {
    type Output = Felt252;
    fn div(self, rhs: Self) -> Self::Output {
        Self::Output {
            value: &self.value / &rhs.value,
        }
    }
}

impl<'a> Div<Felt252> for &'a Felt252 {
    type Output = Felt252;
    fn div(self, rhs: Self::Output) -> Self::Output {
        Self::Output {
            value: &self.value / rhs.value,
        }
    }
}

impl Rem for Felt252 {
    type Output = Self;
    fn rem(self, rhs: Self) -> Self {
        Self {
            value: self.value % rhs.value,
        }
    }
}

impl<'a> Rem<&'a Felt252> for Felt252 {
    type Output = Self;
    fn rem(self, rhs: &Self) -> Self {
        Self {
            value: self.value % &rhs.value,
        }
    }
}

impl Zero for Felt252 {
    fn zero() -> Self {
        Self {
            value: FeltBigInt::zero(),
        }
    }

    fn is_zero(&self) -> bool {
        self.value.is_zero()
    }
}

impl One for Felt252 {
    fn one() -> Self {
        Self {
            value: FeltBigInt::one(),
        }
    }

    fn is_one(&self) -> bool {
        self.value.is_one()
    }
}

impl Bounded for Felt252 {
    fn min_value() -> Self {
        Self {
            value: FeltBigInt::min_value(),
        }
    }

    fn max_value() -> Self {
        Self {
            value: FeltBigInt::max_value(),
        }
    }
}

impl Num for Felt252 {
    type FromStrRadixErr = ParseFeltError;
    fn from_str_radix(string: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        Ok(Self {
            value: FeltBigInt::from_str_radix(string, radix)?,
        })
    }
}

impl Integer for Felt252 {
    fn div_floor(&self, rhs: &Self) -> Self {
        Self {
            value: self.value.div_floor(&rhs.value),
        }
    }

    fn div_rem(&self, other: &Self) -> (Self, Self) {
        let (div, rem) = self.value.div_rem(&other.value);
        (Self { value: div }, Self { value: rem })
    }

    fn divides(&self, other: &Self) -> bool {
        self.value.divides(&other.value)
    }

    fn gcd(&self, other: &Self) -> Self {
        Self {
            value: self.value.gcd(&other.value),
        }
    }

    fn is_even(&self) -> bool {
        self.value.is_even()
    }

    fn is_multiple_of(&self, other: &Self) -> bool {
        self.value.is_multiple_of(&other.value)
    }

    fn is_odd(&self) -> bool {
        self.value.is_odd()
    }

    fn lcm(&self, other: &Self) -> Self {
        Self {
            value: self.value.lcm(&other.value),
        }
    }

    fn mod_floor(&self, rhs: &Self) -> Self {
        Self {
            value: self.value.mod_floor(&rhs.value),
        }
    }
}

impl Signed for Felt252 {
    fn abs(&self) -> Self {
        Self {
            value: self.value.abs(),
        }
    }

    fn abs_sub(&self, other: &Self) -> Self {
        Self {
            value: self.value.abs_sub(&other.value),
        }
    }

    fn signum(&self) -> Self {
        Self {
            value: self.value.signum(),
        }
    }

    fn is_positive(&self) -> bool {
        self.value.is_positive()
    }

    fn is_negative(&self) -> bool {
        self.value.is_negative()
    }
}

impl Shl<u32> for Felt252 {
    type Output = Self;
    fn shl(self, rhs: u32) -> Self {
        Self {
            value: self.value << rhs,
        }
    }
}

impl<'a> Shl<u32> for &'a Felt252 {
    type Output = Felt252;
    fn shl(self, rhs: u32) -> Self::Output {
        Self::Output {
            value: &self.value << rhs,
        }
    }
}

impl Shl<usize> for Felt252 {
    type Output = Self;
    fn shl(self, rhs: usize) -> Self {
        Self {
            value: self.value << rhs,
        }
    }
}

impl<'a> Shl<usize> for &'a Felt252 {
    type Output = Felt252;
    fn shl(self, rhs: usize) -> Self::Output {
        Self::Output {
            value: &self.value << rhs,
        }
    }
}

impl Shr<u32> for Felt252 {
    type Output = Self;
    fn shr(self, rhs: u32) -> Self {
        Self {
            value: self.value >> rhs,
        }
    }
}

impl<'a> Shr<u32> for &'a Felt252 {
    type Output = Felt252;
    fn shr(self, rhs: u32) -> Self::Output {
        Self::Output {
            value: &self.value >> rhs,
        }
    }
}

impl ShrAssign<usize> for Felt252 {
    fn shr_assign(&mut self, rhs: usize) {
        self.value >>= rhs
    }
}

impl<'a> BitAnd for &'a Felt252 {
    type Output = Felt252;
    fn bitand(self, rhs: Self) -> Self::Output {
        Self::Output {
            value: &self.value & &rhs.value,
        }
    }
}

impl<'a> BitAnd<&'a Felt252> for Felt252 {
    type Output = Self;
    fn bitand(self, rhs: &Self) -> Self {
        Self {
            value: self.value & &rhs.value,
        }
    }
}

impl<'a> BitAnd<Felt252> for &'a Felt252 {
    type Output = Felt252;
    fn bitand(self, rhs: Self::Output) -> Self::Output {
        Self::Output {
            value: &self.value & rhs.value,
        }
    }
}

impl<'a> BitOr for &'a Felt252 {
    type Output = Felt252;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self::Output {
            value: &self.value | &rhs.value,
        }
    }
}

impl<'a> BitXor for &'a Felt252 {
    type Output = Felt252;
    fn bitxor(self, rhs: Self) -> Self::Output {
        Self::Output {
            value: &self.value ^ &rhs.value,
        }
    }
}

impl ToPrimitive for Felt252 {
    fn to_u128(&self) -> Option<u128> {
        self.value.to_u128()
    }

    fn to_u64(&self) -> Option<u64> {
        self.value.to_u64()
    }

    fn to_i64(&self) -> Option<i64> {
        self.value.to_i64()
    }
}

impl FromPrimitive for Felt252 {
    fn from_u64(n: u64) -> Option<Self> {
        FeltBigInt::from_u64(n).map(|n| Self { value: n })
    }

    fn from_i64(n: i64) -> Option<Self> {
        FeltBigInt::from_i64(n).map(|n| Self { value: n })
    }
}

impl fmt::Display for Felt252 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl fmt::Debug for Felt252 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

macro_rules! assert_felt_methods {
    ($type:ty) => {
        const _: () = {
            fn assert_felt_ops<T: FeltOps>() {}
            fn assertion() {
                assert_felt_ops::<$type>();
            }
        };
    };
}

macro_rules! assert_felt_impl {
    ($type:ty) => {
        const _: () = {
            fn assert_add<T: Add>() {}
            fn assert_add_ref<'a, T: Add<&'a $type>>() {}
            fn assert_add_u32<T: Add<u32>>() {}
            fn assert_add_usize<T: Add<usize>>() {}
            fn assert_add_assign<T: AddAssign>() {}
            fn assert_add_assign_ref<'a, T: AddAssign<&'a $type>>() {}
            fn assert_sum<T: Sum<$type>>() {}
            fn assert_neg<T: Neg>() {}
            fn assert_sub<T: Sub>() {}
            fn assert_sub_ref<'a, T: Sub<&'a $type>>() {}
            fn assert_sub_assign<T: SubAssign>() {}
            fn assert_sub_assign_ref<'a, T: SubAssign<&'a $type>>() {}
            fn assert_sub_u32<T: Sub<u32>>() {}
            fn assert_sub_usize<T: Sub<usize>>() {}
            fn assert_mul<T: Mul>() {}
            fn assert_mul_ref<'a, T: Mul<&'a $type>>() {}
            fn assert_mul_assign_ref<'a, T: MulAssign<&'a $type>>() {}
            fn assert_pow_u32<T: Pow<u32>>() {}
            fn assert_pow_felt<'a, T: Pow<&'a $type>>() {}
            fn assert_div<T: Div>() {}
            fn assert_ref_div<T: Div<$type>>() {}
            fn assert_rem<T: Rem>() {}
            fn assert_rem_ref<'a, T: Rem<&'a $type>>() {}
            fn assert_zero<T: Zero>() {}
            fn assert_one<T: One>() {}
            fn assert_bounded<T: Bounded>() {}
            fn assert_num<T: Num>() {}
            fn assert_integer<T: Integer>() {}
            fn assert_signed<T: Signed>() {}
            fn assert_shl_u32<T: Shl<u32>>() {}
            fn assert_shl_usize<T: Shl<usize>>() {}
            fn assert_shr_u32<T: Shr<u32>>() {}
            fn assert_shr_assign_usize<T: ShrAssign<usize>>() {}
            fn assert_bitand<T: BitAnd>() {}
            fn assert_bitand_ref<'a, T: BitAnd<&'a $type>>() {}
            fn assert_ref_bitand<T: BitAnd<$type>>() {}
            fn assert_bitor<T: BitOr>() {}
            fn assert_bitxor<T: BitXor>() {}
            fn assert_from_primitive<T: FromPrimitive>() {}
            fn assert_to_primitive<T: ToPrimitive>() {}
            fn assert_display<T: fmt::Display>() {}
            fn assert_debug<T: fmt::Debug>() {}

            #[allow(dead_code)]
            fn assert_all() {
                assert_add::<$type>();
                assert_add::<&$type>();
                assert_add_ref::<$type>();
                assert_add_u32::<$type>();
                assert_add_usize::<$type>();
                assert_add_usize::<&$type>();
                assert_add_assign::<$type>();
                assert_add_assign_ref::<$type>();
                assert_sum::<$type>();
                assert_neg::<$type>();
                assert_neg::<&$type>();
                assert_sub::<$type>();
                assert_sub::<&$type>();
                assert_sub_ref::<$type>();
                assert_sub_assign::<$type>();
                assert_sub_assign_ref::<$type>();
                assert_sub_u32::<$type>();
                assert_sub_u32::<&$type>();
                assert_sub_usize::<$type>();
                assert_mul::<$type>();
                assert_mul::<&$type>();
                assert_mul_ref::<$type>();
                assert_mul_assign_ref::<$type>();
                assert_pow_u32::<$type>();
                assert_pow_felt::<&$type>();
                assert_div::<$type>();
                assert_div::<&$type>();
                assert_ref_div::<&$type>();
                assert_rem::<$type>();
                assert_rem_ref::<$type>();
                assert_zero::<$type>();
                assert_one::<$type>();
                assert_bounded::<$type>();
                assert_num::<$type>();
                assert_integer::<$type>();
                assert_signed::<$type>();
                assert_shl_u32::<$type>();
                assert_shl_u32::<&$type>();
                assert_shl_usize::<$type>();
                assert_shl_usize::<&$type>();
                assert_shr_u32::<$type>();
                assert_shr_u32::<&$type>();
                assert_shr_assign_usize::<$type>();
                assert_bitand::<&$type>();
                assert_bitand_ref::<$type>();
                assert_ref_bitand::<&$type>();
                assert_bitor::<&$type>();
                assert_bitxor::<&$type>();
                assert_from_primitive::<$type>();
                assert_to_primitive::<$type>();
                assert_display::<$type>();
                assert_debug::<$type>();
            }
        };
    };
}

assert_felt_methods!(FeltBigInt<FIELD_HIGH, FIELD_LOW>);
assert_felt_impl!(FeltBigInt<FIELD_HIGH, FIELD_LOW>);
assert_felt_impl!(Felt252);

#[cfg(test)]
mod test {
    use super::*;
    use core::cmp;
    use proptest::prelude::*;

    const FELT_PATTERN: &str = "(0|[1-9][0-9]*)";
    const FELT_NON_ZERO_PATTERN: &str = "[1-9][0-9]*";

    proptest! {
        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property-based test that ensures, for 100 felt values that are randomly generated each time tests are run, that a new felt doesn't fall outside the range [0, p].
        // In this and some of the following tests, The value of {x} can be either [0] or a very large number, in order to try to overflow the value of {p} and thus ensure the modular arithmetic is working correctly.
        fn new_in_range(ref x in any::<[u8; 40]>()) {
            let x = &Felt252::from_bytes_be(x);
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            prop_assert!(&x.to_biguint() < p);
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        fn to_be_bytes(ref x in any::<[u8; 40]>()) {
            let x = &Felt252::from_bytes_be(x);
            let bytes = x.to_be_bytes();
            let y = &Felt252::from_bytes_be(&bytes);
            prop_assert_eq!(x, y);
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        fn to_le_bytes(ref x in any::<[u8; 40]>()) {
            let x = &Felt252::from_bytes_be(x);
            let mut bytes = x.to_le_bytes();
            // Convert to big endian for test
            bytes.reverse();
            let y = &Felt252::from_bytes_be(&bytes);
            prop_assert_eq!(x, y);
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        fn to_u128_ok(x in any::<u128>()) {
            let y = &Felt252::from(x);
            let y = y.to_u128();
            prop_assert_eq!(Some(x), y);
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        fn to_u128_out_of_range(ref x in any::<[u8; 31]>()) {
            let y = &Felt252::from_bytes_be(x) + &Felt252::from(u128::MAX);
            let y = y.to_u128();
            prop_assert_eq!(None, y);
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property-based test that ensures, for 100 felt values that are randomly generated each time tests are run, that a felt created using Felt252::from_bytes_be doesn't fall outside the range [0, p].
        // In this and some of the following tests, The value of {x} can be either [0] or a very large number, in order to try to overflow the value of {p} and thus ensure the modular arithmetic is working correctly.
        fn from_bytes_be_in_range(ref x in FELT_PATTERN) {
            let x = &Felt252::from_bytes_be(x.as_bytes());
            let max_felt = &Felt252::max_value();
            prop_assert!(x <= max_felt);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property-based test that ensures, for 100 felt values that are randomly generated each time tests are run, that the negative of a felt doesn't fall outside the range [0, p].
        fn neg_in_range(ref x in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let neg = -x.clone();
            let as_uint = &neg.to_biguint();
            prop_assert!(as_uint < p);

            // test reference variant
            let neg = -&x;
            let as_uint = &neg.to_biguint();
            prop_assert!(as_uint < p);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property-based test that ensures, for 100 {x} and {y} values that are randomly generated each time tests are run, that a subtraction between two felts {x} and {y} and doesn't fall outside the range [0, p]. The values of {x} and {y} can be either [0] or a very large number.
        fn sub(ref x in any::<[u8; 32]>(), ref y in any::<[u8; 32]>()) {
            let (x, y) = (&Felt252::from_bytes_be(x), &Felt252::from_bytes_be(y));
            let (x_int, y_int) = (&x.to_biguint(), &y.to_biguint());
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let sub_xy = x - y;
            prop_assert!(&sub_xy.to_biguint() < p);
            prop_assert_eq!(Felt252::from(p + x_int - y_int), sub_xy);

            let sub_yx = y - x;
            prop_assert!(&sub_yx.to_biguint() < p);
            prop_assert_eq!(Felt252::from(p + y_int - x_int), sub_yx);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property-based test that ensures, for 100 {x} and {y} values that are randomly generated each time tests are run, that a subtraction with assignment between two felts {x} and {y} and doesn't fall outside the range [0, p]. The values of {x} and {y} can be either [0] or a very large number.
        fn sub_assign_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let mut x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            x -= y.clone();
            let as_uint = &x.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);

            // test reference variant
            x -= &y;
            let as_uint = &x.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property-based test that ensures, for 100 {x} and {y} values that are randomly generated each time tests are run, that a multiplication between two felts {x} and {y} and doesn't fall outside the range [0, p]. The values of {x} and {y} can be either [0] or a very large number.
        fn mul(ref x in any::<[u8; 32]>(), ref y in any::<[u8; 32]>()) {
            let xy_int = &BigUint::from_bytes_be(x) * &BigUint::from_bytes_be(y);

            let x = &Felt252::from_bytes_be(x);
            let y = &Felt252::from_bytes_be(y);
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let (xy, yx) = (x * y, y * x);
            prop_assert_eq!(&xy, &yx);
            prop_assert_eq!(xy.to_biguint(), xy_int.mod_floor(p));
            prop_assert!(&xy.to_biguint() < p);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property-based test that ensures, for 100 pairs of {x} and {y} values that are randomly generated each time tests are run, that a multiplication with assignment between two felts {x} and {y} and doesn't fall outside the range [0, p]. The values of {x} and {y} can be either [0] or a very large number.
        fn mul_assign_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let mut x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = &Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            x *= y;
            let as_uint = &x.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property-based test that ensures, for 100 pairs of {x} and {y} values that are randomly generated each time tests are run, that the result of the division of {x} by {y} is the inverse multiplicative of {x} --that is, multiplying the result by {y} returns the original number {x}. The values of {x} and {y} can be either [0] or a very large number.
        fn div_is_mul_inv(ref x in FELT_PATTERN, ref y in FELT_NON_ZERO_PATTERN) {
            let x = &Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = &Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            prop_assume!(!y.is_zero());

            let q = x / y;
            let as_uint = &q.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
            prop_assert_eq!(&(q * y), x);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
         // Property-based test that ensures, for 100 {value}s that are randomly generated each time tests are run, that performing a bit shift to the left by {shift_amount} of bits (between 0 and 999) returns a result that is inside of the range [0, p].
        fn shift_left_in_range(ref value in FELT_PATTERN, ref shift_amount in "[0-9]{1,3}"){
            let value = Felt252::parse_bytes(value.as_bytes(), 10).unwrap();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let shift_amount:u32 = shift_amount.parse::<u32>().unwrap();
            let result = (value.clone() << shift_amount).to_biguint();
            prop_assert!(&result < p);

            let result = (&value << shift_amount).to_biguint();
            prop_assert!(&result < p);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
         // Property-based test that ensures, for 100 {value}s that are randomly generated each time tests are run, that performing a bit shift to the right by {shift_amount} of bits (between 0 and 999) returns a result that is inside of the range [0, p].
        fn shift_right_in_range(ref value in FELT_PATTERN, ref shift_amount in "[0-9]{1,3}"){
            let value = Felt252::parse_bytes(value.as_bytes(), 10).unwrap();
            let shift_amount:u32 = shift_amount.parse::<u32>().unwrap();
            let result = (value >> shift_amount).to_biguint();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            prop_assert!(&result < p);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property-based test that ensures, for 100 {value}s that are randomly generated each time tests are run, that performing a bit shift to the right by {shift_amount} of bits (between 0 and 999), with assignment, returns a result that is inside of the range [0, p].
        // "With assignment" means that the result of the operation is autommatically assigned to the variable value, replacing its previous content.
        fn shift_right_assign_in_range(ref value in FELT_PATTERN, ref shift_amount in "[0-9]{1,3}"){
            let mut value = Felt252::parse_bytes(value.as_bytes(), 10).unwrap();
            let shift_amount:usize = shift_amount.parse::<usize>().unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            value >>= shift_amount;
            prop_assert!(value.to_biguint() < p);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property based test that ensures, for 100 pairs of values {x} and {y} generated at random each time tests are run, that performing a BitAnd operation between them returns a result that is inside of the range [0, p].
        fn bitand_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN){
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            let result = &x & &y;
            result.to_biguint();
            prop_assert!(result.to_biguint() < p);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property based test that ensures, for 100 pairs of values {x} and {y} generated at random each time tests are run, that performing a BitOr operation between them returns a result that is inside of the range [0, p].
        fn bitor_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN){
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            let result = &x | &y;
            prop_assert!(result.to_biguint() < p);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property based test that ensures, for 100 pairs of values {x} and {y} generated at random each time tests are run, that performing a BitXor operation between them returns a result that is inside of the range [0, p].
        fn bitxor_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN){
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            let result = &x ^ &y;
            prop_assert!(result.to_biguint() < p);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
         // Property-based test that ensures, for 100 values {x} that are randomly generated each time tests are run, that raising {x} to the {y}th power returns a result that is inside of the range [0, p].
        fn pow_in_range(ref x in FELT_PATTERN, ref y in "[0-9]{1,2}"){
            let base = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let exponent:u32 = y.parse()?;
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let result = Pow::pow(base.clone(), exponent);
            let as_uint = &result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);

            // test reference variant
            let result = Pow::pow(&base, exponent);
            let as_uint = &result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }

        #[test]
        #[allow(deprecated)]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
         // Property-based test that ensures, for 100 values {x} that are randomly generated each time tests are run, that raising {x} to the {y}th power returns a result that is inside of the range [0, p].
        fn pow_felt_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN){
            let base = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let exponent = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let result = Pow::pow(&base, &exponent);
            let as_uint = result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);

            // test reference variant
            let result: Felt252 = Pow::pow(&base, &exponent);
            let as_uint = result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property based test that ensures, for 100 pairs of values {x} and {y} generated at random each time tests are run, that performing a Sum operation between them returns a result that is inside of the range [0, p].
        fn sum_in_range(ref x in FELT_NON_ZERO_PATTERN, ref y in "[0-9][0-9]*"){
            let x = &Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = &Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let result = x + y;
            let as_uint = &result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property test to check that the remainder of a division between 100 pairs of values {x} and {y},generated at random each time tests are run, falls in the range [0, p]. x and y can either take the value of 0 or a large integer.
        // In Cairo, the result of x / y is defined to always satisfy the equation (x / y) * y == x, so the remainder is 0 most of the time.
        fn rem_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let result = x.clone() % y.clone();
            let as_uint = &result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);

            // test reference variant
            let result = x % &y;
            let as_uint = &result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property based test that ensures, for 100 Felt252s {x} generated at random each time tests are run, that converting them into the u64 type returns a result that is inside of the range [0, p].
        fn from_u64_and_to_u64_primitive(x in any::<u64>()) {
           let x_felt:Felt252 = Felt252::from_u64(x).unwrap();
           let x_u64:u64 = Felt252::to_u64(&x_felt).unwrap();

            prop_assert_eq!(x, x_u64);
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        fn from_i64_and_to_i64_primitive(x in any::<u32>()) {
            let x: i64 = x as i64;
            let x_felt:Felt252 = Felt252::from_i64(x).unwrap();
            let x_i64:i64 = Felt252::to_i64(&x_felt).unwrap();
            prop_assert_eq!(x, x_i64);
        }

        #[test]
        // Property test to check that lcm(x, y) works. Since we're operating in a prime field, lcm
        // will just be the smaller number.
        fn lcm_doesnt_panic(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let lcm = x.lcm(&y);
            prop_assert!(lcm == cmp::max(x, y));
        }

        #[test]
        #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
        // Property test to check that is_multiple_of(x, y) works. Since we're operating in a prime field, is_multiple_of
        // will always be true
        fn is_multiple_of_doesnt_panic(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
                 let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
                 let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
                 prop_assert!(x.is_multiple_of(&y));
        }

        #[test]
        fn divides_doesnt_panic(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            prop_assert!(x.divides(&y));
        }

        #[test]
        fn gcd_doesnt_panic(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let gcd1 = x.gcd(&y);
            let gcd2 = y.gcd(&x);
            prop_assert_eq!(gcd1, gcd2);
        }

        #[test]
        fn is_even(ref x in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            prop_assert_eq!(x.is_even(), x.to_biguint().is_even());
        }

        #[test]
        fn is_odd(ref x in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            prop_assert_eq!(x.is_odd(), x.to_biguint().is_odd());
        }

        /// Tests the additive identity of the implementation of Zero trait for felts
        ///
        /// ```{.text}
        /// x + 0 = x       ∀ x
        /// 0 + x = x       ∀ x
        /// ```
        #[test]
        fn zero_additive_identity(ref x in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let zero = Felt252::zero();
            prop_assert_eq!(&x, &(&x + &zero));
            prop_assert_eq!(&x, &(&zero + &x));
        }

        /// Tests the multiplicative identity of the implementation of One trait for felts
        ///
        /// ```{.text}
        /// x * 1 = x       ∀ x
        /// 1 * x = x       ∀ x
        /// ```
        #[test]
        fn one_multiplicative_identity(ref x in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let one = Felt252::one();
            prop_assert_eq!(&x, &(&x * &one));
            prop_assert_eq!(&x, &(&one * &x));
        }

        #[test]
        fn non_zero_felt_is_always_positive(ref x in FELT_NON_ZERO_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            prop_assert!(x.is_positive())
        }

        #[test]
        fn felt_is_never_negative(ref x in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            prop_assert!(!x.is_negative())
        }

        #[test]
        fn non_zero_felt_signum_is_always_one(ref x in FELT_NON_ZERO_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let one = Felt252::one();
            prop_assert_eq!(x.signum(), one)
        }

        #[test]
        fn sub_abs(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();

            let expected_abs_sub = if x > y {&x - &y} else {&y - &x};

            prop_assert_eq!(&x.abs_sub(&y), &expected_abs_sub)
        }

        #[test]
        fn abs(ref x in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            prop_assert_eq!(&x, &x.abs())
        }

        #[test]
        fn modpow_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = &Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let p_felt = Felt252::max_value();

            let modpow = x.modpow(y, &p_felt).to_biguint();
            prop_assert!(modpow < p, "{}", modpow);
        }

        #[test]
        fn sqrt_in_range(ref x in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let sqrt = x.sqrt().to_biguint();
            prop_assert!(sqrt < p, "{}", sqrt);
        }

        #[test]
        fn sqrt_is_inv_square(ref x in FELT_PATTERN) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            prop_assert_eq!((&x * &x).sqrt(), x);
        }

        #[test]
        fn add_u32_in_range(ref x in FELT_PATTERN, y in any::<u32>()) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            let x_add_y = (x + y).to_biguint();
            prop_assert!(x_add_y < p, "{}", x_add_y);
        }

        #[test]
        fn add_u32_is_inv_sub(ref x in FELT_PATTERN, y in any::<u32>()) {
            let x = &Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let expected_y = (x.clone() + y - x).to_u32().unwrap();
            prop_assert_eq!(expected_y, y, "{}", expected_y);
        }

        #[test]
        fn sub_u32_in_range(ref x in FELT_PATTERN, y in any::<u32>()) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            let x_sub_y = (x - y).to_biguint();
            prop_assert!(x_sub_y < p, "{}", x_sub_y);
        }

        #[test]
        fn sub_u32_is_inv_add(ref x in FELT_NON_ZERO_PATTERN, y in any::<u32>()) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            prop_assert_eq!(x.clone() - y + y, x)
        }

        #[test]
        fn sub_usize_in_range(ref x in FELT_PATTERN, y in any::<usize>()) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let p = BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
            let x_sub_y = (x - y).to_biguint();
            prop_assert!(x_sub_y < p, "{}", x_sub_y);
        }

        #[test]
        fn sub_usize_is_inv_add(ref x in FELT_PATTERN, y in any::<usize>()) {
            let x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            prop_assert_eq!(x.clone() - y + y, x)
        }

        #[test]
        fn add_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let x = &Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = &Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            let sub = x + y;
            let as_uint = &sub.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }

        #[test]
        fn add_is_inv_sub(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let x = &Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = &Felt252::parse_bytes(y.as_bytes(), 10).unwrap();

            let expected_y = x + y - x;
            prop_assert_eq!(&expected_y, y, "{}", y);
        }

        #[test]
        fn add_assign_in_range(ref x in FELT_PATTERN, ref y in FELT_PATTERN) {
            let mut x = Felt252::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = Felt252::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();

            x += y.clone();
            let as_uint = &x.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);

            // test reference variant
            x += &y;
            let as_uint = &x.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Checks that the result of adding two zeroes is zero
    fn sum_zeros_in_range() {
        let x = Felt252::new(0);
        let y = Felt252::new(0);
        let z = Felt252::new(0);
        assert_eq!(x + y, z)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Checks that the result of multiplying two zeroes is zero
    fn mul_zeros_in_range() {
        let x = Felt252::new(0);
        let y = Felt252::new(0);
        let z = Felt252::new(0);
        assert_eq!(x * y, z)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Checks that the result of performing a bit and operation between zeroes is zero
    fn bit_and_zeros_in_range() {
        let x = Felt252::new(0);
        let y = Felt252::new(0);
        let z = Felt252::new(0);
        assert_eq!(&x & &y, z)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Checks that the result of perfforming a bit or operation between zeroes is zero
    fn bit_or_zeros_in_range() {
        let x = Felt252::new(0);
        let y = Felt252::new(0);
        let z = Felt252::new(0);
        assert_eq!(&x | &y, z)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Checks that the result of perfforming a bit xor operation between zeroes is zero
    fn bit_xor_zeros_in_range() {
        let x = Felt252::new(0);
        let y = Felt252::new(0);
        let z = Felt252::new(0);
        assert_eq!(&x ^ &y, z)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Tests that the maximum value a Felt252 can take is equal to (prime - 1)
    fn upper_bound() {
        let prime = &BigUint::parse_bytes(PRIME_STR[2..].as_bytes(), 16).unwrap();
        let unit = BigUint::one();
        let felt_max_value = Felt252::max_value().to_biguint();
        assert_eq!(prime - unit, felt_max_value)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Tests that the minimum value a Felt252 can take is equal to zero.
    fn lower_bound() {
        let zero = BigUint::zero();
        let felt_min_value = Felt252::min_value().to_biguint();
        assert_eq!(zero, felt_min_value)
    }

    #[test]
    fn zero_value() {
        let zero = BigUint::zero();
        let felt_zero = Felt252::zero().to_biguint();
        assert_eq!(zero, felt_zero)
    }

    #[test]
    fn is_zero() {
        let felt_zero = Felt252::zero();
        let felt_non_zero = Felt252::new(3);
        assert!(felt_zero.is_zero());
        assert!(!felt_non_zero.is_zero())
    }

    #[test]
    fn one_value() {
        let one = BigUint::one();
        let felt_one = Felt252::one().to_biguint();
        assert_eq!(one, felt_one)
    }

    #[test]
    fn is_one() {
        let felt_one = Felt252::one();
        let felt_non_one = Felt252::new(8);
        assert!(felt_one.is_one());
        assert!(!felt_non_one.is_one())
    }

    #[test]
    fn signum_of_zero_is_zero() {
        let zero = Felt252::zero();
        assert_eq!(&zero.signum(), &zero)
    }
}
