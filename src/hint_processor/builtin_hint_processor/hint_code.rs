pub const ADD_SEGMENT: &str = "memory[ap] = segments.add()";

pub const VM_ENTER_SCOPE: &str = "vm_enter_scope()";
pub const VM_EXIT_SCOPE: &str = "vm_exit_scope()";

pub const MEMCPY_ENTER_SCOPE: &str = "vm_enter_scope({'n': ids.len})";
pub const MEMCPY_CONTINUE_COPYING: &str = r#"n -= 1
ids.continue_copying = 1 if n > 0 else 0"#;

pub const MEMSET_ENTER_SCOPE: &str = "vm_enter_scope({'n': ids.n})";
pub const MEMSET_CONTINUE_LOOP: &str = r#"n -= 1
ids.continue_loop = 1 if n > 0 else 0"#;

pub const POW: &str = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";

pub const IS_NN: &str = "memory[ap] = 0 if 0 <= (ids.a % PRIME) < range_check_builtin.bound else 1";
pub const IS_NN_OUT_OF_RANGE: &str =
    "memory[ap] = 0 if 0 <= ((-ids.a - 1) % PRIME) < range_check_builtin.bound else 1";
pub const IS_LE_FELT: &str = "memory[ap] = 0 if (ids.a % PRIME) <= (ids.b % PRIME) else 1";
pub const IS_POSITIVE: &str = r#"from starkware.cairo.common.math_utils import is_positive
ids.is_positive = 1 if is_positive(
    value=ids.value, prime=PRIME, rc_bound=range_check_builtin.bound) else 0"#;

pub const ASSERT_NN: &str = r#"from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.a)
assert 0 <= ids.a % PRIME < range_check_builtin.bound, f'a = {ids.a} is out of range.'"#;

pub const ASSERT_NOT_ZERO: &str = r#"from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.value)
assert ids.value % PRIME != 0, f'assert_not_zero failed: {ids.value} = 0.'"#;

pub const ASSERT_NOT_EQUAL: &str = r#"from starkware.cairo.lang.vm.relocatable import RelocatableValue
both_ints = isinstance(ids.a, int) and isinstance(ids.b, int)
both_relocatable = (
    isinstance(ids.a, RelocatableValue) and isinstance(ids.b, RelocatableValue) and
    ids.a.segment_index == ids.b.segment_index)
assert both_ints or both_relocatable, \
    f'assert_not_equal failed: non-comparable values: {ids.a}, {ids.b}.'
assert (ids.a - ids.b) % PRIME != 0, f'assert_not_equal failed: {ids.a} = {ids.b}.'"#;

pub const ASSERT_LE_FELT: &str = r#"import itertools

from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.a)
assert_integer(ids.b)
a = ids.a % PRIME
b = ids.b % PRIME
assert a <= b, f'a = {a} is not less than or equal to b = {b}.'

# Find an arc less than PRIME / 3, and another less than PRIME / 2.
lengths_and_indices = [(a, 0), (b - a, 1), (PRIME - 1 - b, 2)]
lengths_and_indices.sort()
assert lengths_and_indices[0][0] <= PRIME // 3 and lengths_and_indices[1][0] <= PRIME // 2
excluded = lengths_and_indices[2][1]

memory[ids.range_check_ptr + 1], memory[ids.range_check_ptr + 0] = (
    divmod(lengths_and_indices[0][0], ids.PRIME_OVER_3_HIGH))
memory[ids.range_check_ptr + 3], memory[ids.range_check_ptr + 2] = (
    divmod(lengths_and_indices[1][0], ids.PRIME_OVER_2_HIGH))"#;

pub const ASSERT_LE_FELT_EXCLUDED_0: &str = "memory[ap] = 1 if excluded != 0 else 0";
pub const ASSERT_LE_FELT_EXCLUDED_1: &str = "memory[ap] = 1 if excluded != 1 else 0";
pub const ASSERT_LE_FELT_EXCLUDED_2: &str = "assert excluded == 2";

pub const ASSERT_LT_FELT: &str = r#"from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.a)
assert_integer(ids.b)
assert (ids.a % PRIME) < (ids.b % PRIME), \
    f'a = {ids.a % PRIME} is not less than b = {ids.b % PRIME}.'"#;

pub const SPLIT_INT_ASSERT_RANGE: &str =
    "assert ids.value == 0, 'split_int(): value is out of range.'";

pub const ASSERT_250_BITS: &str = r#"from starkware.cairo.common.math_utils import as_int

# Correctness check.
value = as_int(ids.value, PRIME) % PRIME
assert value < ids.UPPER_BOUND, f'{value} is outside of the range [0, 2**250).'

# Calculation for the assertion.
ids.high, ids.low = divmod(ids.value, ids.SHIFT)"#;

pub const SPLIT_INT: &str = r#"memory[ids.output] = res = (int(ids.value) % PRIME) % ids.base
assert res < ids.bound, f'split_int(): Limb {res} is out of range.'"#;

pub const SPLIT_64: &str = r#"ids.low = ids.a & ((1<<64) - 1)
ids.high = ids.a >> 64"#;

pub const SPLIT_FELT: &str = r#"from starkware.cairo.common.math_utils import assert_integer
assert ids.MAX_HIGH < 2**128 and ids.MAX_LOW < 2**128
assert PRIME - 1 == ids.MAX_HIGH * 2**128 + ids.MAX_LOW
assert_integer(ids.value)
ids.low = ids.value & ((1 << 128) - 1)
ids.high = ids.value >> 128"#;

pub const SQRT: &str = r#"from starkware.python.math_utils import isqrt
value = ids.value % PRIME
assert value < 2 ** 250, f"value={value} is outside of the range [0, 2**250)."
assert 2 ** 250 < PRIME
ids.root = isqrt(value)"#;

