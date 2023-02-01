use std::prelude::v1::*;

use crate::{
    serde::deserialize_program::{
        deserialize_program, Attribute, HintParams, Identifier, InstructionLocation,
        ReferenceManager,
    },
    types::{errors::program_errors::ProgramError, relocatable::MaybeRelocatable},
};
use felt::{Felt, PRIME_STR};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, Read},
    {collections::HashMap, path::Path},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub builtins: Vec<String>,
    pub prime: String,
    pub data: Vec<MaybeRelocatable>,
    pub constants: HashMap<String, Felt>,
    pub main: Option<usize>,
    //start and end labels will only be used in proof-mode
    pub start: Option<usize>,
    pub end: Option<usize>,
    pub hints: HashMap<usize, Vec<HintParams>>,
    pub reference_manager: ReferenceManager,
    pub identifiers: HashMap<String, Identifier>,
    pub error_message_attributes: Vec<Attribute>,
    pub instruction_locations: Option<HashMap<usize, InstructionLocation>>,
}

impl Program {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        builtins: Vec<String>,
        prime: String,
        data: Vec<MaybeRelocatable>,
        main: Option<usize>,
        hints: HashMap<usize, Vec<HintParams>>,
        reference_manager: ReferenceManager,
        identifiers: HashMap<String, Identifier>,
        error_message_attributes: Vec<Attribute>,
        instruction_locations: Option<HashMap<usize, InstructionLocation>>,
    ) -> Result<Program, ProgramError> {
        Ok(Self {
            builtins,
            prime,
            data,
            constants: {
                let mut constants = HashMap::new();
                for (key, value) in identifiers.iter() {
                    if value.type_.as_deref() == Some("const") {
                        let value = value
                            .value
                            .clone()
                            .ok_or_else(|| ProgramError::ConstWithoutValue(key.to_owned()))?;
                        constants.insert(key.to_owned(), value);
                    }
                }

                constants
            },
            main,
            start: None,
            end: None,
            hints,
            reference_manager,
            identifiers,
            error_message_attributes,
            instruction_locations,
        })
    }

    pub fn from_file(path: &Path, entrypoint: Option<&str>) -> Result<Program, ProgramError> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        deserialize_program(reader, entrypoint)
    }

    pub fn from_reader(
        reader: impl Read,
        entrypoint: Option<&str>,
    ) -> Result<Program, ProgramError> {
        deserialize_program(reader, entrypoint)
    }
}

impl Default for Program {
    fn default() -> Self {
        Program {
            builtins: Vec::new(),
            prime: PRIME_STR.to_string(),
            data: Vec::new(),
            constants: HashMap::new(),
            main: None,
            start: None,
            end: None,
            hints: HashMap::new(),
            reference_manager: ReferenceManager {
                references: Vec::new(),
            },
            identifiers: HashMap::new(),
            error_message_attributes: Vec::new(),
            instruction_locations: None,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde::deserialize_program::{ApTracking, FlowTrackingData};
    use crate::utils::test_utils::mayberelocatable;
    use felt::felt_str;
    use num_traits::Zero;

    #[test]
    fn new() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<String> = Vec::new();
        let _r: MaybeRelocatable = mayberelocatable!(33);
        let data: Vec<MaybeRelocatable> = vec![
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(1000),
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(2000),
            mayberelocatable!(5201798304953696256),
            mayberelocatable!(2345108766317314046),
        ];

        let program = Program::new(
            builtins.clone(),
            felt::PRIME_STR.to_string(),
            data.clone(),
            None,
            HashMap::new(),
            reference_manager,
            HashMap::new(),
            Vec::new(),
            None,
        )
        .unwrap();

        assert_eq!(program.builtins, builtins);
        assert_eq!(program.data, data);
        assert_eq!(program.main, None);
        assert_eq!(program.identifiers, HashMap::new());
    }

    #[test]
    fn new_program_with_identifiers() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<String> = Vec::new();

        let data: Vec<MaybeRelocatable> = vec![
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(1000),
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(2000),
            mayberelocatable!(5201798304953696256),
            mayberelocatable!(2345108766317314046),
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
                cairo_type: None,
            },
        );

        identifiers.insert(
            String::from("__main__.main.SIZEOF_LOCALS"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(Felt::zero()),
                full_name: None,
                members: None,
                cairo_type: None,
            },
        );

        let program = Program::new(
            builtins.clone(),
            felt::PRIME_STR.to_string(),
            data.clone(),
            None,
            HashMap::new(),
            reference_manager,
            identifiers.clone(),
            Vec::new(),
            None,
        )
        .unwrap();

