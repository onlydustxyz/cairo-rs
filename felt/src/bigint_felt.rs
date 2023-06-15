#[cfg(all(not(feature = "std"), feature = "alloc"))]
use alloc::{string::String, vec::Vec};

use core::{
    cmp,
    convert::Into,
    fmt,
    iter::Sum,
    ops::{
        Add, AddAssign, BitAnd, BitOr, BitXor, Div, Mul, MulAssign, Neg, Rem, Shl, Shr, ShrAssign,
        Sub, SubAssign,
    },
};

use crate::{FeltOps, ParseFeltError};

pub const FIELD_HIGH: u128 = (1 << 123) + (17 << 64); // this is equal to 10633823966279327296825105735305134080
pub const FIELD_LOW: u128 = 1;
use lazy_static::lazy_static;
use num_bigint::{BigInt, BigUint, ToBigInt, U64Digits};
use num_integer::Integer;
use num_traits::{Bounded, FromPrimitive, Num, One, Pow, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};

lazy_static! {
    static ref CAIRO_PRIME_BIGUINT: BigUint =
        (Into::<BigUint>::into(FIELD_HIGH) << 128) + Into::<BigUint>::into(FIELD_LOW);
    pub static ref SIGNED_FELT_MAX: BigUint = (&*CAIRO_PRIME_BIGUINT).shr(1_u32);
    pub static ref CAIRO_SIGNED_PRIME: BigInt = CAIRO_PRIME_BIGUINT
        .to_bigint()
        .expect("Conversion BigUint -> BigInt can't fail");
}

#[derive(Eq, Hash, PartialEq, PartialOrd, Ord, Clone, Deserialize, Default, Serialize)]
pub(crate) struct FeltBigInt<const PH: u128, const PL: u128> {
    val: BigUint,
}

macro_rules! from_integer {
    ($type:ty) => {
        impl From<$type> for FeltBigInt<FIELD_HIGH, FIELD_LOW> {
            fn from(value: $type) -> Self {
                Self {
                    val: value
                        .try_into()
                        .unwrap_or_else(|_| &*CAIRO_PRIME_BIGUINT - (-value as u128)),
                }
            }
        }
    };
}

macro_rules! from_unsigned {
    ($type:ty) => {
        impl From<$type> for FeltBigInt<FIELD_HIGH, FIELD_LOW> {
            fn from(value: $type) -> Self {
                Self { val: value.into() }
            }
        }
    };
}

from_integer!(i8);
from_integer!(i16);
from_integer!(i32);
from_integer!(i64);
from_integer!(i128);
from_integer!(isize);

from_unsigned!(u8);
from_unsigned!(u16);
from_unsigned!(u32);
from_unsigned!(u64);
from_unsigned!(u128);
from_unsigned!(usize);

impl<const PH: u128, const PL: u128> From<BigUint> for FeltBigInt<PH, PL> {
    fn from(value: BigUint) -> Self {
        Self {
            val: match value {
                _ if value > *CAIRO_PRIME_BIGUINT => value.mod_floor(&CAIRO_PRIME_BIGUINT),
                _ if value == *CAIRO_PRIME_BIGUINT => BigUint::zero(),
                _ => value,
            },
        }
    }
}

impl<const PH: u128, const PL: u128> From<&BigUint> for FeltBigInt<PH, PL> {
    fn from(value: &BigUint) -> Self {
        Self {
            val: match value {
                _ if value > &*CAIRO_PRIME_BIGUINT => value.mod_floor(&CAIRO_PRIME_BIGUINT),
                _ if value == &*CAIRO_PRIME_BIGUINT => BigUint::zero(),
                _ => value.clone(),
            },
        }
    }
}

/* Code used to convert from BigUint to BigInt
   impl ToBigInt for BigUint {
       #[inline]
       fn to_bigint(&self) -> Option<BigInt> {
           if self.is_zero() {
               Some(Zero::zero())
           } else {
               Some(BigInt {
                   sign: Plus,
                   data: self.clone(),
               })
           }
       }
   }
*/

impl<const PH: u128, const PL: u128> From<BigInt> for FeltBigInt<PH, PL> {
    fn from(value: BigInt) -> Self {
        (&value).into()
    }
}

impl<const PH: u128, const PL: u128> From<&BigInt> for FeltBigInt<PH, PL> {
    fn from(value: &BigInt) -> Self {
        Self {
            val: value
                .mod_floor(&CAIRO_SIGNED_PRIME)
                .to_biguint()
                .expect("mod_floor is always positive"),
        }
    }
}

