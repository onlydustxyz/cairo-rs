use crate::serde::deserialize_program::{
    deserialize_from_program_json, deserialize_program, HintParams, Identifier, ProgramJson,
    ReferenceManager,
};
use crate::types::errors::program_errors::ProgramError;
use crate::types::relocatable::MaybeRelocatable;
use num_bigint::BigInt;
use std::{collections::HashMap, path::Path};

#[derive(Clone)]
pub struct Program {
    pub builtins: Vec<String>,
    pub prime: BigInt,
    pub data: Vec<MaybeRelocatable>,
    pub constants: HashMap<String, BigInt>,
    pub main: Option<usize>,
    pub hints: HashMap<usize, Vec<HintParams>>,
    pub reference_manager: ReferenceManager,
    pub identifiers: HashMap<String, Identifier>,
}

impl Program {
    pub fn new(path: &Path, entrypoint: &str) -> Result<Program, ProgramError> {
        deserialize_program(path, entrypoint)
    }
    pub fn from_json(program: ProgramJson, entrypoint: &str) -> Result<Program, ProgramError> {
        deserialize_from_program_json(program, entrypoint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde::deserialize_program::deserialize_program_json;
    use crate::{bigint, bigint_str};
    use num_traits::FromPrimitive;

    #[test]
    fn deserialize_program_from_json_test() {
        let program_json = deserialize_program_json(Path::new(
            "cairo_programs/manually_compiled/valid_program_a.json",
        )).unwrap();
        let program = Program::from_json(program_json, "main").unwrap();

        test_deserialized_program(program);
    }

    #[test]
    fn deserialize_program_test() {
        let program: Program = Program::new(
            Path::new("cairo_programs/manually_compiled/valid_program_a.json"),
            "main",
        )
        .expect("Failed to deserialize program");

        test_deserialized_program(program);
    }

    fn test_deserialized_program(program: Program) {
        let builtins: Vec<String> = Vec::new();
        let data: Vec<MaybeRelocatable> = vec![
            MaybeRelocatable::Int(BigInt::from_i64(5189976364521848832).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(1000).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(5189976364521848832).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(2000).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(5201798304953696256).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(2345108766317314046).unwrap()),
        ];

        let mut identifiers: HashMap<String, Identifier> = HashMap::new();

        identifiers.insert(
            String::from("__main__.main"),
            Identifier {
                pc: Some(0),
                type_: Some(String::from("function")),
                value: None,
                full_name: None,
                members: None,
            },
        );
        identifiers.insert(
            String::from("__main__.main.Args"),
            Identifier {
                pc: None,
                type_: Some(String::from("struct")),
                value: None,
                full_name: Some("__main__.main.Args".to_string()),
                members: Some(HashMap::new()),
            },
        );
        identifiers.insert(
            String::from("__main__.main.ImplicitArgs"),
            Identifier {
                pc: None,
                type_: Some(String::from("struct")),
                value: None,
                full_name: Some("__main__.main.ImplicitArgs".to_string()),
                members: Some(HashMap::new()),
            },
        );
        identifiers.insert(
            String::from("__main__.main.Return"),
            Identifier {
                pc: None,
                type_: Some(String::from("struct")),
                value: None,
                full_name: Some("__main__.main.Return".to_string()),
                members: Some(HashMap::new()),
            },
        );
        identifiers.insert(
            String::from("__main__.main.SIZEOF_LOCALS"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(bigint!(0)),
                full_name: None,
                members: None,
            },
        );

        assert_eq!(
            program.prime,
            BigInt::parse_bytes(
                b"3618502788666131213697322783095070105623107215331596699973092056135872020481",
                10
            )
            .unwrap()
        );
        assert_eq!(program.builtins, builtins);
        assert_eq!(program.data, data);
        assert_eq!(program.main, Some(0));
        assert_eq!(program.identifiers, identifiers);
    }

    #[test]
    fn deserialize_program_constants_test() {
        let program = Program::new(
            Path::new("cairo_programs/manually_compiled/deserialize_constant_test.json"),
            "main",
        )
        .expect("Failed to deserialize program");

        let constants = [
            (
                "__main__.compare_abs_arrays.SIZEOF_LOCALS",
                bigint_str!(
                    b"-3618502788666131213697322783095070105623107215331596699973092056135872020481"
                ),
            ),
            (
                "starkware.cairo.common.cairo_keccak.packed_keccak.ALL_ONES",
                bigint_str!(b"-106710729501573572985208420194530329073740042555888586719234"),
            ),
            (
                "starkware.cairo.common.cairo_keccak.packed_keccak.BLOCK_SIZE",
                bigint!(3),
            ),
            (
                "starkware.cairo.common.alloc.alloc.SIZEOF_LOCALS",
                bigint!(0),
            ),
            (
                "starkware.cairo.common.uint256.SHIFT",
                bigint_str!(b"340282366920938463463374607431768211456"),
            ),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect::<HashMap<_, _>>();

        assert_eq!(program.constants, constants);
    }
}
