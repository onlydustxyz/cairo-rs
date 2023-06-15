from starkware.cairo.common.uint256 import Uint256, split_64
from starkware.cairo.common.math_cmp import is_le

from cairo_programs.uint384_extension import Uint384, Uint768, u384

struct Uint512 {
    d0: felt,
    d1: felt,
    d2: felt,
    d3: felt,
}

const SHIFT = 2 ** 128;
const ALL_ONES = 2 ** 128 - 1;
const HALF_SHIFT = 2 ** 64;

namespace u512 {
    func add_u512_and_u256{range_check_ptr}(a: Uint512, b: Uint256) -> Uint512 {
        alloc_locals;

        let a_low = Uint256(low=a.d0, high=a.d1);
        let a_high = Uint256(low=a.d2, high=a.d3);

        let (sum_low, carry0) = add_carry(a_low, b);

        local res: Uint512;

        res.d0 = sum_low.low;
        res.d1 = sum_low.high;
        // res.d2 = sum_low.d2;

        // TODO : create add_one (high bits not needed)
        let a_high_plus_carry = add(a_high, Uint256(carry0, 0));

        res.d2 = a_high_plus_carry.low;
        res.d3 = a_high_plus_carry.high;

        return res;
    }

    func mul_u512_by_u256{range_check_ptr}(a: Uint512, b: Uint256) -> Uint768 {
        alloc_locals;
        let (a0, a1) = split_64(a.d0);
        let (a2, a3) = split_64(a.d1);
        let (a4, a5) = split_64(a.d2);
        let (a6, a7) = split_64(a.d3);

        let (b0, b1) = split_64(b.low);
        let (b2, b3) = split_64(b.high);

        local B0 = b0 * HALF_SHIFT;
        local b12 = b1 + b2 * HALF_SHIFT;

        let (res0, carry) = u384.split_128(a1 * B0 + a0 * b.low);
        let (res2, carry) = u384.split_128(a3 * B0 + a2 * b.low + a1 * b12 + a0 * b.high + carry);
        let (res4, carry) = u384.split_128(
            a5 * B0 + a4 * b.low + a3 * b12 + a2 * b.high + a1 * b3 + carry
        );
        let (res6, carry) = u384.split_128(
            a7 * B0 + a6 * b.low + a5 * b12 + a4 * b.high + a3 * b3 + carry
        );
        let (res8, carry) = u384.split_128(a7 * b12 + a6 * b.high + a5 * b3 + carry);
        let (res10, carry) = u384.split_128(a7 * b3 + carry);
        let res = Uint768(d0=res0, d1=res2, d2=res4, d3=res6, d4=res8, d5=res10);
        return res;
    }

    func u512_unsigned_div_rem{range_check_ptr}(x: Uint512, div: Uint256) -> (
        q: Uint512, r: Uint256
    ) {
        alloc_locals;
        local quotient: Uint512;
        local remainder: Uint256;

        %{
            def split(num: int, num_bits_shift: int, length: int):
                a = []
                for _ in range(length):
                    a.append( num & ((1 << num_bits_shift) - 1) )
                    num = num >> num_bits_shift
                return tuple(a)

            def pack(z, num_bits_shift: int) -> int:
                limbs = (z.low, z.high)
                return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

            def pack_extended(z, num_bits_shift: int) -> int:
                limbs = (z.d0, z.d1, z.d2, z.d3)
                return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

            x = pack_extended(ids.x, num_bits_shift = 128)
            div = pack(ids.div, num_bits_shift = 128)

            quotient, remainder = divmod(x, div)

            quotient_split = split(quotient, num_bits_shift=128, length=4)

            ids.quotient.d0 = quotient_split[0]
            ids.quotient.d1 = quotient_split[1]
            ids.quotient.d2 = quotient_split[2]
            ids.quotient.d3 = quotient_split[3]

            remainder_split = split(remainder, num_bits_shift=128, length=2)
            ids.remainder.low = remainder_split[0]
            ids.remainder.high = remainder_split[1]
        %}

        let res_mul: Uint768 = mul_u512_by_u256(quotient, div);

        assert res_mul.d4 = 0;
        assert res_mul.d5 = 0;

        let check_val: Uint512 = add_u512_and_u256(
            Uint512(res_mul.d0, res_mul.d1, res_mul.d2, res_mul.d3), remainder
        );

        // assert add_carry = 0;
        assert check_val = x;

        let is_valid = lt(remainder, div);
        assert is_valid = 1;

        return (quotient, remainder);
    }

    // Verifies that the given integer is valid.
    func check{range_check_ptr}(a: Uint256) {
        // tempvar h = a.high - 2 ** 127;
        [range_check_ptr] = a.low;
        [range_check_ptr + 1] = a.high;
        let range_check_ptr = range_check_ptr + 2;
        return ();
    }

    // Assume a and b are lower than 2**255-19
    func add{range_check_ptr}(a: Uint256, b: Uint256) -> Uint256 {
        alloc_locals;
        local res: Uint256;
        local carry_low: felt;
        // unused. added to use UINT256_ADD
        local carry_high: felt;
        // this hint is not implemented:
        // %{
        //     sum_low = ids.a.low + ids.b.low
        //     ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
        // %}
        %{
            sum_low = ids.a.low + ids.b.low
            ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
            sum_high = ids.a.high + ids.b.high + ids.carry_low
            ids.carry_high = 1 if sum_high >= ids.SHIFT else 0
        %}
        // changed hint, no carry_high
        assert carry_low * carry_low = carry_low;

        assert res.low = a.low + b.low - carry_low * SHIFT;
        assert res.high = a.high + b.high + carry_low;
        // check(res);

        return res;
    }

    func add_carry{range_check_ptr}(a: Uint256, b: Uint256) -> (res: Uint256, carry: felt) {
        alloc_locals;
        local res: Uint256;
        local carry_low: felt;
        local carry_high: felt;
        %{
            sum_low = ids.a.low + ids.b.low
            ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
            sum_high = ids.a.high + ids.b.high + ids.carry_low
            ids.carry_high = 1 if sum_high >= ids.SHIFT else 0
        %}

        assert carry_low * carry_low = carry_low;
        assert carry_high * carry_high = carry_high;

        assert res.low = a.low + b.low - carry_low * SHIFT;
        assert res.high = a.high + b.high + carry_low - carry_high * SHIFT;
        check(res);

        return (res, carry_high);
    }

    func lt{range_check_ptr}(a: Uint256, b: Uint256) -> felt {
        if (a.high == b.high) {
            return is_le(a.low + 1, b.low);
        }
        return is_le(a.high + 1, b.high);
    }
}

func main() {
    return ();
}
