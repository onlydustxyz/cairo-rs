%builtins range_check

from cairo_programs.fq import u512, Uint256, Uint512

func test_u512_unsigned_div_rem{range_check_ptr}() {
    let x = Uint512(26362362, 32523523, 135525, 15521);
    let div = Uint256(1, 0);

    let (q, r) = u512.u512_unsigned_div_rem(x, div);

    // x / 1 = x
    assert q = Uint512(26362362, 32523523, 135525, 15521);
    // x % 1 = 0
    assert r = Uint256(0, 0);

    let x = Uint512(
        154693353187447763037373048681595478410,
        49972746532502551770198072697358847685,
        274245096733591597256384467752461786671,
        218971140682455857220416392230637548564,
    );
    let div = Uint256(
        103510830969771876705678198448587782120, 321696934602460025966614305804515599536
    );

    let (q, r) = u512.u512_unsigned_div_rem(x, div);

    assert q = Uint512(
        203702859112426540420143348051200561496, 231621784431619772183895351989849416356, 0, 0
    );
    assert r = Uint256(
        294644766503248848032677663267093316042, 283333580363207111408148984050656446476
    );

    return ();
}

func main{range_check_ptr}() {
    test_u512_unsigned_div_rem();

    return ();
}
