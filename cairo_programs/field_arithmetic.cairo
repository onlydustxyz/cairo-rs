%builtins range_check bitwise

// Code taken from https://github.com/NethermindEth/research-basic-Cairo-operations-big-integers/blob/fbf532651959f27037d70cd70ec6dbaf987f535c/lib/field_arithmetic.cairo
from starkware.cairo.common.bitwise import bitwise_and, bitwise_or, bitwise_xor
from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.math import assert_in_range, assert_le, assert_nn_le, assert_not_zero
from starkware.cairo.common.math_cmp import is_le
from starkware.cairo.common.pow import pow
from starkware.cairo.common.registers import get_ap, get_fp_and_pc
from cairo_programs.uint384 import u384, Uint384, Uint384_expand, SHIFT, HALF_SHIFT
from cairo_programs.uint384_extension import u384_ext, Uint768

// Functions for operating elements in a finite field F_p (i.e. modulo a prime p), with p of at most 384 bits
namespace field_arithmetic {
    // Computes a * b modulo p
    func mul{range_check_ptr}(a: Uint384, b: Uint384, p: Uint384) -> (res: Uint384) {
        let (low: Uint384, high: Uint384) = u384.mul_d(a, b);
        let full_mul_result: Uint768 = Uint768(low.d0, low.d1, low.d2, high.d0, high.d1, high.d2);
        let (quotient: Uint768, remainder: Uint384) = u384_ext.unsigned_div_rem_uint768_by_uint384(
            full_mul_result, p
        );
        return (remainder,);
    }

    // Computes a**2 modulo p
    func square{range_check_ptr}(a: Uint384, p: Uint384) -> (res: Uint384) {
        let (low: Uint384, high: Uint384) = u384.square_e(a);
        let full_mul_result: Uint768 = Uint768(low.d0, low.d1, low.d2, high.d0, high.d1, high.d2);
        let (quotient: Uint768, remainder: Uint384) = u384_ext.unsigned_div_rem_uint768_by_uint384(
            full_mul_result, p
        );
        return (remainder,);
    }

    // Finds a square of x in F_p, i.e. x ≅ y**2 (mod p) for some y
    // To do so, the following is done in a hint:
    // 0. Assume x is not  0 mod p
    // 1. Check if x is a square, if yes, find a square root r of it
    // 2. If (and only if not), then gx *is* a square (for g a generator of F_p^*), so find a square root r of it
    // 3. Check in Cairo that r**2 = x (mod p) or r**2 = gx (mod p), respectively
    // NOTE: The function assumes that 0 <= x < p
    func get_square_root{range_check_ptr, bitwise_ptr: BitwiseBuiltin*}(
        x: Uint384, p: Uint384, generator: Uint384
    ) -> (success: felt, res: Uint384) {
        alloc_locals;

        // TODO: Create an equality function within field_arithmetic to avoid overflow bugs
        let (is_zero) = u384.eq(x, Uint384(0, 0, 0));
        if (is_zero == 1) {
            return (1, Uint384(0, 0, 0));
        }

        local success_x: felt;
        local sqrt_x: Uint384;
        local sqrt_gx: Uint384;

        // Compute square roots in a hint
        %{
            from starkware.python.math_utils import is_quad_residue, sqrt

            def split(num: int, num_bits_shift: int = 128, length: int = 3):
                a = []
                for _ in range(length):
                    a.append( num & ((1 << num_bits_shift) - 1) )
                    num = num >> num_bits_shift
                return tuple(a)

            def pack(z, num_bits_shift: int = 128) -> int:
                limbs = (z.d0, z.d1, z.d2)
                return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))


            generator = pack(ids.generator)
            x = pack(ids.x)
            p = pack(ids.p)

            success_x = is_quad_residue(x, p)
            root_x = sqrt(x, p) if success_x else None

            success_gx = is_quad_residue(generator*x, p)
            root_gx = sqrt(generator*x, p) if success_gx else None

            # Check that one is 0 and the other is 1
            if x != 0:
                assert success_x + success_gx ==1

