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

func test_ec_op_invalid_input{
    ec_op_ptr: EcOpBuiltin*
}() {
    // Choose p = 4 * q.
    // Trying to compute p + 8 * q starts with the following pairs of points:
    //   (p, q),
    //   (p, 2 * q),
    //   (p, 4 * q),
    //   (p, 8 * q),
    // But since p = 4 * q, the pair (p, 4 * q) is invalid (the x-coordinate is the same).
    assert ec_op_ptr[0].p = EcPoint(
        0x6a4beaef5a93425b973179cdba0c9d42f30e01a5f1e2db73da0884b8d6756fc,
        0x72565ec81bc09ff53fbfad99324a92aa5b39fb58267e395e8abe36290ebf24f,
        );
    assert ec_op_ptr[0].q = EcPoint(
        0x654fd7e67a123dd13868093b3b7777f1ffef596c2e324f25ceaf9146698482c,
        0x4fad269cbf860980e38768fe9cb6b0b9ab03ee3fe84cfde2eccce597c874fd8,
        );
    assert ec_op_ptr[0].m = 8;
    let ec_op_ptr = &ec_op_ptr[1];
    return ();
}

func main{
    ec_op_ptr: EcOpBuiltin*
}() {
    test_ec_op_invalid_input();
    return ();
}