pub const UNSIGNED_DIV_REM: &str = r#"from starkware.cairo.common.math_utils import assert_integer
assert_integer(ids.div)
assert 0 < ids.div <= PRIME // range_check_builtin.bound, \
    f'div={hex(ids.div)} is out of the valid range.'
ids.q, ids.r = divmod(ids.value, ids.div)"#;

pub const SIGNED_DIV_REM: &str = r#"from starkware.cairo.common.math_utils import as_int, assert_integer

assert_integer(ids.div)
assert 0 < ids.div <= PRIME // range_check_builtin.bound, \
    f'div={hex(ids.div)} is out of the valid range.'

assert_integer(ids.bound)
assert ids.bound <= range_check_builtin.bound // 2, \
    f'bound={hex(ids.bound)} is out of the valid range.'

int_value = as_int(ids.value, PRIME)
q, ids.r = divmod(int_value, ids.div)

assert -ids.bound <= q < ids.bound, \
    f'{int_value} / {ids.div} = {q} is out of the range [{-ids.bound}, {ids.bound}).'

ids.biased_q = q + ids.bound"#;

pub const IS_QUAD_RESIDUE: &str = r#"from starkware.crypto.signature.signature import FIELD_PRIME
from starkware.python.math_utils import div_mod, is_quad_residue, sqrt

x = ids.x
if is_quad_residue(x, FIELD_PRIME):
    ids.y = sqrt(x, FIELD_PRIME)
else:
    ids.y = sqrt(div_mod(x, 3, FIELD_PRIME), FIELD_PRIME)"#;

pub const FIND_ELEMENT: &str = r#"array_ptr = ids.array_ptr
elm_size = ids.elm_size
assert isinstance(elm_size, int) and elm_size > 0, \
    f'Invalid value for elm_size. Got: {elm_size}.'
key = ids.key

if '__find_element_index' in globals():
    ids.index = __find_element_index
    found_key = memory[array_ptr + elm_size * __find_element_index]
    assert found_key == key, \
        f'Invalid index found in __find_element_index. index: {__find_element_index}, ' \
        f'expected key {key}, found key: {found_key}.'
    # Delete __find_element_index to make sure it's not used for the next calls.
    del __find_element_index
else:
    n_elms = ids.n_elms
    assert isinstance(n_elms, int) and n_elms >= 0, \
        f'Invalid value for n_elms. Got: {n_elms}.'
    if '__find_element_max_size' in globals():
        assert n_elms <= __find_element_max_size, \
            f'find_element() can only be used with n_elms<={__find_element_max_size}. ' \
            f'Got: n_elms={n_elms}.'

    for i in range(n_elms):
        if memory[array_ptr + elm_size * i] == key:
            ids.index = i
            break
    else:
        raise ValueError(f'Key {key} was not found.')"#;

pub const SEARCH_SORTED_LOWER: &str = r#"array_ptr = ids.array_ptr
elm_size = ids.elm_size
assert isinstance(elm_size, int) and elm_size > 0, \
    f'Invalid value for elm_size. Got: {elm_size}.'

n_elms = ids.n_elms
assert isinstance(n_elms, int) and n_elms >= 0, \
    f'Invalid value for n_elms. Got: {n_elms}.'
if '__find_element_max_size' in globals():
    assert n_elms <= __find_element_max_size, \
        f'find_element() can only be used with n_elms<={__find_element_max_size}. ' \
        f'Got: n_elms={n_elms}.'

for i in range(n_elms):
    if memory[array_ptr + elm_size * i] >= ids.key:
        ids.index = i
        break
else:
    ids.index = n_elms"#;

pub const SET_ADD: &str = r#"assert ids.elm_size > 0
assert ids.set_ptr <= ids.set_end_ptr
elm_list = memory.get_range(ids.elm_ptr, ids.elm_size)
for i in range(0, ids.set_end_ptr - ids.set_ptr, ids.elm_size):
    if memory.get_range(ids.set_ptr + i, ids.elm_size) == elm_list:
        ids.index = i // ids.elm_size
        ids.is_elm_in_set = 1
        break
else:
    ids.is_elm_in_set = 0"#;

pub const DEFAULT_DICT_NEW: &str = r#"if '__dict_manager' not in globals():
    from starkware.cairo.common.dict import DictManager
    __dict_manager = DictManager()

memory[ap] = __dict_manager.new_default_dict(segments, ids.default_value)"#;

pub const DICT_NEW: &str = r#"if '__dict_manager' not in globals():
    from starkware.cairo.common.dict import DictManager
    __dict_manager = DictManager()

memory[ap] = __dict_manager.new_dict(segments, initial_dict)
del initial_dict"#;

pub const DICT_READ: &str = r#"dict_tracker = __dict_manager.get_tracker(ids.dict_ptr)
dict_tracker.current_ptr += ids.DictAccess.SIZE
ids.value = dict_tracker.data[ids.key]"#;

pub const DICT_WRITE: &str = r#"dict_tracker = __dict_manager.get_tracker(ids.dict_ptr)
dict_tracker.current_ptr += ids.DictAccess.SIZE
ids.dict_ptr.prev_value = dict_tracker.data[ids.key]
dict_tracker.data[ids.key] = ids.new_value"#;

pub const DICT_UPDATE: &str = r#"# Verify dict pointer and prev value.
dict_tracker = __dict_manager.get_tracker(ids.dict_ptr)
current_value = dict_tracker.data[ids.key]
assert current_value == ids.prev_value, \
    f'Wrong previous value in dict. Got {ids.prev_value}, expected {current_value}.'

# Update value.
dict_tracker.data[ids.key] = ids.new_value
dict_tracker.current_ptr += ids.DictAccess.SIZE"#;