        assert_eq!(program.builtins, builtins);
        assert_eq!(program.data, data);
        assert_eq!(program.main, None);
        assert_eq!(program.identifiers, identifiers);
        assert_eq!(
            program.constants,
            [("__main__.main.SIZEOF_LOCALS", Felt::zero())]
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect::<HashMap<_, _>>(),
        );
    }

    #[test]
    fn new_program_with_invalid_identifiers() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<String> = Vec::new();

        let data: Vec<MaybeRelocatable> = vec![
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(1000),
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(2000),
            mayberelocatable!(5201798304953696256),
            mayberelocatable!(2345108766317314046),
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
                cairo_type: None,
            },
        );

        identifiers.insert(
            String::from("__main__.main.SIZEOF_LOCALS"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: None,
                full_name: None,
                members: None,
                cairo_type: None,
            },
        );

        let program = Program::new(
            builtins,
            felt::PRIME_STR.to_string(),
            data,
            None,
            HashMap::new(),
            reference_manager,
            identifiers.clone(),
            Vec::new(),
            None,
        );

        assert!(program.is_err());
    }

    #[test]
    fn deserialize_program_test() {
        let program: Program = Program::from_file(
            Path::new("cairo_programs/manually_compiled/valid_program_a.json"),
            Some("main"),
        )
        .expect("Failed to deserialize program");

        let builtins: Vec<String> = Vec::new();
        let data: Vec<MaybeRelocatable> = vec![
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(1000),
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(2000),
            mayberelocatable!(5201798304953696256),
            mayberelocatable!(2345108766317314046),
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
                cairo_type: None,
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
                cairo_type: None,
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
                cairo_type: None,
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
                cairo_type: None,
            },
        );
        identifiers.insert(
            String::from("__main__.main.SIZEOF_LOCALS"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(Felt::zero()),
                full_name: None,
                members: None,
                cairo_type: None,
            },
        );

        assert_eq!(program.prime, PRIME_STR.to_string());
        assert_eq!(program.builtins, builtins);
        assert_eq!(program.data, data);
        assert_eq!(program.main, Some(0));
        assert_eq!(program.identifiers, identifiers);
    }

    /// Deserialize a program without an entrypoint.
    #[test]
    fn deserialize_program_without_entrypoint_test() {
        let program: Program = Program::from_file(
            Path::new("cairo_programs/manually_compiled/valid_program_a.json"),
            None,
        )
        .expect("Failed to deserialize program");

        let builtins: Vec<String> = Vec::new();

        let error_message_attributes: Vec<Attribute> = vec![Attribute {
            name: String::from("error_message"),
            start_pc: 379,
            end_pc: 381,
            value: String::from("SafeUint256: addition overflow"),
            flow_tracking_data: Some(FlowTrackingData {
                ap_tracking: ApTracking {
                    group: 14,
                    offset: 35,
                },
                reference_ids: HashMap::new(),
            }),
        }];

        let data: Vec<MaybeRelocatable> = vec![
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(1000),
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(2000),
            mayberelocatable!(5201798304953696256),
            mayberelocatable!(2345108766317314046),
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
                cairo_type: None,
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
                cairo_type: None,
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
                cairo_type: None,
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
                cairo_type: None,
            },
        );
        identifiers.insert(
            String::from("__main__.main.SIZEOF_LOCALS"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(Felt::zero()),
                full_name: None,
                members: None,
                cairo_type: None,
            },
        );

        assert_eq!(program.prime, PRIME_STR.to_string());
        assert_eq!(program.builtins, builtins);
        assert_eq!(program.data, data);
        assert_eq!(program.main, None);
        assert_eq!(program.identifiers, identifiers);
        assert_eq!(program.error_message_attributes, error_message_attributes)
    }

    #[test]
    fn deserialize_program_constants_test() {
        let program = Program::from_file(
            Path::new("cairo_programs/manually_compiled/deserialize_constant_test.json"),
            Some("main"),
        )
        .expect("Failed to deserialize program");

        let constants = [
            ("__main__.compare_abs_arrays.SIZEOF_LOCALS", Felt::zero()),
            (
                "starkware.cairo.common.cairo_keccak.packed_keccak.ALL_ONES",
                felt_str!(
                    "3618502788666131106986593281521497120414687020801267626233049500247285301247"
                ),
            ),
            (
                "starkware.cairo.common.cairo_keccak.packed_keccak.BLOCK_SIZE",
                Felt::new(3),
            ),
            (
                "starkware.cairo.common.alloc.alloc.SIZEOF_LOCALS",
                felt_str!(
                    "-3618502788666131213697322783095070105623107215331596699973092056135872020481"
                ),
            ),
            (
                "starkware.cairo.common.uint256.SHIFT",
                felt_str!("340282366920938463463374607431768211456"),
            ),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect::<HashMap<_, _>>();

        assert_eq!(program.constants, constants);
    }

    #[test]
    fn default_program() {
        let program = Program {
            builtins: Vec::new(),
            prime: PRIME_STR.to_string(),
            data: Vec::new(),
            constants: HashMap::new(),
            main: None,
            start: None,
            end: None,
            hints: HashMap::new(),
            reference_manager: ReferenceManager {
                references: Vec::new(),
            },
            identifiers: HashMap::new(),
            error_message_attributes: Vec::new(),
            instruction_locations: None,
        };

        assert_eq!(program, Program::default())
    }
}
