use crate::stdlib::{cell::RefCell, cmp::min, ops::Shr, rc::Rc, vec::Vec};
use crate::types::errors::math_errors::MathError;
use felt::Felt252;
use num_bigint::{BigInt, BigUint, RandBigInt};
use num_integer::Integer;
use num_prime::nt_funcs::is_prime64;
use num_prime::PrimalityUtils;
use num_prime::{buffer::NaiveBuffer, Primality, PrimalityTestConfig, PrimeBuffer};
use num_traits::{Bounded, One, Pow, Signed, Zero};
use num_traits::{FromPrimitive, ToPrimitive};
use rand::{rngs::SmallRng, RngCore, SeedableRng};

#[cfg(not(feature = "std"))]
use num_traits::float::FloatCore;

///Returns the integer square root of the nonnegative integer n.
///This is the floor of the exact square root of n.
///Unlike math.sqrt(), this function doesn't have rounding error issues.
pub fn isqrt(n: &BigUint) -> Result<BigUint, MathError> {
    /*    # The following algorithm was copied from
    # https://stackoverflow.com/questions/15390807/integer-square-root-in-python.
    x = n
    y = (x + 1) // 2
    while y < x:
        x = y
        y = (x + n // x) // 2
    assert x**2 <= n < (x + 1) ** 2
    return x*/

    let mut x = n.clone();
    //n.shr(1) = n.div_floor(2)
    let mut y = (&x + 1_u32).shr(1_u32);

    while y < x {
        x = y;
        y = (&x + n.div_floor(&x)).shr(1_u32);
    }

    if !(&BigUint::pow(&x, 2_u32) <= n && n < &BigUint::pow(&(&x + 1_u32), 2_u32)) {
        return Err(MathError::FailedToGetSqrt(n.clone()));
    };
    Ok(x)
}

/// Performs integer division between x and y; fails if x is not divisible by y.
pub fn safe_div(x: &Felt252, y: &Felt252) -> Result<Felt252, MathError> {
    if y.is_zero() {
        return Err(MathError::DividedByZero);
    }

    let (q, r) = x.div_mod_floor(y);

    if !r.is_zero() {
        return Err(MathError::SafeDivFail(x.clone(), y.clone()));
    }

    Ok(q)
}

/// Performs integer division between x and y; fails if x is not divisible by y.
pub fn safe_div_bigint(x: &BigInt, y: &BigInt) -> Result<BigInt, MathError> {
    if y.is_zero() {
        return Err(MathError::DividedByZero);
    }

    let (q, r) = x.div_mod_floor(y);

    if !r.is_zero() {
        return Err(MathError::SafeDivFailBigInt(x.clone(), y.clone()));
    }

    Ok(q)
}

/// Performs integer division between x and y; fails if x is not divisible by y.
pub fn safe_div_usize(x: usize, y: usize) -> Result<usize, MathError> {
    if y.is_zero() {
        return Err(MathError::DividedByZero);
    }

    let (q, r) = x.div_mod_floor(&y);

    if !r.is_zero() {
        return Err(MathError::SafeDivFailUsize(x, y));
    }

    Ok(q)
}

///Returns num_a^-1 mod p
pub(crate) fn mul_inv(num_a: &BigInt, p: &BigInt) -> BigInt {
    if num_a.is_zero() {
        return BigInt::zero();
    }
    let mut a = num_a.abs();
    let x_sign = num_a.signum();
    let mut b = p.abs();
    let (mut x, mut r) = (BigInt::one(), BigInt::zero());
    let (mut c, mut q);
    while !b.is_zero() {
        (q, c) = a.div_mod_floor(&b);
        x -= &q * &r;
        (a, b, r, x) = (b, c, x, r)
    }

    x * x_sign
}

///Finds a nonnegative integer x < p such that (m * x) % p == n.
pub fn div_mod(n: &BigInt, m: &BigInt, p: &BigInt) -> BigInt {
    let a = mul_inv(m, p);
    (n * a).mod_floor(p)
}

pub fn ec_add(
    point_a: (BigInt, BigInt),
    point_b: (BigInt, BigInt),
    prime: &BigInt,
) -> (BigInt, BigInt) {
    let m = line_slope(&point_a, &point_b, prime);
    let x = (m.clone() * m.clone() - point_a.0.clone() - point_b.0).mod_floor(prime);
    let y = (m * (point_a.0 - x.clone()) - point_a.1).mod_floor(prime);
    (x, y)
}