pub const SQUASH_DICT: &str = r#"dict_access_size = ids.DictAccess.SIZE
address = ids.dict_accesses.address_
assert ids.ptr_diff % dict_access_size == 0, \
    'Accesses array size must be divisible by DictAccess.SIZE'
n_accesses = ids.n_accesses
if '__squash_dict_max_size' in globals():
    assert n_accesses <= __squash_dict_max_size, \
        f'squash_dict() can only be used with n_accesses<={__squash_dict_max_size}. ' \
        f'Got: n_accesses={n_accesses}.'
# A map from key to the list of indices accessing it.
access_indices = {}
for i in range(n_accesses):
    key = memory[address + dict_access_size * i]
    access_indices.setdefault(key, []).append(i)
# Descending list of keys.
keys = sorted(access_indices.keys(), reverse=True)
# Are the keys used bigger than range_check bound.
ids.big_keys = 1 if keys[0] >= range_check_builtin.bound else 0
ids.first_key = key = keys.pop()"#;

pub const SQUASH_DICT_INNER_SKIP_LOOP: &str =
    "ids.should_skip_loop = 0 if current_access_indices else 1";
pub const SQUASH_DICT_INNER_FIRST_ITERATION: &str = r#"current_access_indices = sorted(access_indices[key])[::-1]
current_access_index = current_access_indices.pop()
memory[ids.range_check_ptr] = current_access_index"#;

pub const SQUASH_DICT_INNER_CHECK_ACCESS_INDEX: &str = r#"new_access_index = current_access_indices.pop()
ids.loop_temps.index_delta_minus1 = new_access_index - current_access_index - 1
current_access_index = new_access_index"#;

pub const SQUASH_DICT_INNER_CONTINUE_LOOP: &str =
    "ids.loop_temps.should_continue = 1 if current_access_indices else 0";
pub const SQUASH_DICT_INNER_ASSERT_LEN_KEYS: &str = "assert len(keys) == 0";
pub const SQUASH_DICT_INNER_LEN_ASSERT: &str = "assert len(current_access_indices) == 0";
pub const SQUASH_DICT_INNER_USED_ACCESSES_ASSERT: &str =
    "assert ids.n_used_accesses == len(access_indices[key])";
pub const SQUASH_DICT_INNER_NEXT_KEY: &str = r#"assert len(keys) > 0, 'No keys left but remaining_accesses > 0.'
ids.next_key = key = keys.pop()"#;

pub const DICT_SQUASH_COPY_DICT: &str = r#"# Prepare arguments for dict_new. In particular, the same dictionary values should be copied
# to the new (squashed) dictionary.
vm_enter_scope({
    # Make __dict_manager accessible.
    '__dict_manager': __dict_manager,
    # Create a copy of the dict, in case it changes in the future.
    'initial_dict': dict(__dict_manager.get_dict(ids.dict_accesses_end)),
})"#;

pub const DICT_SQUASH_UPDATE_PTR: &str = r#"# Update the DictTracker's current_ptr to point to the end of the squashed dict.
__dict_manager.get_tracker(ids.squashed_dict_start).current_ptr = \
    ids.squashed_dict_end.address_"#;

pub const BIGINT_TO_UINT256: &str = "ids.low = (ids.x.d0 + ids.x.d1 * ids.BASE) & ((1 << 128) - 1)";
pub const UINT256_ADD: &str = r#"sum_low = ids.a.low + ids.b.low
ids.carry_low = 1 if sum_low >= ids.SHIFT else 0
sum_high = ids.a.high + ids.b.high + ids.carry_low
ids.carry_high = 1 if sum_high >= ids.SHIFT else 0"#;

pub const UINT128_ADD: &str = r#"res = ids.a + ids.b
ids.carry = 1 if res >= ids.SHIFT else 0"#;

pub const UINT256_SUB: &str = r#"def split(num: int, num_bits_shift: int = 128, length: int = 2):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift
    return tuple(a)

def pack(z, num_bits_shift: int = 128) -> int:
    limbs = (z.low, z.high)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack(ids.a)
b = pack(ids.b)
res = (a - b)%2**256
res_split = split(res)
ids.res.low = res_split[0]
ids.res.high = res_split[1]"#;

pub const UINT256_SQRT: &str = r#"from starkware.python.math_utils import isqrt
n = (ids.n.high << 128) + ids.n.low
root = isqrt(n)
assert 0 <= root < 2 ** 128
ids.root.low = root
ids.root.high = 0"#;

pub const UINT256_SQRT_FELT: &str = r#"from starkware.python.math_utils import isqrt
n = (ids.n.high << 128) + ids.n.low
root = isqrt(n)
assert 0 <= root < 2 ** 128
ids.root = root"#;

pub const UINT256_SIGNED_NN: &str = "memory[ap] = 1 if 0 <= (ids.a.high % PRIME) < 2 ** 127 else 0";

pub const UINT256_UNSIGNED_DIV_REM: &str = r#"a = (ids.a.high << 128) + ids.a.low
div = (ids.div.high << 128) + ids.div.low
quotient, remainder = divmod(a, div)

ids.quotient.low = quotient & ((1 << 128) - 1)
ids.quotient.high = quotient >> 128
ids.remainder.low = remainder & ((1 << 128) - 1)
ids.remainder.high = remainder >> 128"#;

pub const UINT256_EXPANDED_UNSIGNED_DIV_REM: &str = r#"a = (ids.a.high << 128) + ids.a.low
div = (ids.div.b23 << 128) + ids.div.b01
quotient, remainder = divmod(a, div)

ids.quotient.low = quotient & ((1 << 128) - 1)
ids.quotient.high = quotient >> 128
ids.remainder.low = remainder & ((1 << 128) - 1)
ids.remainder.high = remainder >> 128"#;

