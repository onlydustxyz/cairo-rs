%builtins ec_op

from starkware.cairo.common.cairo_builtins import EcOpBuiltin, SignatureBuiltin
from starkware.cairo.common.ec_point import (
    EcPoint,
)
from starkware.cairo.common.cairo_secp.ec import (
    ec_negate,
    compute_doubling_slope,
    compute_slope,
    ec_double,
    fast_ec_add,
    ec_mul_inner,
)
from starkware.cairo.common.cairo_secp.bigint import BigInt3

func test_ec_op_point_not_on_curve{
    ec_op_ptr: EcOpBuiltin*
}() {
    tempvar p = EcPoint(
        0x654fd7e67a123dd13868093b3b7777f1ffef596c2e324f25ceaf9146698482c,
        0x4fad269cbf860980e38768fe9cb6b0b9ab03ee3fe84cfde2eccce597c874fd8,
        );
    assert ec_op_ptr[0].p = p;
    assert ec_op_ptr[0].q = EcPoint(x=p.x, y=p.y + 1);
    assert ec_op_ptr[0].m = 7;
    let ec_op_ptr = &ec_op_ptr[1];
    return ();
}

func main{
    ec_op_ptr: EcOpBuiltin*
}() {
    test_ec_op_point_not_on_curve();
    return ();
}
