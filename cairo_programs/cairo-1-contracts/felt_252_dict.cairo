#[starknet::contract]
mod Felt252Dict {
    use dict::{felt252_dict_entry_finalize, Felt252DictTrait};

    #[storage]
    struct Storage {}

    /// An external method that requires the `segment_arena` builtin.
    #[external(v0)]
    fn squash_empty_dict(ref self: ContractState) -> bool {
        let x = felt252_dict_new::<felt252>();
        x.squash();
        return true;
    }
}