pub const UINT256_MUL_DIV_MOD: &str = r#"a = (ids.a.high << 128) + ids.a.low
b = (ids.b.high << 128) + ids.b.low
div = (ids.div.high << 128) + ids.div.low
quotient, remainder = divmod(a * b, div)

ids.quotient_low.low = quotient & ((1 << 128) - 1)
ids.quotient_low.high = (quotient >> 128) & ((1 << 128) - 1)
ids.quotient_high.low = (quotient >> 256) & ((1 << 128) - 1)
ids.quotient_high.high = quotient >> 384
ids.remainder.low = remainder & ((1 << 128) - 1)
ids.remainder.high = remainder >> 128"#;

pub const USORT_ENTER_SCOPE: &str =
    "vm_enter_scope(dict(__usort_max_size = globals().get('__usort_max_size')))";
pub const USORT_BODY: &str = r#"from collections import defaultdict

input_ptr = ids.input
input_len = int(ids.input_len)
if __usort_max_size is not None:
    assert input_len <= __usort_max_size, (
        f"usort() can only be used with input_len<={__usort_max_size}. "
        f"Got: input_len={input_len}."
    )

positions_dict = defaultdict(list)
for i in range(input_len):
    val = memory[input_ptr + i]
    positions_dict[val].append(i)

output = sorted(positions_dict.keys())
ids.output_len = len(output)
ids.output = segments.gen_arg(output)
ids.multiplicities = segments.gen_arg([len(positions_dict[k]) for k in output])"#;

pub const USORT_VERIFY: &str = r#"last_pos = 0
positions = positions_dict[ids.value][::-1]"#;

pub const USORT_VERIFY_MULTIPLICITY_ASSERT: &str = "assert len(positions) == 0";
pub const USORT_VERIFY_MULTIPLICITY_BODY: &str = r#"current_pos = positions.pop()
ids.next_item_index = current_pos - last_pos
last_pos = current_pos + 1"#;

pub const BLAKE2S_COMPUTE: &str = r#"from starkware.cairo.common.cairo_blake2s.blake2s_utils import compute_blake2s_func
compute_blake2s_func(segments=segments, output_ptr=ids.output)"#;

pub const BLAKE2S_FINALIZE: &str = r#"# Add dummy pairs of input and output.
from starkware.cairo.common.cairo_blake2s.blake2s_utils import IV, blake2s_compress

_n_packed_instances = int(ids.N_PACKED_INSTANCES)
assert 0 <= _n_packed_instances < 20
_blake2s_input_chunk_size_felts = int(ids.INPUT_BLOCK_FELTS)
assert 0 <= _blake2s_input_chunk_size_felts < 100

message = [0] * _blake2s_input_chunk_size_felts
modified_iv = [IV[0] ^ 0x01010020] + IV[1:]
output = blake2s_compress(
    message=message,
    h=modified_iv,
    t0=0,
    t1=0,
    f0=0xffffffff,
    f1=0,
)
padding = (modified_iv + message + [0, 0xffffffff] + output) * (_n_packed_instances - 1)
segments.write_arg(ids.blake2s_ptr_end, padding)"#;

