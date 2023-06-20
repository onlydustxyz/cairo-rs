#[starknet::contract]
mod TestDict {
    use dict::Felt252DictTrait;
    use nullable::NullableTrait;
    use traits::Index;

    #[storage]
    struct Storage {}

    #[external(v0)]
    fn test_dict_init(ref self: ContractState, test_value: felt252) -> felt252 {
        let mut dict = felt252_dict_new::<felt252>();

        dict.insert(10, test_value);
        let (entry, value) = dict.entry(10);
        assert(value == test_value, 'dict[10] == test_value');

        return test_value;
    }
}
