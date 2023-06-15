%builtins output range_check bitwise

from starkware.cairo.common.cairo_keccak.keccak import (
    _prepare_block,
    KECCAK_FULL_RATE_IN_BYTES,
    KECCAK_FULL_RATE_IN_WORDS,
    KECCAK_STATE_SIZE_FELTS,
)
from starkware.cairo.common.math import assert_nn_le
from starkware.cairo.common.cairo_builtins import BitwiseBuiltin
from starkware.cairo.common.alloc import alloc
from starkware.cairo.common.serialize import serialize_word

func _keccak{range_check_ptr, bitwise_ptr: BitwiseBuiltin*, keccak_ptr: felt*}(
    inputs: felt*, n_bytes: felt, state: felt*
) -> (output: felt*) {
    alloc_locals;
    if (nondet %{ ids.n_bytes >= ids.KECCAK_FULL_RATE_IN_BYTES %} != 0) {
        _prepare_block(inputs=inputs, n_bytes=KECCAK_FULL_RATE_IN_BYTES, state=state);
        _block_permutation();

        return _keccak(
            inputs=inputs + KECCAK_FULL_RATE_IN_WORDS,
            n_bytes=n_bytes - KECCAK_FULL_RATE_IN_BYTES,
            state=keccak_ptr - KECCAK_STATE_SIZE_FELTS,
        );
    }

    assert_nn_le(n_bytes, KECCAK_FULL_RATE_IN_BYTES - 1);

    _prepare_block(inputs=inputs, n_bytes=n_bytes, state=state);
    _block_permutation();

    return (output=keccak_ptr - KECCAK_STATE_SIZE_FELTS);
}

func _block_permutation{keccak_ptr: felt*}() {
    %{
        from starkware.cairo.common.cairo_keccak.keccak_utils import keccak_func
        _keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
        assert 0 <= _keccak_state_size_felts < 100

        output_values = keccak_func(memory.get_range(
            ids.keccak_ptr - _keccak_state_size_felts, _keccak_state_size_felts))
        segments.write_arg(ids.keccak_ptr, output_values)
    %}
    let keccak_ptr = keccak_ptr + KECCAK_STATE_SIZE_FELTS;

    return ();
}

func fill_array(array: felt*, base: felt, array_length: felt, iterator: felt) {
    if (iterator == array_length) {
        return ();
    }

    assert array[iterator] = base;

    return fill_array(array, base, array_length, iterator + 1);
}

func main{output_ptr: felt*, range_check_ptr, bitwise_ptr: BitwiseBuiltin*}() {
    alloc_locals;

    let (output: felt*) = alloc();
    let keccak_output = output;

    let (inputs: felt*) = alloc();
    let inputs_start = inputs;
    fill_array(inputs, 9, 3, 0);

    let (state: felt*) = alloc();
    let state_start = state;
    fill_array(state, 5, 25, 0);

    let n_bytes = 24;

    let (res: felt*) = _keccak{keccak_ptr=keccak_output}(
        inputs=inputs_start, n_bytes=n_bytes, state=state_start
    );

    serialize_word(res[0]);
    serialize_word(res[1]);
    serialize_word(res[2]);
    serialize_word(res[4]);

    return ();
}