pub const BLAKE2S_ADD_UINT256: &str = r#"B = 32
MASK = 2 ** 32 - 1
segments.write_arg(ids.data, [(ids.low >> (B * i)) & MASK for i in range(4)])
segments.write_arg(ids.data + 4, [(ids.high >> (B * i)) & MASK for i in range(4)]"#;

pub const BLAKE2S_ADD_UINT256_BIGEND: &str = r#"B = 32
MASK = 2 ** 32 - 1
segments.write_arg(ids.data, [(ids.high >> (B * (3 - i))) & MASK for i in range(4)])
segments.write_arg(ids.data + 4, [(ids.low >> (B * (3 - i))) & MASK for i in range(4)])"#;

pub const NONDET_BIGINT3: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import split

segments.write_arg(ids.res.address_, split(value))"#;

pub const VERIFY_ZERO_V1: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

q, r = divmod(pack(ids.val, PRIME), SECP_P)
assert r == 0, f"verify_zero: Invalid input {ids.val.d0, ids.val.d1, ids.val.d2}."
ids.q = q % PRIME"#;

pub const VERIFY_ZERO_V2: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P
q, r = divmod(pack(ids.val, PRIME), SECP_P)
assert r == 0, f"verify_zero: Invalid input {ids.val.d0, ids.val.d1, ids.val.d2}."
ids.q = q % PRIME"#;

pub const VERIFY_ZERO_EXTERNAL_SECP: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import pack

q, r = divmod(pack(ids.val, PRIME), SECP_P)
assert r == 0, f"verify_zero: Invalid input {ids.val.d0, ids.val.d1, ids.val.d2}."
ids.q = q % PRIME"#;

pub const REDUCE: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

value = pack(ids.x, PRIME) % SECP_P"#;

pub const UNSAFE_KECCAK: &str = r#"from eth_hash.auto import keccak

data, length = ids.data, ids.length

if '__keccak_max_size' in globals():
    assert length <= __keccak_max_size, \
        f'unsafe_keccak() can only be used with length<={__keccak_max_size}. ' \
        f'Got: length={length}.'

keccak_input = bytearray()
for word_i, byte_i in enumerate(range(0, length, 16)):
    word = memory[data + word_i]
    n_bytes = min(16, length - byte_i)
    assert 0 <= word < 2 ** (8 * n_bytes)
    keccak_input += word.to_bytes(n_bytes, 'big')

hashed = keccak(keccak_input)
ids.high = int.from_bytes(hashed[:16], 'big')
ids.low = int.from_bytes(hashed[16:32], 'big')"#;

pub const UNSAFE_KECCAK_FINALIZE: &str = r#"from eth_hash.auto import keccak
keccak_input = bytearray()
n_elms = ids.keccak_state.end_ptr - ids.keccak_state.start_ptr
for word in memory.get_range(ids.keccak_state.start_ptr, n_elms):
    keccak_input += word.to_bytes(16, 'big')
hashed = keccak(keccak_input)
ids.high = int.from_bytes(hashed[:16], 'big')
ids.low = int.from_bytes(hashed[16:32], 'big')"#;

pub const IS_ZERO_NONDET: &str = "memory[ap] = to_felt_or_relocatable(x == 0)";
pub const IS_ZERO_INT: &str = "memory[ap] = int(x == 0)";
pub const IS_ZERO_PACK: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

x = pack(ids.x, PRIME) % SECP_P"#;

pub const IS_ZERO_PACK_EXTERNAL_SECP: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import pack

x = pack(ids.x, PRIME) % SECP_P"#;

pub const IS_ZERO_ASSIGN_SCOPE_VARS: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P
from starkware.python.math_utils import div_mod

value = x_inv = div_mod(1, x, SECP_P)"#;

pub const IS_ZERO_ASSIGN_SCOPE_VARS_EXTERNAL_SECP: &str = r#"from starkware.python.math_utils import div_mod

value = x_inv = div_mod(1, x, SECP_P)"#;

pub const DIV_MOD_N_PACKED_DIVMOD_V1: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import N, pack
from starkware.python.math_utils import div_mod, safe_div

a = pack(ids.a, PRIME)
b = pack(ids.b, PRIME)
value = res = div_mod(a, b, N)"#;

pub const DIV_MOD_N_PACKED_DIVMOD_EXTERNAL_N: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import div_mod, safe_div

a = pack(ids.a, PRIME)
b = pack(ids.b, PRIME)
value = res = div_mod(a, b, N)"#;

pub const DIV_MOD_N_SAFE_DIV: &str = r#"value = k = safe_div(res * b - a, N)"#;

pub const GET_FELT_BIT_LENGTH: &str = r#"x = ids.x
ids.bit_length = x.bit_length()"#;

pub const BIGINT_PACK_DIV_MOD: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.cairo.common.math_utils import as_int
from starkware.python.math_utils import div_mod, safe_div

p = pack(ids.P, PRIME)
x = pack(ids.x, PRIME) + as_int(ids.x.d3, PRIME) * ids.BASE ** 3 + as_int(ids.x.d4, PRIME) * ids.BASE ** 4
y = pack(ids.y, PRIME)

value = res = div_mod(x, y, p)"#;

pub const BIGINT_SAFE_DIV: &str = r#"k = safe_div(res * y - x, p)
value = k if k > 0 else 0 - k
ids.flag = 1 if k > 0 else 0"#;

pub const DIV_MOD_N_SAFE_DIV_PLUS_ONE: &str =
    r#"value = k_plus_one = safe_div(res * b - a, N) + 1"#;

pub const GET_POINT_FROM_X: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

x_cube_int = pack(ids.x_cube, PRIME) % SECP_P
y_square_int = (x_cube_int + ids.BETA) % SECP_P
y = pow(y_square_int, (SECP_P + 1) // 4, SECP_P)

# We need to decide whether to take y or SECP_P - y.
if ids.v % 2 == y % 2:
    value = y
else:
    value = (-y) % SECP_P"#;

pub const EC_NEGATE: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

y = pack(ids.point.y, PRIME) % SECP_P
# The modulo operation in python always returns a nonnegative number.
value = (-y) % SECP_P"#;

pub const EC_DOUBLE_SCOPE: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
from starkware.python.math_utils import ec_double_slope

# Compute the slope.
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)
value = slope = ec_double_slope(point=(x, y), alpha=0, p=SECP_P)"#;

pub const EC_DOUBLE_SCOPE_WHITELIST: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
from starkware.python.math_utils import div_mod

# Compute the slope.
x = pack(ids.pt.x, PRIME)
y = pack(ids.pt.y, PRIME)
value = slope = div_mod(3 * x ** 2, 2 * y, SECP_P)"#;

pub const COMPUTE_SLOPE: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
from starkware.python.math_utils import line_slope

# Compute the slope.
x0 = pack(ids.point0.x, PRIME)
y0 = pack(ids.point0.y, PRIME)
x1 = pack(ids.point1.x, PRIME)
y1 = pack(ids.point1.y, PRIME)
value = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP_P)"#;

pub const COMPUTE_SLOPE_SECP256R1: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import line_slope

# Compute the slope.
x0 = pack(ids.point0.x, PRIME)
y0 = pack(ids.point0.y, PRIME)
x1 = pack(ids.point1.x, PRIME)
y1 = pack(ids.point1.y, PRIME)
value = slope = line_slope(point1=(x0, y0), point2=(x1, y1), p=SECP_P)"#;
pub const IMPORT_SECP256R1_P: &str =
    "from starkware.cairo.common.cairo_secp.secp256r1_utils import SECP256R1_P as SECP_P";

pub const COMPUTE_SLOPE_WHITELIST: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack
from starkware.python.math_utils import div_mod

# Compute the slope.
x0 = pack(ids.pt0.x, PRIME)
y0 = pack(ids.pt0.y, PRIME)
x1 = pack(ids.pt1.x, PRIME)
y1 = pack(ids.pt1.y, PRIME)
value = slope = div_mod(y0 - y1, x0 - x1, SECP_P)"#;

pub const EC_DOUBLE_ASSIGN_NEW_X_V1: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

slope = pack(ids.slope, PRIME)
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P"#;

pub const EC_DOUBLE_ASSIGN_NEW_X_V2: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import pack