            # `None` means that no root was found, but we need to transform these into a felt no matter what
            if root_x == None:
                root_x = 0
            if root_gx == None:
                root_gx = 0
            ids.success_x = int(success_x)
            split_root_x = split(root_x)
            split_root_gx = split(root_gx)
            ids.sqrt_x.d0 = split_root_x[0]
            ids.sqrt_x.d1 = split_root_x[1]
            ids.sqrt_x.d2 = split_root_x[2]
            ids.sqrt_gx.d0 = split_root_gx[0]
            ids.sqrt_gx.d1 = split_root_gx[1]
            ids.sqrt_gx.d2 = split_root_gx[2]
        %}

        // Verify that the values computed in the hint are what they are supposed to be
        let (gx: Uint384) = mul(generator, x, p);
        if (success_x == 1) {
            u384.check(sqrt_x);
            let (is_valid) = u384.lt(sqrt_x, p);
            assert is_valid = 1;
            let (sqrt_x_squared: Uint384) = mul(sqrt_x, sqrt_x, p);
            // Note these checks may fail if the input x does not satisfy 0<= x < p
            // TODO: Create a equality function within field_arithmetic to avoid overflow bugs
            let (check_x) = u384.eq(x, sqrt_x_squared);
            assert check_x = 1;
            return (1, sqrt_x);
        } else {
            // In this case success_gx = 1
            u384.check(sqrt_gx);
            let (is_valid) = u384.lt(sqrt_gx, p);
            assert is_valid = 1;
            let (sqrt_gx_squared: Uint384) = mul(sqrt_gx, sqrt_gx, p);
            let (check_gx) = u384.eq(gx, sqrt_gx_squared);
            assert check_gx = 1;
            // No square roots were found
            // Note that Uint384(0, 0, 0) is not a square root here, but something needs to be returned
            return (0, Uint384(0, 0, 0));
        }
    }

    // Computes a * b^{-1} modulo p
    // NOTE: The modular inverse of b modulo p is computed in a hint and verified outside the hint with a multiplicaiton
    func div{range_check_ptr}(a: Uint384, b: Uint384, p: Uint384) -> (res: Uint384) {
        alloc_locals;
        local b_inverse_mod_p: Uint384;
        %{
            from starkware.python.math_utils import div_mod

            def split(num: int, num_bits_shift: int, length: int):
                a = []
                for _ in range(length):
                    a.append( num & ((1 << num_bits_shift) - 1) )
                    num = num >> num_bits_shift
                return tuple(a)

            def pack(z, num_bits_shift: int) -> int:
                limbs = (z.d0, z.d1, z.d2)
                return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

            a = pack(ids.a, num_bits_shift = 128)
            b = pack(ids.b, num_bits_shift = 128)
            p = pack(ids.p, num_bits_shift = 128)
            # For python3.8 and above the modular inverse can be computed as follows:
            # b_inverse_mod_p = pow(b, -1, p)
            # Instead we use the python3.7-friendly function div_mod from starkware.python.math_utils
            b_inverse_mod_p = div_mod(1, b, p)


            b_inverse_mod_p_split = split(b_inverse_mod_p, num_bits_shift=128, length=3)

            ids.b_inverse_mod_p.d0 = b_inverse_mod_p_split[0]
            ids.b_inverse_mod_p.d1 = b_inverse_mod_p_split[1]
            ids.b_inverse_mod_p.d2 = b_inverse_mod_p_split[2]
        %}
        u384.check(b_inverse_mod_p);
        let (b_times_b_inverse) = mul(b, b_inverse_mod_p, p);
        assert b_times_b_inverse = Uint384(1, 0, 0);

        let (res: Uint384) = mul(a, b_inverse_mod_p, p);
        return (res,);
    }
}

func test_field_arithmetics_extension_operations{range_check_ptr, bitwise_ptr: BitwiseBuiltin*}() {
    alloc_locals;
    // Test get_square

    // Small prime
    let p_a = Uint384(7, 0, 0);
    let x_a = Uint384(2, 0, 0);
    let generator_a = Uint384(3, 0, 0);
    let (s_a, r_a) = field_arithmetic.get_square_root(x_a, p_a, generator_a);
    assert s_a = 1;

    assert r_a.d0 = 3;
    assert r_a.d1 = 0;
    assert r_a.d2 = 0;

    // Goldilocks Prime
    let p_b = Uint384(18446744069414584321, 0, 0);  // Goldilocks Prime
    let x_b = Uint384(25, 0, 0);
    let generator_b = Uint384(7, 0, 0);
    let (s_b, r_b) = field_arithmetic.get_square_root(x_b, p_b, generator_b);
    assert s_b = 1;

    assert r_b.d0 = 5;
    assert r_b.d1 = 0;
    assert r_b.d2 = 0;

    // Prime 2**101-99
    let p_c = Uint384(77371252455336267181195165, 32767, 0);
    let x_c = Uint384(96059601, 0, 0);
    let generator_c = Uint384(3, 0, 0);
    let (s_c, r_c) = field_arithmetic.get_square_root(x_c, p_c, generator_c);
    assert s_c = 1;

    assert r_c.d0 = 9801;
    assert r_c.d1 = 0;
    assert r_c.d2 = 0;

    // Test div
    // Small inputs
    let a = Uint384(25, 0, 0);
    let a_div = Uint384(5, 0, 0);
    let a_p = Uint384(31, 0, 0);
    let (a_r) = field_arithmetic.div(a, a_div, a_p);
    assert a_r.d0 = 5;
    assert a_r.d1 = 0;
    assert a_r.d2 = 0;

    // Cairo Prime
    let b = Uint384(1, 0, 5044639098474805171426);
    let b_div = Uint384(1, 0, 2);
    let b_p = Uint384(1, 0, 604462909807314605178880);
    let (b_r) = field_arithmetic.div(b, b_div, b_p);
    assert b_r.d0 = 280171807489444591652763463227596156607;
    assert b_r.d1 = 122028556426724038784654414222572127555;
    assert b_r.d2 = 410614585309032623322981;

    return ();
}

func main{range_check_ptr: felt, bitwise_ptr: BitwiseBuiltin*}() {
    test_field_arithmetics_extension_operations();
    return ();
}
