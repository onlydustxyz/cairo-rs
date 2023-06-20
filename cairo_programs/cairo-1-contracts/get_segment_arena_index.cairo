#[starknet::contract]
mod SegmentArenaIndex {
    use dict::Felt252DictTrait;

    #[storage]
    struct Storage {}

    #[external(v0)]
    fn test_arena_index(ref self: ContractState) -> bool {
        let mut dict = felt252_dict_new::<felt252>();
        let squashed_dict = dict.squash();
        return true;
    }
}