slope = pack(ids.slope, PRIME)
x = pack(ids.point.x, PRIME)
y = pack(ids.point.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - 2 * x) % SECP_P"#;

pub const EC_DOUBLE_ASSIGN_NEW_Y: &str = r#"value = new_y = (slope * (x - new_x) - y) % SECP_P"#;

pub const SHA256_INPUT: &str = r#"ids.full_word = int(ids.n_bytes >= 4)"#;

pub const SHA256_MAIN: &str = r#"from starkware.cairo.common.cairo_sha256.sha256_utils import (
    IV, compute_message_schedule, sha2_compress_function)

_sha256_input_chunk_size_felts = int(ids.SHA256_INPUT_CHUNK_SIZE_FELTS)
assert 0 <= _sha256_input_chunk_size_felts < 100

w = compute_message_schedule(memory.get_range(
    ids.sha256_start, _sha256_input_chunk_size_felts))
new_state = sha2_compress_function(IV, w)
segments.write_arg(ids.output, new_state)"#;

pub const SHA256_FINALIZE: &str = r#"# Add dummy pairs of input and output.
from starkware.cairo.common.cairo_sha256.sha256_utils import (
    IV, compute_message_schedule, sha2_compress_function)

_block_size = int(ids.BLOCK_SIZE)
assert 0 <= _block_size < 20
_sha256_input_chunk_size_felts = int(ids.SHA256_INPUT_CHUNK_SIZE_FELTS)
assert 0 <= _sha256_input_chunk_size_felts < 100

message = [0] * _sha256_input_chunk_size_felts
w = compute_message_schedule(message)
output = sha2_compress_function(IV, w)
padding = (message + IV + output) * (_block_size - 1)
segments.write_arg(ids.sha256_ptr_end, padding)"#;