impl FeltOps for FeltBigInt<FIELD_HIGH, FIELD_LOW> {
    fn new<T: Into<FeltBigInt<FIELD_HIGH, FIELD_LOW>>>(
        value: T,
    ) -> FeltBigInt<FIELD_HIGH, FIELD_LOW> {
        value.into()
    }

    fn modpow(
        &self,
        exponent: &FeltBigInt<FIELD_HIGH, FIELD_LOW>,
        modulus: &FeltBigInt<FIELD_HIGH, FIELD_LOW>,
    ) -> FeltBigInt<FIELD_HIGH, FIELD_LOW> {
        FeltBigInt {
            val: self.val.modpow(&exponent.val, &modulus.val),
        }
    }

    fn iter_u64_digits(&self) -> U64Digits {
        self.val.iter_u64_digits()
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    fn to_signed_bytes_le(&self) -> Vec<u8> {
        self.val.to_bytes_le()
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    fn to_bytes_be(&self) -> Vec<u8> {
        self.val.to_bytes_be()
    }

    fn parse_bytes(buf: &[u8], radix: u32) -> Option<FeltBigInt<FIELD_HIGH, FIELD_LOW>> {
        match BigUint::parse_bytes(buf, radix) {
            Some(parsed) => Some(FeltBigInt::new(parsed)),
            None => BigInt::parse_bytes(buf, radix).map(FeltBigInt::new),
        }
    }

    fn from_bytes_be(bytes: &[u8]) -> FeltBigInt<FIELD_HIGH, FIELD_LOW> {
        let mut value = BigUint::from_bytes_be(bytes);
        if value >= *CAIRO_PRIME_BIGUINT {
            value = value.mod_floor(&CAIRO_PRIME_BIGUINT);
        }
        Self::from(value)
    }

    #[cfg(any(feature = "std", feature = "alloc"))]
    fn to_str_radix(&self, radix: u32) -> String {
        self.val.to_str_radix(radix)
    }

    fn to_bigint(&self) -> BigInt {
        if self.is_negative() {
            BigInt::from_biguint(num_bigint::Sign::Minus, &*CAIRO_PRIME_BIGUINT - &self.val)
        } else {
            self.val.clone().into()
        }
    }

    fn to_biguint(&self) -> BigUint {
        self.val.clone()
    }

    fn sqrt(&self) -> FeltBigInt<FIELD_HIGH, FIELD_LOW> {
        FeltBigInt {
            val: self.val.sqrt(),
        }
    }

    fn bits(&self) -> u64 {
        self.val.bits()
    }

    fn prime() -> BigUint {
        (Into::<BigUint>::into(FIELD_HIGH) << 128) + Into::<BigUint>::into(FIELD_LOW)
    }
}

impl<const PH: u128, const PL: u128> Add for FeltBigInt<PH, PL> {
    type Output = Self;
    fn add(mut self, rhs: Self) -> Self {
        self.val += rhs.val;
        if self.val >= *CAIRO_PRIME_BIGUINT {
            self.val -= &*CAIRO_PRIME_BIGUINT;
        }
        self
    }
}

impl<'a, const PH: u128, const PL: u128> Add for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;

    fn add(self, rhs: Self) -> Self::Output {
        let mut sum = &self.val + &rhs.val;
        if sum >= *CAIRO_PRIME_BIGUINT {
            sum -= &*CAIRO_PRIME_BIGUINT;
        }
        FeltBigInt { val: sum }
    }
}

impl<'a, const PH: u128, const PL: u128> Add<&'a FeltBigInt<PH, PL>> for FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;

    fn add(mut self, rhs: &'a FeltBigInt<PH, PL>) -> Self::Output {
        self.val += &rhs.val;
        if self.val >= *CAIRO_PRIME_BIGUINT {
            self.val -= &*CAIRO_PRIME_BIGUINT;
        }
        self
    }
}

impl<const PH: u128, const PL: u128> Add<u32> for FeltBigInt<PH, PL> {
    type Output = Self;
    fn add(mut self, rhs: u32) -> Self {
        self.val += rhs;
        if self.val >= *CAIRO_PRIME_BIGUINT {
            self.val -= &*CAIRO_PRIME_BIGUINT;
        }
        self
    }
}

impl<const PH: u128, const PL: u128> Add<usize> for FeltBigInt<PH, PL> {
    type Output = Self;
    fn add(mut self, rhs: usize) -> Self {
        self.val += rhs;
        if self.val >= *CAIRO_PRIME_BIGUINT {
            self.val -= &*CAIRO_PRIME_BIGUINT;
        }
        self
    }
}

