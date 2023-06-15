%builtins range_check

from starkware.cairo.common.cairo_secp.bigint import BigInt3, UnreducedBigInt3, nondet_bigint3
from starkware.cairo.common.cairo_secp.field import unreduced_mul, verify_zero

func is_zero_pack{range_check_ptr}(x: BigInt3) -> (res: felt) {
    // just to import SECP_P
    %{
        from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

        value = pack(ids.x, PRIME) % SECP_P
    %}
    %{
        from starkware.cairo.common.cairo_secp.secp_utils import pack

        x = pack(ids.x, PRIME) % SECP_P
    %}
    if (nondet %{ x == 0 %} != 0) {
        return (res=1);
    }
    return (res=0);
}

func test_is_zero_pack{range_check_ptr}() -> () {
    let zero = BigInt3(0, 0, 0);

    let (res: felt) = is_zero_pack(zero);
    assert res = 1;

    let one = BigInt3(1, 0, 0);

    let (res: felt) = is_zero_pack(one);
    assert res = 0;

    let secp256k1_prime = BigInt3(
        77371252455336262886226991, 77371252455336267181195263, 19342813113834066795298815
    );

    let (res: felt) = is_zero_pack(secp256k1_prime);
    assert res = 1;

    return ();
}

func main{range_check_ptr}() -> () {
    test_is_zero_pack();

    return ();
}