pub const KECCAK_WRITE_ARGS: &str = r#"segments.write_arg(ids.inputs, [ids.low % 2 ** 64, ids.low // 2 ** 64])
segments.write_arg(ids.inputs + 2, [ids.high % 2 ** 64, ids.high // 2 ** 64])"#;

pub const COMPARE_BYTES_IN_WORD_NONDET: &str =
    r#"memory[ap] = to_felt_or_relocatable(ids.n_bytes < ids.BYTES_IN_WORD)"#;

pub const COMPARE_KECCAK_FULL_RATE_IN_BYTES_NONDET: &str =
    r#"memory[ap] = to_felt_or_relocatable(ids.n_bytes >= ids.KECCAK_FULL_RATE_IN_BYTES)"#;

pub const BLOCK_PERMUTATION: &str = r#"from starkware.cairo.common.keccak_utils.keccak_utils import keccak_func
_keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
assert 0 <= _keccak_state_size_felts < 100

output_values = keccak_func(memory.get_range(
    ids.keccak_ptr - _keccak_state_size_felts, _keccak_state_size_felts))
segments.write_arg(ids.keccak_ptr, output_values)"#;

// The 0.10.3 whitelist uses this variant (instead of the one used by the common library), but both hints have the same behaviour
// We should check for future refactors that may discard one of the variants
pub const BLOCK_PERMUTATION_WHITELIST: &str = r#"from starkware.cairo.common.cairo_keccak.keccak_utils import keccak_func
_keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
assert 0 <= _keccak_state_size_felts < 100

output_values = keccak_func(memory.get_range(
    ids.keccak_ptr - _keccak_state_size_felts, _keccak_state_size_felts))
segments.write_arg(ids.keccak_ptr, output_values)"#;

pub const CAIRO_KECCAK_FINALIZE_V1: &str = r#"# Add dummy pairs of input and output.
_keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
_block_size = int(ids.BLOCK_SIZE)
assert 0 <= _keccak_state_size_felts < 100
assert 0 <= _block_size < 10
inp = [0] * _keccak_state_size_felts
padding = (inp + keccak_func(inp)) * _block_size
segments.write_arg(ids.keccak_ptr_end, padding)"#;

pub const CAIRO_KECCAK_FINALIZE_V2: &str = r#"# Add dummy pairs of input and output.
_keccak_state_size_felts = int(ids.KECCAK_STATE_SIZE_FELTS)
_block_size = int(ids.BLOCK_SIZE)
assert 0 <= _keccak_state_size_felts < 100
assert 0 <= _block_size < 1000
inp = [0] * _keccak_state_size_felts
padding = (inp + keccak_func(inp)) * _block_size
segments.write_arg(ids.keccak_ptr_end, padding)"#;

pub const FAST_EC_ADD_ASSIGN_NEW_X: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import SECP_P, pack

slope = pack(ids.slope, PRIME)
x0 = pack(ids.point0.x, PRIME)
x1 = pack(ids.point1.x, PRIME)
y0 = pack(ids.point0.y, PRIME)

value = new_x = (pow(slope, 2, SECP_P) - x0 - x1) % SECP_P"#;

pub const FAST_EC_ADD_ASSIGN_NEW_Y: &str =
    r#"value = new_y = (slope * (x0 - new_x) - y0) % SECP_P"#;

pub const EC_MUL_INNER: &str = r#"memory[ap] = (ids.scalar % PRIME) % 2"#;

pub const RELOCATE_SEGMENT: &str =
    r#"memory.add_relocation_rule(src_ptr=ids.src_ptr, dest_ptr=ids.dest_ptr)"#;

pub const TEMPORARY_ARRAY: &str = r#"ids.temporary_array = segments.add_temp_segment()"#;
pub const VERIFY_ECDSA_SIGNATURE: &str =
    r#"ecdsa_builtin.add_signature(ids.ecdsa_ptr.address_, (ids.signature_r, ids.signature_s))"#;

pub const SPLIT_OUTPUT_0: &str = "ids.output0_low = ids.output0 & ((1 << 128) - 1)
ids.output0_high = ids.output0 >> 128";
pub const SPLIT_OUTPUT_1: &str = "ids.output1_low = ids.output1 & ((1 << 128) - 1)
ids.output1_high = ids.output1 >> 128";

pub const SPLIT_INPUT_3: &str = "ids.high3, ids.low3 = divmod(memory[ids.inputs + 3], 256)";
pub const SPLIT_INPUT_6: &str = "ids.high6, ids.low6 = divmod(memory[ids.inputs + 6], 256 ** 2)";
pub const SPLIT_INPUT_9: &str = "ids.high9, ids.low9 = divmod(memory[ids.inputs + 9], 256 ** 3)";
pub const SPLIT_INPUT_12: &str =
    "ids.high12, ids.low12 = divmod(memory[ids.inputs + 12], 256 ** 4)";
pub const SPLIT_INPUT_15: &str =
    "ids.high15, ids.low15 = divmod(memory[ids.inputs + 15], 256 ** 5)";

pub const SPLIT_N_BYTES: &str =
    "ids.n_words_to_copy, ids.n_bytes_left = divmod(ids.n_bytes, ids.BYTES_IN_WORD)";
pub const SPLIT_OUTPUT_MID_LOW_HIGH: &str = "tmp, ids.output1_low = divmod(ids.output1, 256 ** 7)
ids.output1_high, ids.output1_mid = divmod(tmp, 2 ** 128)";

pub const NONDET_N_GREATER_THAN_10: &str = "memory[ap] = to_felt_or_relocatable(ids.n >= 10)";
pub const NONDET_N_GREATER_THAN_2: &str = "memory[ap] = to_felt_or_relocatable(ids.n >= 2)";
pub const RANDOM_EC_POINT: &str = r#"from starkware.crypto.signature.signature import ALPHA, BETA, FIELD_PRIME
from starkware.python.math_utils import random_ec_point
from starkware.python.utils import to_bytes

# Define a seed for random_ec_point that's dependent on all the input, so that:
#   (1) The added point s is deterministic.
#   (2) It's hard to choose inputs for which the builtin will fail.
seed = b"".join(map(to_bytes, [ids.p.x, ids.p.y, ids.m, ids.q.x, ids.q.y]))
ids.s.x, ids.s.y = random_ec_point(FIELD_PRIME, ALPHA, BETA, seed)"#;
pub const CHAINED_EC_OP_RANDOM_EC_POINT: &str = r#"from starkware.crypto.signature.signature import ALPHA, BETA, FIELD_PRIME
from starkware.python.math_utils import random_ec_point
from starkware.python.utils import to_bytes

n_elms = ids.len
assert isinstance(n_elms, int) and n_elms >= 0, \
    f'Invalid value for len. Got: {n_elms}.'
if '__chained_ec_op_max_len' in globals():
    assert n_elms <= __chained_ec_op_max_len, \
        f'chained_ec_op() can only be used with len<={__chained_ec_op_max_len}. ' \
        f'Got: n_elms={n_elms}.'

# Define a seed for random_ec_point that's dependent on all the input, so that:
#   (1) The added point s is deterministic.
#   (2) It's hard to choose inputs for which the builtin will fail.
seed = b"".join(
    map(
        to_bytes,
        [
            ids.p.x,
            ids.p.y,
            *memory.get_range(ids.m, n_elms),
            *memory.get_range(ids.q.address_, 2 * n_elms),
        ],
    )
)
ids.s.x, ids.s.y = random_ec_point(FIELD_PRIME, ALPHA, BETA, seed)"#;
pub const RECOVER_Y: &str =
    "from starkware.crypto.signature.signature import ALPHA, BETA, FIELD_PRIME
from starkware.python.math_utils import recover_y
ids.p.x = ids.x
# This raises an exception if `x` is not on the curve.
ids.p.y = recover_y(ids.x, ALPHA, BETA, FIELD_PRIME)";
pub(crate) const PACK_MODN_DIV_MODN: &str =
    "from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import div_mod, safe_div

N = 0xfffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141
x = pack(ids.x, PRIME) % N
s = pack(ids.s, PRIME) % N
value = res = div_mod(x, s, N)";
pub(crate) const XS_SAFE_DIV: &str = "value = k = safe_div(res * s - x, N)";

// The following hints support the lib https://github.com/NethermindEth/research-basic-Cairo-operations-big-integers/blob/main/lib
pub const UINT384_UNSIGNED_DIV_REM: &str = "def split(num: int, num_bits_shift: int, length: int):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift
    return tuple(a)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack(ids.a, num_bits_shift = 128)
div = pack(ids.div, num_bits_shift = 128)
quotient, remainder = divmod(a, div)

quotient_split = split(quotient, num_bits_shift=128, length=3)
assert len(quotient_split) == 3

ids.quotient.d0 = quotient_split[0]
ids.quotient.d1 = quotient_split[1]
ids.quotient.d2 = quotient_split[2]

remainder_split = split(remainder, num_bits_shift=128, length=3)
ids.remainder.d0 = remainder_split[0]
ids.remainder.d1 = remainder_split[1]
ids.remainder.d2 = remainder_split[2]";
pub const UINT384_SPLIT_128: &str = "ids.low = ids.a & ((1<<128) - 1)
ids.high = ids.a >> 128";
pub const ADD_NO_UINT384_CHECK: &str = "sum_d0 = ids.a.d0 + ids.b.d0
ids.carry_d0 = 1 if sum_d0 >= ids.SHIFT else 0
sum_d1 = ids.a.d1 + ids.b.d1 + ids.carry_d0
ids.carry_d1 = 1 if sum_d1 >= ids.SHIFT else 0
sum_d2 = ids.a.d2 + ids.b.d2 + ids.carry_d1
ids.carry_d2 = 1 if sum_d2 >= ids.SHIFT else 0";
pub const UINT384_UNSIGNED_DIV_REM_EXPANDED: &str =
    "def split(num: int, num_bits_shift: int, length: int):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift
    return tuple(a)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

def pack2(z, num_bits_shift: int) -> int:
    limbs = (z.b01, z.b23, z.b45)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack(ids.a, num_bits_shift = 128)
div = pack2(ids.div, num_bits_shift = 128)
quotient, remainder = divmod(a, div)

quotient_split = split(quotient, num_bits_shift=128, length=3)
assert len(quotient_split) == 3

ids.quotient.d0 = quotient_split[0]
ids.quotient.d1 = quotient_split[1]
ids.quotient.d2 = quotient_split[2]

remainder_split = split(remainder, num_bits_shift=128, length=3)
ids.remainder.d0 = remainder_split[0]
ids.remainder.d1 = remainder_split[1]
ids.remainder.d2 = remainder_split[2]";
pub const UINT384_SQRT: &str = "from starkware.python.math_utils import isqrt

def split(num: int, num_bits_shift: int, length: int):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift
    return tuple(a)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack(ids.a, num_bits_shift=128)
root = isqrt(a)
assert 0 <= root < 2 ** 192
root_split = split(root, num_bits_shift=128, length=3)
ids.root.d0 = root_split[0]
ids.root.d1 = root_split[1]
ids.root.d2 = root_split[2]";
pub const UNSIGNED_DIV_REM_UINT768_BY_UINT384: &str =
    "def split(num: int, num_bits_shift: int, length: int):
    a = []
    for _ in range(length):
        a.append( num & ((1 << num_bits_shift) - 1) )
        num = num >> num_bits_shift 
    return tuple(a)

def pack(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))
    
def pack_extended(z, num_bits_shift: int) -> int:
    limbs = (z.d0, z.d1, z.d2, z.d3, z.d4, z.d5)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

a = pack_extended(ids.a, num_bits_shift = 128)
div = pack(ids.div, num_bits_shift = 128)

quotient, remainder = divmod(a, div)

quotient_split = split(quotient, num_bits_shift=128, length=6)

ids.quotient.d0 = quotient_split[0]
ids.quotient.d1 = quotient_split[1]
ids.quotient.d2 = quotient_split[2]
ids.quotient.d3 = quotient_split[3]
ids.quotient.d4 = quotient_split[4]
ids.quotient.d5 = quotient_split[5]

remainder_split = split(remainder, num_bits_shift=128, length=3)
ids.remainder.d0 = remainder_split[0]
ids.remainder.d1 = remainder_split[1]
ids.remainder.d2 = remainder_split[2]";
pub const UINT384_SIGNED_NN: &str = "memory[ap] = 1 if 0 <= (ids.a.d2 % PRIME) < 2 ** 127 else 0";

pub(crate) const GET_SQUARE_ROOT: &str =
    "from starkware.python.math_utils import is_quad_residue, sqrt

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
ids.sqrt_gx.d2 = split_root_gx[2]";
pub const UINT384_DIV: &str = "from starkware.python.math_utils import div_mod

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
ids.b_inverse_mod_p.d2 = b_inverse_mod_p_split[2]";
pub const HI_MAX_BITLEN: &str =
    "ids.len_hi = max(ids.scalar_u.d2.bit_length(), ids.scalar_v.d2.bit_length())-1";

pub const QUAD_BIT: &str = r#"ids.quad_bit = (
    8 * ((ids.scalar_v >> ids.m) & 1)
    + 4 * ((ids.scalar_u >> ids.m) & 1)
    + 2 * ((ids.scalar_v >> (ids.m - 1)) & 1)
    + ((ids.scalar_u >> (ids.m - 1)) & 1)
)"#;

