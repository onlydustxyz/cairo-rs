#[starknet::contract]
mod Factorial {
    #[storage]
    struct Storage {}

    #[external(v0)]
    fn factorial(ref self: ContractState, n: felt252) -> felt252 {
        if (n == 0) {
            return 1;
        }
        n * factorial(ref self, n - 1)
    }
}