/// Computes the slope of the line connecting the two given EC points over the field GF(p).
/// Assumes the points are given in affine form (x, y) and have different x coordinates.
pub fn line_slope(
    point_a: &(BigInt, BigInt),
    point_b: &(BigInt, BigInt),
    prime: &BigInt,
) -> BigInt {
    debug_assert!(!(&point_a.0 - &point_b.0).is_multiple_of(prime));
    div_mod(
        &(&point_a.1 - &point_b.1),
        &(&point_a.0 - &point_b.0),
        prime,
    )
}

///  Doubles a point on an elliptic curve with the equation y^2 = x^3 + alpha*x + beta mod p.
/// Assumes the point is given in affine form (x, y) and has y != 0.
pub fn ec_double(point: (BigInt, BigInt), alpha: &BigInt, prime: &BigInt) -> (BigInt, BigInt) {
    let m = ec_double_slope(&point, alpha, prime);
    let x = ((&m * &m) - (2_i32 * &point.0)).mod_floor(prime);
    let y = (m * (point.0 - &x) - point.1).mod_floor(prime);
    (x, y)
}
/// Computes the slope of an elliptic curve with the equation y^2 = x^3 + alpha*x + beta mod p, at
/// the given point.
/// Assumes the point is given in affine form (x, y) and has y != 0.
pub fn ec_double_slope(point: &(BigInt, BigInt), alpha: &BigInt, prime: &BigInt) -> BigInt {
    debug_assert!(!point.1.is_multiple_of(prime));
    div_mod(
        &(3_i32 * &point.0 * &point.0 + alpha),
        &(2_i32 * &point.1),
        prime,
    )
}

pub fn sqrt(n: &Felt252) -> Felt252 {
    // Based on Tonelli-Shanks' algorithm for finding square roots
    // and sympy's library implementation of said algorithm.
    if n.is_zero() || n.is_one() {
        return n.clone();
    }

    let max_felt = Felt252::max_value();
    let trailing_prime = Felt252::max_value() >> 192; // 0x800000000000011
    let a = n.pow(&trailing_prime);
    let d = (&Felt252::new(3_i32)).pow(&trailing_prime);
    let mut m = Felt252::zero();
    let mut exponent = Felt252::one() << 191_u32;
    let mut adm;
    for i in 0..192_u32 {
        adm = &a * &(&d).pow(&m);
        adm = (&adm).pow(&exponent);
        exponent >>= 1;
        // if adm ≡ -1 (mod CAIRO_PRIME)
        if adm == max_felt {
            m += Felt252::one() << i;
        }
    }
    let root_1 = n.pow(&((trailing_prime + 1_u32) >> 1)) * (&d).pow(&(m >> 1));
    let root_2 = &max_felt - &root_1 + 1_usize;
    if root_1 < root_2 {
        root_1
    } else {
        root_2
    }
}

trait MyPrime {
    fn my_is_prime(&self, n: &BigUint, rng: Rc<RefCell<dyn RngCore>>) -> Primality;
}

impl MyPrime for NaiveBuffer {
    fn my_is_prime(&self, target: &BigUint, rng: Rc<RefCell<dyn RngCore>>) -> Primality {
        // shortcuts
        if target.is_even() {
            return if target == &BigUint::from_u8(2u8).unwrap() {
                Primality::Yes
            } else {
                Primality::No
            };
        }

        // do deterministic test if target is under 2^64
        if let Some(x) = target.to_u64() {
            return match is_prime64(x) {
                true => Primality::Yes,
                false => Primality::No,
            };
        }

        let config = PrimalityTestConfig::default();
        let mut probability = 1.;

        // miller-rabin test
        let mut witness_list: Vec<u64> = Vec::new();
        if config.sprp_trials > 0 {
            witness_list.extend(self.iter().take(config.sprp_trials));
            probability *= 1. - 0.25f32.powi(config.sprp_trials as i32);
        }
        if config.sprp_random_trials > 0 {
            let mut key = [0u8; 16];
            rng.borrow_mut().fill_bytes(&mut key);
            for _ in 0..config.sprp_random_trials {
                // we have ensured target is larger than 2^64
                let mut w: u64 = rng.borrow_mut().next_u64();
                while witness_list.iter().find(|&x| x == &w).is_some() {
                    w = rng.borrow_mut().next_u64();
                }
                witness_list.push(w);
            }
            probability *= 1. - 0.25f32.powi(config.sprp_random_trials as i32);
        }
        if !witness_list
            .into_iter()
            .all(|x| target.is_sprp(BigUint::from_u64(x).unwrap()))
        {
            return Primality::No;
        }

        // lucas probable prime test
        if config.slprp_test {
            probability *= 1. - 4f32 / 15f32;
            if !target.is_slprp(None, None) {
                return Primality::No;
            }
        }
        if config.eslprp_test {
            probability *= 1. - 4f32 / 15f32;
            if !target.is_eslprp(None) {
                return Primality::No;
            }
        }

        Primality::Probable(probability)
    }
}