pub const INV_MOD_P_UINT512: &str = "def pack_512(u, num_bits_shift: int) -> int:
    limbs = (u.d0, u.d1, u.d2, u.d3)
    return sum(limb << (num_bits_shift * i) for i, limb in enumerate(limbs))

x = pack_512(ids.x, num_bits_shift = 128)
p = ids.p.low + (ids.p.high << 128)
x_inverse_mod_p = pow(x,-1, p) 

x_inverse_mod_p_split = (x_inverse_mod_p & ((1 << 128) - 1), x_inverse_mod_p >> 128)

ids.x_inverse_mod_p.low = x_inverse_mod_p_split[0]
ids.x_inverse_mod_p.high = x_inverse_mod_p_split[1]";

pub const DI_BIT: &str =
    r#"ids.dibit = ((ids.scalar_u >> ids.m) & 1) + 2 * ((ids.scalar_v >> ids.m) & 1)"#;

pub const EC_RECOVER_DIV_MOD_N_PACKED: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import div_mod, safe_div

N = pack(ids.n, PRIME)
x = pack(ids.x, PRIME) % N
s = pack(ids.s, PRIME) % N
value = res = div_mod(x, s, N)"#;

pub const UINT512_UNSIGNED_DIV_REM: &str = r#"def split(num: int, num_bits_shift: int, length: int):
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
ids.remainder.high = remainder_split[1]"#;

pub const EC_RECOVER_SUB_A_B: &str = r#"from starkware.cairo.common.cairo_secp.secp_utils import pack
from starkware.python.math_utils import div_mod, safe_div

a = pack(ids.a, PRIME)
b = pack(ids.b, PRIME)
value = res = a - b"#;

#[cfg(feature = "skip_next_instruction_hint")]
pub const SKIP_NEXT_INSTRUCTION: &str = "skip_next_instruction()";
