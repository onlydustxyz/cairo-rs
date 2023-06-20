#[starknet::contract]
mod U8Sqrt {
    use integer::u8_sqrt;
    use core::traits::Into;
    use traits::TryInto;
    use option::OptionTrait;

    #[storage]
    struct Storage {}

    #[external(v0)]
    fn sqrt(ref self: ContractState, num: felt252) -> felt252 {
        let num_in_u8: u8 = num.try_into().unwrap();
        let a: u8 = u8_sqrt(num_in_u8);
        let to_return: felt252 = a.into();
        to_return
    }
}