// Adapted from sympy _sqrt_prime_power with k == 1
pub fn sqrt_prime_power(
    a: &BigUint,
    p: &BigUint,
    rng: Rc<RefCell<dyn RngCore>>,
) -> Option<BigUint> {
    let buffer = NaiveBuffer::new();

    if p.is_zero() || !buffer.my_is_prime(p, rng).probably() {
        return None;
    }
    let two = BigUint::from(2_u32);
    let a = a.mod_floor(p);
    if p == &two {
        return Some(a);
    }
    if !(a < two || (a.modpow(&(p - 1_u32).div_floor(&two), p)).is_one()) {
        return None;
    };

    if p.mod_floor(&BigUint::from(4_u32)) == 3_u32.into() {
        let res = a.modpow(&(p + 1_u32).div_floor(&BigUint::from(4_u32)), p);
        return Some(min(res.clone(), p - res));
    };

    if p.mod_floor(&BigUint::from(8_u32)) == 5_u32.into() {
        let sign = a.modpow(&(p - 1_u32).div_floor(&BigUint::from(4_u32)), p);
        if sign.is_one() {
            let res = a.modpow(&(p + 3_u32).div_floor(&BigUint::from(8_u32)), p);
            return Some(min(res.clone(), p - res));
        } else {
            let b = (4_u32 * &a).modpow(&(p - 5_u32).div_floor(&BigUint::from(8_u32)), p);
            let x = (2_u32 * &a * b).mod_floor(p);
            if x.modpow(&two, p) == a {
                return Some(x);
            }
        }
    };

    Some(sqrt_tonelli_shanks(&a, p))
}

fn sqrt_tonelli_shanks(n: &BigUint, prime: &BigUint) -> BigUint {
    // Based on Tonelli-Shanks' algorithm for finding square roots
    // and sympy's library implementation of said algorithm.
    if n.is_zero() || n.is_one() {
        return n.clone();
    }
    let s = (prime - 1_u32).trailing_zeros().unwrap_or_default();
    let t = prime >> s;
    let a = n.modpow(&t, prime);
    // Rng is not critical here so its safe to use a seeded value
    let mut rng = SmallRng::seed_from_u64(11480028852697973135);
    let mut d;
    loop {
        d = RandBigInt::gen_biguint_range(&mut rng, &BigUint::from(2_u32), &(prime - 1_u32));
        let r = legendre_symbol(&d, prime);
        if r == -1 {
            break;
        };
    }
    d = d.modpow(&t, prime);
    let mut m = BigUint::zero();
    let mut exponent = BigUint::one() << (s - 1);
    let mut adm;
    for i in 0..s as u32 {
        adm = &a * &d.modpow(&m, prime);
        adm = adm.modpow(&exponent, prime);
        exponent >>= 1;
        if adm == (prime - 1_u32) {
            m += BigUint::from(1_u32) << i;
        }
    }
    let root_1 =
        (n.modpow(&((t + 1_u32) >> 1), prime) * d.modpow(&(m >> 1), prime)).mod_floor(prime);
    let root_2 = prime - &root_1;
    if root_1 < root_2 {
        root_1
    } else {
        root_2
    }
}

