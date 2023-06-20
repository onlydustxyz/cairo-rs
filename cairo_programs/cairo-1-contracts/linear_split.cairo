#[starknet::contract]
mod LinearSplit {
    use integer::u16_try_from_felt252;

    #[storage]
    struct Storage {}

    #[external(v0)]
    fn cast(ref self: ContractState, a: felt252) -> Option<u16> {
        u16_try_from_felt252(a)
    }
}