impl<'a, const PH: u128, const PL: u128> Add<usize> for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn add(self, rhs: usize) -> Self::Output {
        let mut sum = &self.val + rhs;
        if sum >= *CAIRO_PRIME_BIGUINT {
            sum -= &*CAIRO_PRIME_BIGUINT;
        }
        FeltBigInt { val: sum }
    }
}

impl<const PH: u128, const PL: u128> AddAssign for FeltBigInt<PH, PL> {
    fn add_assign(&mut self, rhs: Self) {
        *self = &*self + &rhs;
    }
}

impl<'a, const PH: u128, const PL: u128> AddAssign<&'a FeltBigInt<PH, PL>> for FeltBigInt<PH, PL> {
    fn add_assign(&mut self, rhs: &'a FeltBigInt<PH, PL>) {
        *self = &*self + rhs;
    }
}

impl<const PH: u128, const PL: u128> Sum for FeltBigInt<PH, PL> {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        iter.fold(FeltBigInt::zero(), |mut acc, x| {
            acc += x;
            acc
        })
    }
}

impl<const PH: u128, const PL: u128> Neg for FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn neg(self) -> Self::Output {
        if self.is_zero() {
            self
        } else {
            FeltBigInt {
                val: &*CAIRO_PRIME_BIGUINT - self.val,
            }
        }
    }
}

impl<'a, const PH: u128, const PL: u128> Neg for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn neg(self) -> Self::Output {
        if self.is_zero() {
            self.clone()
        } else {
            FeltBigInt {
                val: &*CAIRO_PRIME_BIGUINT - &self.val,
            }
        }
    }
}

impl<const PH: u128, const PL: u128> Sub for FeltBigInt<PH, PL> {
    type Output = Self;
    fn sub(mut self, rhs: Self) -> Self::Output {
        if self.val < rhs.val {
            self.val += &*CAIRO_PRIME_BIGUINT;
        }
        self.val -= rhs.val;
        self
    }
}

impl<'a, const PH: u128, const PL: u128> Sub<&'a FeltBigInt<PH, PL>> for FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn sub(mut self, rhs: &'a FeltBigInt<PH, PL>) -> Self::Output {
        if self.val < rhs.val {
            self.val += &*CAIRO_PRIME_BIGUINT;
        }
        self.val -= &rhs.val;
        self
    }
}

impl<'a, const PH: u128, const PL: u128> Sub for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn sub(self, rhs: Self) -> Self::Output {
        FeltBigInt {
            val: if self.val < rhs.val {
                &*CAIRO_PRIME_BIGUINT - (&rhs.val - &self.val)
            } else {
                &self.val - &rhs.val
            },
        }
    }
}

impl<const PH: u128, const PL: u128> Sub<u32> for FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn sub(self, rhs: u32) -> Self {
        match (self.val).to_u32() {
            Some(num) if num < rhs => Self {
                val: &*CAIRO_PRIME_BIGUINT - (rhs - self.val),
            },
            _ => Self {
                val: self.val - rhs,
            },
        }
    }
}

impl<'a, const PH: u128, const PL: u128> Sub<u32> for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn sub(self, rhs: u32) -> Self::Output {
        match (self.val).to_u32() {
            Some(num) if num < rhs => FeltBigInt {
                val: &*CAIRO_PRIME_BIGUINT - (rhs - &self.val),
            },
            _ => FeltBigInt {
                val: &self.val - rhs,
            },
        }
    }
}

impl<const PH: u128, const PL: u128> Sub<usize> for FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn sub(self, rhs: usize) -> Self {
        match (self.val).to_usize() {
            Some(num) if num < rhs => FeltBigInt {
                val: &*CAIRO_PRIME_BIGUINT - (rhs - num),
            },
            _ => FeltBigInt {
                val: self.val - rhs,
            },
        }
    }
}

