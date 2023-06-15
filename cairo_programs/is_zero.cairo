%builtins range_check

from starkware.cairo.common.cairo_secp.bigint import BigInt3, UnreducedBigInt3, nondet_bigint3
from starkware.cairo.common.cairo_secp.field import unreduced_mul, verify_zero

// Returns 1 if x == 0 (mod secp256k1_prime), and 0 otherwise.
//
// Completeness assumption: x's limbs are in the range (-BASE, 2*BASE).
// Soundness assumption: x's limbs are in the range (-2**107.49, 2**107.49).
func is_zero{range_check_ptr}(x: BigInt3) -> (res: felt) {
    %{
        from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

        x = pack(ids.x, PRIME) % SECP_P
    %}
    if (nondet %{ x == 0 %} != 0) {
        verify_zero(UnreducedBigInt3(d0=x.d0, d1=x.d1, d2=x.d2));
        return (res=1);
    }

    %{
        from starkware.python.math_utils import div_mod

        value = x_inv = div_mod(1, x, SECP_P)
    %}
    let (x_inv) = nondet_bigint3();
    let (x_x_inv) = unreduced_mul(x, x_inv);

    // Check that x * x_inv = 1 to verify that x != 0.
    verify_zero(UnreducedBigInt3(d0=x_x_inv.d0 - 1, d1=x_x_inv.d1, d2=x_x_inv.d2));
    return (res=0);
}

func is_zero_alt{range_check_ptr}(x: BigInt3) -> (res: felt) {
    %{
        from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

        x = pack(ids.x, PRIME) % SECP_P
    %}
    %{ memory[ap] = int(x == 0) %}
    tempvar x_is_zero;

    if (x_is_zero != 0) {
        verify_zero(UnreducedBigInt3(d0=x.d0, d1=x.d1, d2=x.d2));
        return (res=1);
    }

    %{
        from starkware.cairo.common.cairo_secp.secp_utils import SECP_P
        from starkware.python.math_utils import div_mod

        value = x_inv = div_mod(1, x, SECP_P)
    %}
    let (x_inv) = nondet_bigint3();
    let (x_x_inv) = unreduced_mul(x, x_inv);

    // Check that x * x_inv = 1 to verify that x != 0.
    verify_zero(UnreducedBigInt3(d0=x_x_inv.d0 - 1, d1=x_x_inv.d1, d2=x_x_inv.d2));
    return (res=0);
}

func test_is_zero{range_check_ptr}() -> () {
    let zero = BigInt3(0, 0, 0);

    let (res: felt) = is_zero(zero);
    assert res = 1;

    let one = BigInt3(1, 0, 0);

    let (res: felt) = is_zero(one);
    assert res = 0;

    let secp256k1_prime = BigInt3(
        77371252455336262886226991, 77371252455336267181195263, 19342813113834066795298815
    );

    let (res: felt) = is_zero(secp256k1_prime);
    assert res = 1;

    return ();
}

func test_is_zero_alt{range_check_ptr}() -> () {
    let zero = BigInt3(0, 0, 0);

    let (res: felt) = is_zero_alt(zero);
    assert res = 1;

    let one = BigInt3(1, 0, 0);

    let (res: felt) = is_zero_alt(one);
    assert res = 0;

    let secp256k1_prime = BigInt3(
        77371252455336262886226991, 77371252455336267181195263, 19342813113834066795298815
    );

    let (res: felt) = is_zero_alt(secp256k1_prime);
    assert res = 1;

    return ();
}

func main{range_check_ptr}() -> () {
    test_is_zero();
    test_is_zero_alt();

    return ();
}