/* Disclaimer: Some asumptions have been taken based on the functions that rely on this function, make sure these are true before calling this function individually
Adpted from sympy implementation, asuming:
    - p is an odd prime number
    - a.mod_floor(p) == a
Returns the Legendre symbol `(a / p)`.

    For an integer ``a`` and an odd prime ``p``, the Legendre symbol is
    defined as

    .. math ::
        \genfrac(){}{}{a}{p} = \begin{cases}
             0 & \text{if } p \text{ divides } a\\
             1 & \text{if } a \text{ is a quadratic residue modulo } p\\
            -1 & \text{if } a \text{ is a quadratic nonresidue modulo } p
        \end{cases}
*/
fn legendre_symbol(a: &BigUint, p: &BigUint) -> i8 {
    if a.is_zero() {
        return 0;
    };
    if is_quad_residue(a, p).unwrap_or_default() {
        1
    } else {
        -1
    }
}

// Ported from sympy implementation
// Simplified as a & p are nonnegative
// Asumes p is a prime number
pub(crate) fn is_quad_residue(a: &BigUint, p: &BigUint) -> Result<bool, MathError> {
    if p.is_zero() {
        return Err(MathError::IsQuadResidueZeroPrime);
    }
    let a = if a >= p { a.mod_floor(p) } else { a.clone() };
    if a < BigUint::from(2_u8) || p < &BigUint::from(3_u8) {
        return Ok(true);
    }
    Ok(
        a.modpow(&(p - BigUint::one()).div_floor(&BigUint::from(2_u8)), p)
            .is_one(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::test_utils::*;
    use crate::utils::CAIRO_PRIME;
    use assert_matches::assert_matches;

    use num_traits::Num;

    #[cfg(not(target_arch = "wasm32"))]
    use proptest::prelude::*;

    // Only used in proptest for now
    #[cfg(not(target_arch = "wasm32"))]
    use num_bigint::Sign;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_divmod_a() {
        let a = bigint_str!(
            "11260647941622813594563746375280766662237311019551239924981511729608487775604310196863705127454617186486639011517352066501847110680463498585797912894788"
        );
        let b = bigint_str!(
            "4020711254448367604954374443741161860304516084891705811279711044808359405970"
        );
        assert_eq!(
            bigint_str!(
                "2904750555256547440469454488220756360634457312540595732507835416669695939476"
            ),
            div_mod(
                &a,
                &b,
                &BigInt::from_str_radix(&felt::PRIME_STR[2..], 16).expect("Couldn't parse prime")
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_divmod_b() {
        let a = bigint_str!(
            "29642372811668969595956851264770043260610851505766181624574941701711520154703788233010819515917136995474951116158286220089597404329949295479559895970988"
        );
        let b = bigint_str!(
            "3443173965374276972000139705137775968422921151703548011275075734291405722262"
        );
        assert_eq!(
            bigint_str!(
                "3601388548860259779932034493250169083811722919049731683411013070523752439691"
            ),
            div_mod(
                &a,
                &b,
                &BigInt::from_str_radix(&felt::PRIME_STR[2..], 16).expect("Couldn't parse prime")
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_divmod_c() {
        let a = bigint_str!(
            "1208267356464811040667664150251401430616174694388968865551115897173431833224432165394286799069453655049199580362994484548890574931604445970825506916876"
        );
        let b = bigint_str!(
            "1809792356889571967986805709823554331258072667897598829955472663737669990418"
        );
        assert_eq!(
            bigint_str!(
                "1545825591488572374291664030703937603499513742109806697511239542787093258962"
            ),
            div_mod(
                &a,
                &b,
                &BigInt::from_str_radix(&felt::PRIME_STR[2..], 16).expect("Couldn't parse prime")
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div() {
        let x = Felt252::new(26);
        let y = Felt252::new(13);
        assert_matches!(safe_div(&x, &y), Ok(i) if i == Felt252::new(2));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_non_divisor() {
        let x = Felt252::new(25);
        let y = Felt252::new(4);
        let result = safe_div(&x, &y);
        assert_matches!(
            result,
            Err(MathError::SafeDivFail(
                i, j
            )) if i == Felt252::new(25) && j == Felt252::new(4));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_by_zero() {
        let x = Felt252::new(25);
        let y = Felt252::zero();
        let result = safe_div(&x, &y);
        assert_matches!(result, Err(MathError::DividedByZero));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_usize() {
        assert_matches!(safe_div_usize(26, 13), Ok(2));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_usize_non_divisor() {
        assert_matches!(
            safe_div_usize(25, 4),
            Err(MathError::SafeDivFailUsize(25, 4))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_safe_div_usize_by_zero() {
        assert_matches!(safe_div_usize(25, 0), Err(MathError::DividedByZero));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_line_slope_for_valid_points() {
        let point_a = (
            bigint_str!(
                "3139037544796708144595053687182055617920475701120786241351436619796497072089"
            ),
            bigint_str!(
                "2119589567875935397690285099786081818522144748339117565577200220779667999801"
            ),
        );
        let point_b = (
            bigint_str!(
                "3324833730090626974525872402899302150520188025637965566623476530814354734325"
            ),
            bigint_str!(
                "3147007486456030910661996439995670279305852583596209647900952752170983517249"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        assert_eq!(
            bigint_str!(
                "992545364708437554384321881954558327331693627531977596999212637460266617010"
            ),
            line_slope(&point_a, &point_b, &prime)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_double_slope_for_valid_point_a() {
        let point = (
            bigint_str!(
                "3143372541908290873737380228370996772020829254218248561772745122290262847573"
            ),
            bigint_str!(
                "1721586982687138486000069852568887984211460575851774005637537867145702861131"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            bigint_str!(
                "3601388548860259779932034493250169083811722919049731683411013070523752439691"
            ),
            ec_double_slope(&point, &alpha, &prime)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn compute_double_slope_for_valid_point_b() {
        let point = (
            bigint_str!(
                "1937407885261715145522756206040455121546447384489085099828343908348117672673"
            ),
            bigint_str!(
                "2010355627224183802477187221870580930152258042445852905639855522404179702985"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            bigint_str!(
                "2904750555256547440469454488220756360634457312540595732507835416669695939476"
            ),
            ec_double_slope(&point, &alpha, &prime)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_double_for_valid_point_a() {
        let point = (
            bigint_str!(
                "1937407885261715145522756206040455121546447384489085099828343908348117672673"
            ),
            bigint_str!(
                "2010355627224183802477187221870580930152258042445852905639855522404179702985"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            (
                bigint_str!(
                    "58460926014232092148191979591712815229424797874927791614218178721848875644"
                ),
                bigint_str!(
                    "1065613861227134732854284722490492186040898336012372352512913425790457998694"
                )
            ),
            ec_double(point, &alpha, &prime)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_double_for_valid_point_b() {
        let point = (
            bigint_str!(
                "3143372541908290873737380228370996772020829254218248561772745122290262847573"
            ),
            bigint_str!(
                "1721586982687138486000069852568887984211460575851774005637537867145702861131"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            (
                bigint_str!(
                    "1937407885261715145522756206040455121546447384489085099828343908348117672673"
                ),
                bigint_str!(
                    "2010355627224183802477187221870580930152258042445852905639855522404179702985"
                )
            ),
            ec_double(point, &alpha, &prime)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_double_for_valid_point_c() {
        let point = (
            bigint_str!(
                "634630432210960355305430036410971013200846091773294855689580772209984122075"
            ),
            bigint_str!(
                "904896178444785983993402854911777165629036333948799414977736331868834995209"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        let alpha = bigint!(1);
        assert_eq!(
            (
                bigint_str!(
                    "3143372541908290873737380228370996772020829254218248561772745122290262847573"
                ),
                bigint_str!(
                    "1721586982687138486000069852568887984211460575851774005637537867145702861131"
                )
            ),
            ec_double(point, &alpha, &prime)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_add_for_valid_points_a() {
        let point_a = (
            bigint_str!(
                "1183418161532233795704555250127335895546712857142554564893196731153957537489"
            ),
            bigint_str!(
                "1938007580204102038458825306058547644691739966277761828724036384003180924526"
            ),
        );
        let point_b = (
            bigint_str!(
                "1977703130303461992863803129734853218488251484396280000763960303272760326570"
            ),
            bigint_str!(
                "2565191853811572867032277464238286011368568368717965689023024980325333517459"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        assert_eq!(
            (
                bigint_str!(
                    "1977874238339000383330315148209250828062304908491266318460063803060754089297"
                ),
                bigint_str!(
                    "2969386888251099938335087541720168257053975603483053253007176033556822156706"
                )
            ),
            ec_add(point_a, point_b, &prime)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_add_for_valid_points_b() {
        let point_a = (
            bigint_str!(
                "3139037544796708144595053687182055617920475701120786241351436619796497072089"
            ),
            bigint_str!(
                "2119589567875935397690285099786081818522144748339117565577200220779667999801"
            ),
        );
        let point_b = (
            bigint_str!(
                "3324833730090626974525872402899302150520188025637965566623476530814354734325"
            ),
            bigint_str!(
                "3147007486456030910661996439995670279305852583596209647900952752170983517249"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        assert_eq!(
            (
                bigint_str!(
                    "1183418161532233795704555250127335895546712857142554564893196731153957537489"
                ),
                bigint_str!(
                    "1938007580204102038458825306058547644691739966277761828724036384003180924526"
                )
            ),
            ec_add(point_a, point_b, &prime)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_ec_add_for_valid_points_c() {
        let point_a = (
            bigint_str!(
                "1183418161532233795704555250127335895546712857142554564893196731153957537489"
            ),
            bigint_str!(
                "1938007580204102038458825306058547644691739966277761828724036384003180924526"
            ),
        );
        let point_b = (
            bigint_str!(
                "1977703130303461992863803129734853218488251484396280000763960303272760326570"
            ),
            bigint_str!(
                "2565191853811572867032277464238286011368568368717965689023024980325333517459"
            ),
        );
        let prime = (*CAIRO_PRIME).clone().into();
        assert_eq!(
            (
                bigint_str!(
                    "1977874238339000383330315148209250828062304908491266318460063803060754089297"
                ),
                bigint_str!(
                    "2969386888251099938335087541720168257053975603483053253007176033556822156706"
                )
            ),
            ec_add(point_a, point_b, &prime)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_isqrt_a() {
        let n = biguint!(81);
        assert_matches!(isqrt(&n), Ok(x) if x == biguint!(9));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_isqrt_b() {
        let n = biguint_str!("4573659632505831259480");
        assert_matches!(isqrt(&BigUint::pow(&n, 2_u32)), Ok(num) if num == n);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_isqrt_c() {
        let n = biguint_str!(
            "3618502788666131213697322783095070105623107215331596699973092056135872020481"
        );
        assert_matches!(isqrt(&BigUint::pow(&n, 2_u32)), Ok(inner) if inner == n);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn calculate_isqrt_zero() {
        let n = BigUint::zero();
        assert_matches!(isqrt(&n), Ok(inner) if inner.is_zero());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn safe_div_bigint_by_zero() {
        let x = BigInt::one();
        let y = BigInt::zero();
        assert_matches!(safe_div_bigint(&x, &y), Err(MathError::DividedByZero))
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt() {
        let n = Felt252::from_str_radix(
            "99957092485221722822822221624080199277265330641980989815386842231144616633668",
            10,
        )
        .unwrap();
        let expected_sqrt = Felt252::from_str_radix(
            "205857351767627712295703269674687767888261140702556021834663354704341414042",
            10,
        )
        .unwrap();
        assert_eq!(sqrt(&n), expected_sqrt);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power() {
        let n: BigUint = 25_u32.into();
        let p: BigUint = 18446744069414584321_u128.into();
        let rng = Rc::new(RefCell::new(SmallRng::from_seed([42; 32])));
        assert_eq!(sqrt_prime_power(&n, &p, rng), Some(5_u32.into()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_p_is_zero() {
        let n = BigUint::one();
        let p: BigUint = BigUint::zero();
        let rng = Rc::new(RefCell::new(SmallRng::from_seed([42; 32])));
        assert_eq!(sqrt_prime_power(&n, &p, rng), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_non_prime() {
        let p: BigUint = BigUint::from_bytes_be(&[
            69, 15, 232, 82, 215, 167, 38, 143, 173, 94, 133, 111, 1, 2, 182, 229, 110, 113, 76, 0,
            47, 110, 148, 109, 6, 133, 27, 190, 158, 197, 168, 219, 165, 254, 81, 53, 25, 34,
        ]);
        let n = BigUint::from_bytes_be(&[
            9, 13, 22, 191, 87, 62, 157, 83, 157, 85, 93, 105, 230, 187, 32, 101, 51, 181, 49, 202,
            203, 195, 76, 193, 149, 78, 109, 146, 240, 126, 182, 115, 161, 238, 30, 118, 157, 252,
        ]);

        let rng = Rc::new(RefCell::new(SmallRng::from_seed([42; 32])));
        assert_eq!(sqrt_prime_power(&n, &p, rng), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_none() {
        let n: BigUint = 10_u32.into();
        let p: BigUint = 602_u32.into();
        let rng = Rc::new(RefCell::new(SmallRng::from_seed([42; 32])));
        assert_eq!(sqrt_prime_power(&n, &p, rng), None);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_prime_two() {
        let n: BigUint = 25_u32.into();
        let p: BigUint = 2_u32.into();
        let rng = Rc::new(RefCell::new(SmallRng::from_seed([42; 32])));
        assert_eq!(sqrt_prime_power(&n, &p, rng), Some(BigUint::one()));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_prime_mod_8_is_5_sign_not_one() {
        let n: BigUint = 676_u32.into();
        let p: BigUint = 9956234341095173_u64.into();
        let rng = Rc::new(RefCell::new(SmallRng::from_seed([42; 32])));
        assert_eq!(
            sqrt_prime_power(&n, &p, rng),
            Some(BigUint::from(9956234341095147_u64))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_sqrt_prime_power_prime_mod_8_is_5_sign_is_one() {
        let n: BigUint = 130283432663_u64.into();
        let p: BigUint = 743900351477_u64.into();
        let rng = Rc::new(RefCell::new(SmallRng::from_seed([42; 32])));
        assert_eq!(
            sqrt_prime_power(&n, &p, rng),
            Some(BigUint::from(123538694848_u64))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_legendre_symbol_zero() {
        assert!(legendre_symbol(&BigUint::zero(), &BigUint::one()).is_zero())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_is_quad_residue_prime_zero() {
        assert_eq!(
            is_quad_residue(&BigUint::one(), &BigUint::zero()),
            Err(MathError::IsQuadResidueZeroPrime)
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_is_quad_residue_prime_a_one_true() {
        assert_eq!(is_quad_residue(&BigUint::one(), &BigUint::one()), Ok(true))
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn mul_inv_0_is_0() {
        let p = &(*CAIRO_PRIME).clone().into();
        let x = &BigInt::zero();
        let x_inv = mul_inv(x, p);

        assert_eq!(x_inv, BigInt::zero());
    }

    #[cfg(not(target_arch = "wasm32"))]
    proptest! {
        #[test]
        // Test for sqrt of a quadratic residue. Result should be the minimum root.
        fn sqrt_felt_test(ref x in any::<[u8; 32]>()) {
            let x = &Felt252::from_bytes_be(x);
            let x_sq = x * x;
            let sqrt = sqrt(&x_sq);

            if &sqrt != x {
                prop_assert_eq!(&(Felt252::max_value() - sqrt + 1_usize), x);
            } else {
                prop_assert_eq!(&sqrt, x);
            }
        }

        #[test]
        // Test for sqrt_prime_power_ of a quadratic residue. Result should be the minimum root.
        fn sqrt_prime_power_using_random_prime(ref x in any::<[u8; 38]>(), ref y in any::<u64>()) {
            let mut rng = SmallRng::seed_from_u64(*y);
            let x = &BigUint::from_bytes_be(x);
            // Generate a prime here instead of relying on y, otherwise y may never be a prime number
            let p : &BigUint = &original_num_prime::RandPrime::gen_prime(&mut rng, 384,  None);
            let x_sq = x * x;
            if let Some(sqrt) = sqrt_prime_power(&x_sq, p, Rc::new(RefCell::new(rng))) {
                if &sqrt != x {
                    prop_assert_eq!(&(p - sqrt), x);
                } else {
                prop_assert_eq!(&sqrt, x);
                }
            }
        }

        #[test]
        fn mul_inv_x_by_x_is_1(ref x in any::<[u8; 32]>()) {
            let p = &(*CAIRO_PRIME).clone().into();
            let pos_x = &BigInt::from_bytes_be(Sign::Plus, x);
            let neg_x = &BigInt::from_bytes_be(Sign::Minus, x);
            let pos_x_inv = mul_inv(pos_x, p);
            let neg_x_inv = mul_inv(neg_x, p);

            prop_assert_eq!((pos_x * pos_x_inv).mod_floor(p), BigInt::one());
            prop_assert_eq!((neg_x * neg_x_inv).mod_floor(p), BigInt::one());
        }
    }
}