impl<'a, const PH: u128, const PL: u128> Pow<&'a FeltBigInt<PH, PL>> for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn pow(self, rhs: Self) -> Self::Output {
        FeltBigInt {
            val: self.val.modpow(&rhs.val, &CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<const PH: u128, const PL: u128> SubAssign for FeltBigInt<PH, PL> {
    fn sub_assign(&mut self, rhs: Self) {
        *self = &*self - &rhs;
    }
}

impl<'a, const PH: u128, const PL: u128> SubAssign<&'a FeltBigInt<PH, PL>> for FeltBigInt<PH, PL> {
    fn sub_assign(&mut self, rhs: &'a FeltBigInt<PH, PL>) {
        *self = &*self - rhs;
    }
}

impl Sub<FeltBigInt<FIELD_HIGH, FIELD_LOW>> for usize {
    type Output = FeltBigInt<FIELD_HIGH, FIELD_LOW>;
    fn sub(self, rhs: FeltBigInt<FIELD_HIGH, FIELD_LOW>) -> Self::Output {
        self - &rhs
    }
}

impl Sub<&FeltBigInt<FIELD_HIGH, FIELD_LOW>> for usize {
    type Output = FeltBigInt<FIELD_HIGH, FIELD_LOW>;
    fn sub(self, rhs: &FeltBigInt<FIELD_HIGH, FIELD_LOW>) -> Self::Output {
        match (rhs.val).to_usize() {
            Some(num) => {
                if num > self {
                    FeltBigInt {
                        val: &*CAIRO_PRIME_BIGUINT - (num - self),
                    }
                } else {
                    FeltBigInt::new(self - num)
                }
            }
            None => FeltBigInt {
                val: &*CAIRO_PRIME_BIGUINT - (&rhs.val - self),
            },
        }
    }
}

impl<const PH: u128, const PL: u128> Mul for FeltBigInt<PH, PL> {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self::Output {
        FeltBigInt {
            val: (self.val * rhs.val).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<'a, const PH: u128, const PL: u128> Mul for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn mul(self, rhs: Self) -> Self::Output {
        FeltBigInt {
            val: (&self.val * &rhs.val).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<'a, const PH: u128, const PL: u128> Mul<&'a FeltBigInt<PH, PL>> for FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn mul(self, rhs: &'a FeltBigInt<PH, PL>) -> Self::Output {
        FeltBigInt {
            val: (&self.val * &rhs.val).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<'a, const PH: u128, const PL: u128> MulAssign<&'a FeltBigInt<PH, PL>> for FeltBigInt<PH, PL> {
    fn mul_assign(&mut self, rhs: &'a FeltBigInt<PH, PL>) {
        *self = &*self * rhs;
    }
}

impl<const PH: u128, const PL: u128> Pow<u32> for FeltBigInt<PH, PL> {
    type Output = Self;
    fn pow(self, rhs: u32) -> Self {
        FeltBigInt {
            val: self.val.pow(rhs).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<'a, const PH: u128, const PL: u128> Pow<u32> for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    #[allow(clippy::needless_borrow)] // the borrow of self.val is necessary becase it's of the type BigUInt, which doesn't implement the Copy trait
    fn pow(self, rhs: u32) -> Self::Output {
        FeltBigInt {
            val: (&self.val).pow(rhs).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<const PH: u128, const PL: u128> Div for FeltBigInt<PH, PL> {
    type Output = Self;
    // In Felts `x / y` needs to be expressed as `x * y^-1`
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        if rhs.is_zero() {
            panic!("Can't divide Felt by zero")
        }
        let x = rhs
            .val
            .to_bigint() // Always succeeds for BigUint -> BigInt
            .unwrap()
            .extended_gcd(&CAIRO_SIGNED_PRIME)
            .x;
        self * &FeltBigInt::from(x)
    }
}

impl<'a, const PH: u128, const PL: u128> Div for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    // In Felts `x / y` needs to be expressed as `x * y^-1`
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: Self) -> Self::Output {
        if rhs.is_zero() {
            panic!("Can't divide Felt by zero")
        }
        let x = rhs
            .val
            .to_bigint() // Always succeeds for BitUint -> BigInt
            .unwrap()
            .extended_gcd(&CAIRO_SIGNED_PRIME)
            .x;
        self * &FeltBigInt::from(x)
    }
}

impl<'a, const PH: u128, const PL: u128> Div<FeltBigInt<PH, PL>> for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    // In Felts `x / y` needs to be expressed as `x * y^-1`
    #[allow(clippy::suspicious_arithmetic_impl)]
    fn div(self, rhs: FeltBigInt<PH, PL>) -> Self::Output {
        self / &rhs
    }
}

impl<const PH: u128, const PL: u128> Rem for FeltBigInt<PH, PL> {
    type Output = Self;
    fn rem(self, _rhs: Self) -> Self {
        FeltBigInt::zero()
    }
}

impl<'a, const PH: u128, const PL: u128> Rem<&'a FeltBigInt<PH, PL>> for FeltBigInt<PH, PL> {
    type Output = Self;
    fn rem(self, _rhs: &'a FeltBigInt<PH, PL>) -> Self::Output {
        FeltBigInt::zero()
    }
}

impl<const PH: u128, const PL: u128> Zero for FeltBigInt<PH, PL> {
    fn zero() -> Self {
        Self {
            val: BigUint::zero(),
        }
    }

    fn is_zero(&self) -> bool {
        self.val.is_zero()
    }
}

impl<const PH: u128, const PL: u128> One for FeltBigInt<PH, PL> {
    fn one() -> Self {
        Self {
            val: BigUint::one(),
        }
    }

    fn is_one(&self) -> bool
    where
        Self: PartialEq,
    {
        self.val.is_one()
    }
}

impl<const PH: u128, const PL: u128> Bounded for FeltBigInt<PH, PL> {
    fn min_value() -> Self {
        Self::zero()
    }
    fn max_value() -> Self {
        Self {
            val: &*CAIRO_PRIME_BIGUINT - 1_u32,
        }
    }
}

impl Num for FeltBigInt<FIELD_HIGH, FIELD_LOW> {
    type FromStrRadixErr = ParseFeltError;
    fn from_str_radix(string: &str, radix: u32) -> Result<Self, Self::FromStrRadixErr> {
        match BigUint::from_str_radix(string, radix) {
            Ok(num) => Ok(FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(num)),
            Err(_) => Err(ParseFeltError),
        }
    }
}

impl Integer for FeltBigInt<FIELD_HIGH, FIELD_LOW> {
    fn div_floor(&self, other: &Self) -> Self {
        FeltBigInt {
            val: &self.val / &other.val,
        }
    }

    fn div_rem(&self, other: &Self) -> (Self, Self) {
        let (d, m) = self.val.div_mod_floor(&other.val);
        (FeltBigInt { val: d }, FeltBigInt { val: m })
    }

    fn divides(&self, other: &Self) -> bool {
        self.is_multiple_of(other)
    }

    fn gcd(&self, other: &Self) -> Self {
        Self {
            val: self.val.gcd(&other.val),
        }
    }

    fn is_even(&self) -> bool {
        self.val.is_even()
    }

    fn is_multiple_of(&self, _other: &Self) -> bool {
        true
    }

    fn is_odd(&self) -> bool {
        self.val.is_odd()
    }

    fn lcm(&self, other: &Self) -> Self {
        Self::new(cmp::max(&self.val, &other.val))
    }

    fn mod_floor(&self, other: &Self) -> Self {
        Self {
            val: self.val.mod_floor(&other.val),
        }
    }
}

impl Signed for FeltBigInt<FIELD_HIGH, FIELD_LOW> {
    fn abs(&self) -> Self {
        if self.is_negative() {
            self.neg()
        } else {
            self.clone()
        }
    }

    fn abs_sub(&self, other: &Self) -> Self {
        if self > other {
            self - other
        } else {
            other - self
        }
    }

    fn signum(&self) -> Self {
        if self.is_zero() {
            FeltBigInt::zero()
        } else if self.is_positive() {
            FeltBigInt::one()
        } else {
            FeltBigInt::max_value()
        }
    }

    fn is_positive(&self) -> bool {
        !self.is_zero() && self.val < *SIGNED_FELT_MAX
    }

    fn is_negative(&self) -> bool {
        !(self.is_positive() || self.is_zero())
    }
}

impl<const PH: u128, const PL: u128> Shl<u32> for FeltBigInt<PH, PL> {
    type Output = Self;
    fn shl(self, other: u32) -> Self::Output {
        FeltBigInt {
            val: (self.val).shl(other).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<'a, const PH: u128, const PL: u128> Shl<u32> for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn shl(self, other: u32) -> Self::Output {
        FeltBigInt {
            val: (&self.val).shl(other).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<const PH: u128, const PL: u128> Shl<usize> for FeltBigInt<PH, PL> {
    type Output = Self;
    fn shl(self, other: usize) -> Self::Output {
        FeltBigInt {
            val: (self.val).shl(other).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<'a, const PH: u128, const PL: u128> Shl<usize> for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn shl(self, other: usize) -> Self::Output {
        FeltBigInt {
            val: (&self.val).shl(other).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<const PH: u128, const PL: u128> Shr<u32> for FeltBigInt<PH, PL> {
    type Output = Self;
    fn shr(self, other: u32) -> Self::Output {
        FeltBigInt {
            val: self.val.shr(other).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<'a, const PH: u128, const PL: u128> Shr<u32> for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn shr(self, other: u32) -> Self::Output {
        FeltBigInt {
            val: (&self.val).shr(other).mod_floor(&CAIRO_PRIME_BIGUINT),
        }
    }
}

impl<const PH: u128, const PL: u128> ShrAssign<usize> for FeltBigInt<PH, PL> {
    fn shr_assign(&mut self, other: usize) {
        self.val = (&self.val).shr(other).mod_floor(&CAIRO_PRIME_BIGUINT);
    }
}

impl<'a, const PH: u128, const PL: u128> BitAnd for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn bitand(self, rhs: Self) -> Self::Output {
        FeltBigInt {
            val: &self.val & &rhs.val,
        }
    }
}

impl<'a, const PH: u128, const PL: u128> BitAnd<&'a FeltBigInt<PH, PL>> for FeltBigInt<PH, PL> {
    type Output = Self;
    fn bitand(self, rhs: &'a FeltBigInt<PH, PL>) -> Self::Output {
        FeltBigInt {
            val: self.val & &rhs.val,
        }
    }
}

impl<'a, const PH: u128, const PL: u128> BitAnd<FeltBigInt<PH, PL>> for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn bitand(self, rhs: Self::Output) -> Self::Output {
        FeltBigInt {
            val: &self.val & rhs.val,
        }
    }
}

impl<'a, const PH: u128, const PL: u128> BitOr for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn bitor(self, rhs: Self) -> Self::Output {
        FeltBigInt {
            val: &self.val | &rhs.val,
        }
    }
}

impl<'a, const PH: u128, const PL: u128> BitXor for &'a FeltBigInt<PH, PL> {
    type Output = FeltBigInt<PH, PL>;
    fn bitxor(self, rhs: Self) -> Self::Output {
        FeltBigInt {
            val: &self.val ^ &rhs.val,
        }
    }
}

impl<const PH: u128, const PL: u128> ToPrimitive for FeltBigInt<PH, PL> {
    fn to_u128(&self) -> Option<u128> {
        self.val.to_u128()
    }

    fn to_u64(&self) -> Option<u64> {
        self.val.to_u64()
    }

    fn to_i64(&self) -> Option<i64> {
        self.val.to_i64()
    }

    fn to_usize(&self) -> Option<usize> {
        self.val.to_usize()
    }
}

impl<const PH: u128, const PL: u128> FromPrimitive for FeltBigInt<PH, PL> {
    fn from_u64(n: u64) -> Option<Self> {
        BigUint::from_u64(n).map(|n| Self { val: n })
    }

    fn from_i64(n: i64) -> Option<Self> {
        BigUint::from_i64(n).map(|n| Self { val: n })
    }

    fn from_usize(n: usize) -> Option<Self> {
        BigUint::from_usize(n).map(|n| Self { val: n })
    }
}

impl<const PH: u128, const PL: u128> fmt::Display for FeltBigInt<PH, PL> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.val)
    }
}

impl<const PH: u128, const PL: u128> fmt::Debug for FeltBigInt<PH, PL> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.val)
    }
}

impl fmt::Display for ParseFeltError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{ParseFeltError:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    use alloc::string::ToString;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    // Tests that the result of adding two zeros is zero.
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn add_zeros() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::new(0);
        let c = FeltBigInt::new(0);

        assert_eq!(a + b, c);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Tests that the result of performing add assign with two zeros is zero.
    fn add_assign_zeros() {
        let mut a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::new(0);
        a += b;
        let c = FeltBigInt::new(0);

        assert_eq!(a, c);
    }
    #[test]
    // Tests that the result of performing a bitwise "and" operation with two zeros is zero.
    fn bit_and_zeros() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::new(0);
        let c = FeltBigInt::new(0);

        assert_eq!(&a & &b, c);
    }
    #[test]
    // Tests that the result of performing a bitwise "or" operation with two zeros is zero.

    fn bit_or_zeros() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::new(0);
        let c = FeltBigInt::new(0);

        assert_eq!(&a | &b, c);
    }

    #[test]
    // Tests that the result of performing a bitwise "xor" operation with two zeros results in zero.
    fn bit_xor_zeros() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::new(0);
        let c = FeltBigInt::new(0);

        assert_eq!(&a ^ &b, c);
    }

    #[test]
    #[should_panic]
    // Tests that the result of performing a division by zero results in panic.
    fn div_zeros() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let _ = a / b;
    }

    #[test]
    #[should_panic]
    // Tests that the result of performing a division by zero results in panic.
    fn div_zeros_ref() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let _ = &a / &b;
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Tests that the result of multiplying two zeros is zero.
    fn mul_zeros() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::new(0);
        let c = FeltBigInt::new(0);

        assert_eq!(a * b, c);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Tests that the result of multiplying two zeros with assignment is zero.
    fn mul_assign_zeros() {
        let mut a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::new(0);
        a *= &b;
        let c = FeltBigInt::new(0);

        assert_eq!(a, c);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Tests that the result of subtracting two zeros is zero.
    fn sub_zeros() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::new(0);
        let c = FeltBigInt::new(0);

        assert_eq!(a - b, c);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Tests that the result of subtracting two zeros with assignment is zero.
    fn sub_assign_zeros() {
        let mut a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::new(0);
        a -= b;
        let c = FeltBigInt::new(0);

        assert_eq!(a, c);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn sub_usize_felt() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(4u32);
        let b = FeltBigInt::new(2u32);

        assert_eq!(6usize - &a, b);
        assert_eq!(6usize - a, b);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    // Tests that the negative of zero is zero
    fn negate_zero() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = a.neg();
        assert_eq!(
            b,
            FeltBigInt::from_str_radix("0", 10).expect("Couldn't parse int")
        );

        let c = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::from_str_radix("0", 10)
            .expect("Couldn't parse int");
        let d = c.neg();
        assert_eq!(d, FeltBigInt::new(0));
    }

    #[test]
    // Tests a shift left operation performed on a felt of value zero
    fn shift_left_zero() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let result = &a << 10_u32;
        assert_eq!(result, b)
    }

    #[test]
    // Tests a shift right operation performed on a felt of value zero
    fn shift_right_zero() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let result = &a >> 10_u32;
        assert_eq!(result, b)
    }

    #[test]
    // Tests a shift right operation with assignment performed on a felt of value zero
    fn shift_right_assign_zero() {
        let mut a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        a >>= 10;
        assert_eq!(a, b)
    }

    #[test]
    // Test that an iterative sum of zeros results in zero
    fn sum_zeros() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let c = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let v = vec![a, b, c];
        let result: FeltBigInt<FIELD_HIGH, FIELD_LOW> = v.into_iter().sum();
        assert_eq!(result, FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0))
    }

