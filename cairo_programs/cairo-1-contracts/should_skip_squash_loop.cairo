#[starknet::contract]
mod ShouldSkipSquashLoop {
    use dict::Felt252DictTrait;

    #[storage]
    struct Storage {}

    #[external(v0)]
    fn should_skip_squash_loop(ref self: ContractState) {
        let x = felt252_dict_new::<felt252>();
        x.squash();
    }
}
