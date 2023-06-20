#[starknet::contract]
mod U32Sqrt {
    use integer::u32_sqrt;
    use core::traits::Into;
    use traits::TryInto;
    use option::OptionTrait;

    #[storage]
    struct Storage {}

    #[external(v0)]
    fn sqrt(ref self: ContractState, num: felt252) -> felt252 {
        let num_in_u32: u32 = num.try_into().unwrap();
        let a: u16 = u32_sqrt(num_in_u32);
        let to_return: felt252 = a.into();
        to_return
    }
}