    #[test]
    // Tests that the remainder of a division where the dividend is 0, results in 0
    fn rem_zero() {
        let a = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let b = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        let c = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(10);
        let d = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::new(0);
        assert_eq!(a.clone() % b, d);
        assert_eq!(a % c, d)
    }

    proptest! {
        #[test]
        // Property-based test that ensures, for 100 pairs of values that are randomly generated each time tests are run, that performing a subtraction returns a result that is inside of the range [0, p].
        fn sub_bigint_felt_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            let result = x - y;
            let as_uint = &result.to_biguint();
            prop_assert!(as_uint < &p, "{}", as_uint);
        }

        #[test]
        // Property-based test that ensures, for 100 pairs of values that are randomly generated each time tests are run, that performing a subtraction returns a result that is inside of the range [0, p].
        fn sub_assign_bigint_felt_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let mut x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            x -= y;
            let as_uint = &x.to_biguint();
            prop_assert!(as_uint < &p, "{}", as_uint);
        }

        #[test]
        // Property-based test that ensures that the remainder of a division between two random bigint felts returns 0. The test is performed 100 times each run.
        fn rem_bigint_felt_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();

            let result = x % y;
            prop_assert!(result.is_zero());
        }
        // Tests that the result of adding two random large bigint felts falls within the range [0, p]. This test is performed 100 times each run.
        #[test]
        fn add_bigint_felts_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = &CAIRO_PRIME_BIGUINT;
            let result = x + y;
            let as_uint = &result.to_biguint();
            prop_assert!(as_uint < &p, "{}", as_uint);

        }
        #[test]
        // Tests that the result of performing add assign on two random large bigint felts falls within the range [0, p]. This test is performed 100 times each run.
        fn add_assign_bigint_felts_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let mut x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p = &CAIRO_PRIME_BIGUINT;
            x += y;
            let as_uint = &x.to_biguint();
            prop_assert!(as_uint < &p, "{}", as_uint);
        }

        #[test]
        // Tests that the result of performing the bitwise "and" operation on two random large bigint felts falls within the range [0, p]. This test is performed 100 times each run.
        fn bitand_bigint_felts_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            let result = &x & &y;
            let as_uint = result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }
        #[test]
        // Tests that the result of performing the bitwise "or" operation on two random large bigint felts falls within the range [0, p]. This test is performed 100 times each run.
        fn bitor_bigint_felts_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            let result = &x | &y;
            let as_uint = result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }
        #[test]
        // Tests that the result of performing the bitwise "xor" operation on two random large bigint felts falls within the range [0, p]. This test is performed 100 times each run.
        fn bitxor_bigint_felts_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            let result = &x ^ &y;
            let as_uint = result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }
        #[test]
        // Tests that the result dividing two random large bigint felts falls within the range [0, p]. This test is performed 100 times each run.
        fn div_bigint_felts_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            let result = &x / &y;
            let as_uint = result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }
        #[test]
        // Tests that the result multiplying two random large bigint felts falls within the range [0, p]. This test is performed 100 times each run.
        fn mul_bigint_felts_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            let result = &x * &y;
            let as_uint = result.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }
        #[test]
        // Tests that the result of performing a multiplication with assignment between two random large bigint felts falls within the range [0, p]. This test is performed 100 times each run.
        fn mul_assign_bigint_felts_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)") {
            let mut x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            x *= &y;
            let as_uint = x.to_biguint();
            prop_assert!(as_uint < p, "{}", as_uint);
        }
        #[test]
        // Tests that the result of applying the negative operation to a large bigint felt falls within the range [0, p]. This test is performed 100 times each run.
        fn neg_bigint_felt_within_field(ref x in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            let result = -x;
            let as_uint = &result.to_biguint();
            prop_assert!(as_uint < &p, "{}", as_uint);
        }

        #[test]
         // Property-based test that ensures, for 100 {value}s that are randomly generated each time tests are run, that performing a bit shift to the left by an amount {y} of bits (between 0 and 999) returns a result that is inside of the range [0, p].
         fn shift_left_bigint_felt_within_field(ref x in "([1-9][0-9]*)", ref y in "[0-9]{1,3}") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = y.parse::<u32>().unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            let result = x << y;
            let as_uint = &result.to_biguint();
            prop_assert!(as_uint < &p, "{}", as_uint);
        }

        #[test]
        // Property-based test that ensures, for 100 {value}s that are randomly generated each time tests are run, that performing a bit shift to the right by an amount {y} of bits (between 0 and 999) returns a result that is inside of the range [0, p].
        fn shift_right_bigint_felt_within_field(ref x in "([1-9][0-9]*)", ref y in "[0-9]{1,3}") {
           let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
           let y = y.parse::<u32>().unwrap();
           let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
           let result = x >> y;
           let as_uint = &result.to_biguint();
           prop_assert!(as_uint < &p, "{}", as_uint);
       }

       #[test]
       // Property-based test that ensures, for 100 {value}s that are randomly generated each time tests are run, that performing a bit shift to the right with assignment by an amount {y} of bits (between 0 and 999) returns a result that is inside of the range [0, p].
       fn shift_right_assign_bigint_felt_within_field(ref x in "([1-9][0-9]*)", ref y in "[0-9]{1,3}") {
          let mut x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
          let y = y.parse::<u32>().unwrap();
          let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
          x >>= y.try_into().unwrap();
          let as_uint = &x.to_biguint();
          prop_assert!(as_uint < &p, "{}", as_uint);
        }

        #[test]
        // Property-based test that ensures, vectors of three of values that are randomly generated each time tests are run, that performing an iterative sum returns a result that is inside of the range [0, p]. The test is performed 100 times each run.
        fn sum_bigint_felt_within_field(ref x in "([1-9][0-9]*)", ref y in "([1-9][0-9]*)", ref z in "([1-9][0-9]*)") {
            let x = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(x.as_bytes(), 10).unwrap();
            let y = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(y.as_bytes(), 10).unwrap();
            let z = FeltBigInt::<FIELD_HIGH, FIELD_LOW>::parse_bytes(z.as_bytes(), 10).unwrap();
            let p:BigUint = BigUint::parse_bytes(CAIRO_PRIME_BIGUINT.to_string().as_bytes(), 16).unwrap();
            let v = vec![x.clone(), y, z];
            let result: FeltBigInt<FIELD_HIGH, FIELD_LOW> = v.into_iter().sum();
            let as_uint = result.to_biguint();
            prop_assert!(&as_uint < &p, "{}", as_uint);
        }
    }
}
