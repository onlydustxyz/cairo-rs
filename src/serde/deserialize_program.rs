use crate::serde::deserialize_utils;
use crate::types::instruction::Register;
use crate::types::{
    errors::program_errors::ProgramError, program::Program, relocatable::MaybeRelocatable,
};
use num_bigint::{BigInt, Sign};
use serde::{de, de::MapAccess, de::SeqAccess, Deserialize, Deserializer};
use serde_json::Number;
use std::io::Read;
use std::{collections::HashMap, fmt};

#[derive(Deserialize, Debug)]
pub struct ProgramJson {
    #[serde(deserialize_with = "deserialize_bigint_hex")]
    pub prime: BigInt,
    pub builtins: Vec<String>,
    #[serde(deserialize_with = "deserialize_array_of_bigint_hex")]
    pub data: Vec<MaybeRelocatable>,
    pub identifiers: HashMap<String, Identifier>,
    pub hints: HashMap<usize, Vec<HintParams>>,
    pub reference_manager: ReferenceManager,
    pub attributes: Vec<Attribute>,
    pub debug_info: Option<DebugInfo>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct HintParams {
    pub code: String,
    pub accessible_scopes: Vec<String>,
    pub flow_tracking_data: FlowTrackingData,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlowTrackingData {
    pub ap_tracking: ApTracking,
    #[serde(deserialize_with = "deserialize_map_to_string_and_usize_hashmap")]
    pub reference_ids: HashMap<String, usize>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ApTracking {
    pub group: usize,
    pub offset: usize,
}

impl ApTracking {
    pub fn new() -> ApTracking {
        ApTracking {
            group: 0,
            offset: 0,
        }
    }
}

impl Default for ApTracking {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Identifier {
    pub pc: Option<usize>,
    #[serde(rename(deserialize = "type"))]
    pub type_: Option<String>,
    #[serde(default)]
    #[serde(deserialize_with = "bigint_from_number")]
    pub value: Option<BigInt>,

    pub full_name: Option<String>,
    pub members: Option<HashMap<String, Member>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Member {
    pub cairo_type: String,
    pub offset: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub start_pc: usize,
    pub end_pc: usize,
    pub value: String,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Location {
    pub end_line: u32,
    pub end_col: u32,
    pub input_file: InputFile,
    pub parent_location: Option<(Box<Location>, String)>,
    pub start_line: u32,
    pub start_col: u32,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct DebugInfo {
    instruction_locations: HashMap<usize, InstructionLocation>,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct InstructionLocation {
    pub inst: Location,
    pub hints: Vec<HintLocation>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct InputFile {
    pub filename: String,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct HintLocation {
    pub location: Location,
    pub n_prefix_newlines: u32,
}

fn bigint_from_number<'de, D>(deserializer: D) -> Result<Option<BigInt>, D::Error>
where
    D: Deserializer<'de>,
{
    let n = Number::deserialize(deserializer)?;
    Ok(BigInt::parse_bytes(n.to_string().as_bytes(), 10))
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ReferenceManager {
    pub references: Vec<Reference>,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Reference {
    pub ap_tracking_data: ApTracking,
    pub pc: Option<usize>,
    #[serde(deserialize_with = "deserialize_value_address")]
    #[serde(rename(deserialize = "value"))]
    pub value_address: ValueAddress,
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]
pub enum OffsetValue {
    Immediate(BigInt),
    Value(i32),
    Reference(Register, i32, bool),
}

#[derive(Deserialize, Debug, PartialEq, Eq, Clone)]

pub struct ValueAddress {
    pub offset1: OffsetValue,
    pub offset2: OffsetValue,
    pub dereference: bool,
    pub value_type: String,
}

impl ValueAddress {
    // The parsing functionality is focused on the string formats that appear in the
    // references used by hints. Errors may occur when parsing references not used by hints.
    // When this happens, this default ValueAddress is returned to make explicit that the value was not
    // parsed correctly.
    // In case an incorrectly parsed reference is used by a hint, an error will be raised (IdNotFound) in the
    // get_address_from_reference function call to notify this, and the parsing functionality should be
    // extended to contemplate this new case.
    pub fn no_hint_reference_default() -> ValueAddress {
        ValueAddress {
            offset1: OffsetValue::Value(99),
            offset2: OffsetValue::Value(99),
            dereference: false,
            value_type: String::from("felt"),
        }
    }
}

struct BigIntVisitor;

impl<'de> de::Visitor<'de> for BigIntVisitor {
    type Value = BigInt;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Could not deserialize hexadecimal string")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        // Strip the '0x' prefix from the encoded hex string
        if let Some(no_prefix_hex) = value.strip_prefix("0x") {
            // Add padding if necessary
            let no_prefix_hex = deserialize_utils::maybe_add_padding(no_prefix_hex.to_string());
            let decoded_result: Result<Vec<u8>, hex::FromHexError> = hex::decode(&no_prefix_hex);

            match decoded_result {
                Ok(decoded_hex) => Ok(BigInt::from_bytes_be(Sign::Plus, &decoded_hex)),
                Err(e) => Err(e).map_err(de::Error::custom),
            }
        } else {
            Err(String::from("hex prefix error")).map_err(de::Error::custom)
        }
    }
}

struct MaybeRelocatableVisitor;

impl<'de> de::Visitor<'de> for MaybeRelocatableVisitor {
    type Value = Vec<MaybeRelocatable>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Could not deserialize array of hexadecimal")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut data: Vec<MaybeRelocatable> = vec![];

        while let Some(value) = seq.next_element::<String>()? {
            if let Some(no_prefix_hex) = value.strip_prefix("0x") {
                // Add padding if necessary
                let no_prefix_hex = deserialize_utils::maybe_add_padding(no_prefix_hex.to_string());
                let decoded_result: Result<Vec<u8>, hex::FromHexError> =
                    hex::decode(&no_prefix_hex);

                match decoded_result {
                    Ok(decoded_hex) => data.push(MaybeRelocatable::Int(BigInt::from_bytes_be(
                        Sign::Plus,
                        &decoded_hex,
                    ))),
                    Err(e) => return Err(e).map_err(de::Error::custom),
                };
            } else {
                return Err(String::from("hex prefix error")).map_err(de::Error::custom);
            };
        }
        Ok(data)
    }
}

struct ReferenceIdsVisitor;

impl<'de> de::Visitor<'de> for ReferenceIdsVisitor {
    type Value = HashMap<String, usize>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map with string keys and integer values")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut data: HashMap<String, usize> = HashMap::new();

        while let Some((key, value)) = map.next_entry::<String, usize>()? {
            data.insert(key, value);
        }

        Ok(data)
    }
}

struct ValueAddressVisitor;

impl<'de> de::Visitor<'de> for ValueAddressVisitor {
    type Value = ValueAddress;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string representing the address in memory of a variable")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let parse_res = deserialize_utils::parse_value(value);

        if let Ok((_, res)) = parse_res {
            return Ok(res);
        }

        Ok(ValueAddress::no_hint_reference_default())
    }
}

pub fn deserialize_bigint_hex<'de, D: Deserializer<'de>>(d: D) -> Result<BigInt, D::Error> {
    d.deserialize_str(BigIntVisitor)
}

pub fn deserialize_array_of_bigint_hex<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Vec<MaybeRelocatable>, D::Error> {
    d.deserialize_seq(MaybeRelocatableVisitor)
}

pub fn deserialize_map_to_string_and_usize_hashmap<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<HashMap<String, usize>, D::Error> {
    d.deserialize_map(ReferenceIdsVisitor)
}

pub fn deserialize_value_address<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<ValueAddress, D::Error> {
    d.deserialize_str(ValueAddressVisitor)
}

pub fn deserialize_program_json(reader: impl Read) -> Result<ProgramJson, ProgramError> {
    let program_json = serde_json::from_reader(reader)?;

    Ok(program_json)
}

pub fn deserialize_program(
    reader: impl Read,
    entrypoint: Option<&str>,
) -> Result<Program, ProgramError> {
    let program_json: ProgramJson = deserialize_program_json(reader)?;

    let entrypoint_pc = match entrypoint {
        Some(entrypoint) => match program_json
            .identifiers
            .get(&format!("__main__.{entrypoint}"))
        {
            Some(entrypoint_identifier) => entrypoint_identifier.pc,
            None => return Err(ProgramError::EntrypointNotFound(entrypoint.to_string())),
        },
        None => None,
    };

    let start = match program_json.identifiers.get("__main__.__start__") {
        Some(identifier) => identifier.pc,
        None => None,
    };
    let end = match program_json.identifiers.get("__main__.__end__") {
        Some(identifier) => identifier.pc,
        None => None,
    };

    Ok(Program {
        builtins: program_json.builtins,
        prime: program_json.prime,
        data: program_json.data,
        constants: {
            let mut constants = HashMap::new();
            for (key, value) in program_json.identifiers.iter() {
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
        main: entrypoint_pc,
        start,
        end,
        hints: program_json.hints,
        reference_manager: program_json.reference_manager,
        identifiers: program_json.identifiers,
        error_message_attributes: program_json
            .attributes
            .into_iter()
            .filter(|attr| attr.name == "error_message")
            .collect(),
        instruction_locations: program_json
            .debug_info
            .map(|debug_info| debug_info.instruction_locations),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{bigint, bigint_str};
    use num_traits::FromPrimitive;
    use std::{fs::File, io::BufReader};

    #[test]
    fn deserialize_bigint_from_string_json_gives_error() {
        let invalid_even_length_hex_json = r#"
            {
                "prime": "0bx000A"
            }"#;

        // ProgramJson result instance for the json with an even length encoded hex.
        let even_result: Result<ProgramJson, _> =
            serde_json::from_str(invalid_even_length_hex_json);

        assert!(even_result.is_err());

        let invalid_odd_length_hex_json = r#"
            {
                "prime": "0bx00A"
            }"#;

        // ProgramJson result instance for the json with an odd length encoded hex.
        let odd_result: Result<ProgramJson, _> = serde_json::from_str(invalid_odd_length_hex_json);

        assert!(odd_result.is_err());
    }

    #[test]
    fn deserialize_bigint_invalid_char_error() {
        let invalid_char = r#"
            {
                "prime": "0xlambda"
            }"#;

        let invalid_char_error: Result<ProgramJson, _> = serde_json::from_str(invalid_char);

        assert!(invalid_char_error.is_err());
    }

    #[test]
    fn deserialize_bigint_no_prefix_error() {
        let no_prefix = r#"
            {
                "prime": "00A"
            }"#;

        // ProgramJson result instance for the json with an odd length encoded hex.
        let no_prefix_error: Result<ProgramJson, _> = serde_json::from_str(no_prefix);

        assert!(no_prefix_error.is_err());
    }

    #[test]
    fn deserialize_from_string_json() {
        let valid_json = r#"
            {
                "prime": "0x000A",
                "attributes": [],
                "debug_info": {
                    "instruction_locations": {}
                }, 
                "builtins": [],
                "data": [
                    "0x480680017fff8000",
                    "0x3e8",
                    "0x480680017fff8000",
                    "0x7d0",
                    "0x48307fff7ffe8000",
                    "0x208b7fff7fff7ffe"
                ],
                "identifiers": {
                    "__main__.main": {
                        "decorators": [],
                        "pc": 0,
                        "type": "function"
                    },
                    "__main__.main.Args": {
                        "full_name": "__main__.main.Args",
                        "members": {},
                        "size": 0,
                        "type": "struct"
                    },
                    "__main__.main.ImplicitArgs": {
                        "full_name": "__main__.main.ImplicitArgs",
                        "members": {},
                        "size": 0,
                        "type": "struct"
                    }
                },
                "hints": {
                    "0": [
                        {
                            "accessible_scopes": [
                                "starkware.cairo.common.alloc",
                                "starkware.cairo.common.alloc.alloc"
                            ],
                            "code": "memory[ap] = segments.add()",
                            "flow_tracking_data": {
                                "ap_tracking": {
                                    "group": 0,
                                    "offset": 0
                                },
                                "reference_ids": {
                                    "starkware.cairo.common.math.split_felt.high": 0,
                                    "starkware.cairo.common.math.split_felt.low": 14,
                                    "starkware.cairo.common.math.split_felt.range_check_ptr": 16,
                                    "starkware.cairo.common.math.split_felt.value": 12
                                }
                            }
                        }
                    ]
                },
                "reference_manager": {
                    "references": [
                        {
                            "ap_tracking_data": {
                                "group": 0,
                                "offset": 0
                            },
                            "pc": 0,
                            "value": "[cast(fp + (-4), felt*)]"
                        },
                        {
                            "ap_tracking_data": {
                                "group": 0,
                                "offset": 0
                            },
                            "pc": 0,
                            "value": "[cast(fp + (-3), felt*)]"
                        },
                        {
                            "ap_tracking_data": {
                                "group": 0,
                                "offset": 0
                            },
                            "pc": 0,
                            "value": "cast([fp + (-3)] + 2, felt)"
                        },
                        {
                            "ap_tracking_data": {
                                "group": 0,
                                "offset": 0
                            },
                            "pc": 0,
                            "value": "[cast(fp, felt**)]"
                        }
                    ]
                }
            }"#;

        // ProgramJson instance for the json with an even length encoded hex.
        let program_json: ProgramJson = serde_json::from_str(valid_json).unwrap();

        let builtins: Vec<String> = Vec::new();

        let data: Vec<MaybeRelocatable> = vec![
            MaybeRelocatable::Int(BigInt::from_i64(5189976364521848832).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(1000).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(5189976364521848832).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(2000).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(5201798304953696256).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(2345108766317314046).unwrap()),
        ];

        let mut hints: HashMap<usize, Vec<HintParams>> = HashMap::new();
        hints.insert(
            0,
            vec![HintParams {
                code: "memory[ap] = segments.add()".to_string(),
                accessible_scopes: vec![
                    String::from("starkware.cairo.common.alloc"),
                    String::from("starkware.cairo.common.alloc.alloc"),
                ],
                flow_tracking_data: FlowTrackingData {
                    ap_tracking: ApTracking {
                        group: 0,
                        offset: 0,
                    },
                    reference_ids: HashMap::from([
                        (
                            String::from("starkware.cairo.common.math.split_felt.high"),
                            0,
                        ),
                        (
                            String::from("starkware.cairo.common.math.split_felt.low"),
                            14,
                        ),
                        (
                            String::from("starkware.cairo.common.math.split_felt.range_check_ptr"),
                            16,
                        ),
                        (
                            String::from("starkware.cairo.common.math.split_felt.value"),
                            12,
                        ),
                    ]),
                },
            }],
        );

        let reference_manager = ReferenceManager {
            references: vec![
                Reference {
                    ap_tracking_data: ApTracking {
                        group: 0,
                        offset: 0,
                    },
                    pc: Some(0),
                    value_address: ValueAddress {
                        offset1: OffsetValue::Reference(Register::FP, -4, false),
                        offset2: OffsetValue::Value(0),
                        dereference: true,
                        value_type: "felt".to_string(),
                    },
                },
                Reference {
                    ap_tracking_data: ApTracking {
                        group: 0,
                        offset: 0,
                    },
                    pc: Some(0),
                    value_address: ValueAddress {
                        offset1: OffsetValue::Reference(Register::FP, -3, false),
                        offset2: OffsetValue::Value(0),
                        dereference: true,
                        value_type: "felt".to_string(),
                    },
                },
                Reference {
                    ap_tracking_data: ApTracking {
                        group: 0,
                        offset: 0,
                    },
                    pc: Some(0),
                    value_address: ValueAddress {
                        offset1: OffsetValue::Reference(Register::FP, -3, true),
                        offset2: OffsetValue::Immediate(bigint!(2)),
                        dereference: false,
                        value_type: "felt".to_string(),
                    },
                },
                Reference {
                    ap_tracking_data: ApTracking {
                        group: 0,
                        offset: 0,
                    },
                    pc: Some(0),
                    value_address: ValueAddress {
                        offset1: OffsetValue::Reference(Register::FP, 0, false),
                        offset2: OffsetValue::Value(0),
                        dereference: true,
                        value_type: "felt*".to_string(),
                    },
                },
            ],
        };

        assert_eq!(program_json.prime, bigint!(10));
        assert_eq!(program_json.builtins, builtins);
        assert_eq!(program_json.data, data);
        assert_eq!(program_json.identifiers["__main__.main"].pc, Some(0));
        assert_eq!(program_json.hints, hints);
        assert_eq!(program_json.reference_manager, reference_manager);
    }

    #[test]
    fn deserialize_program_json_from_json_file_a() {
        // Open json file with (valid) even length encoded hex
        let file = File::open("cairo_programs/manually_compiled/valid_program_a.json").unwrap();
        let mut reader = BufReader::new(file);

        let program_json: ProgramJson = serde_json::from_reader(&mut reader).unwrap();
        let builtins: Vec<String> = Vec::new();

        assert_eq!(
            program_json.prime,
            BigInt::parse_bytes(
                b"3618502788666131213697322783095070105623107215331596699973092056135872020481",
                10
            )
            .unwrap()
        );
        assert_eq!(program_json.builtins, builtins);
        assert_eq!(program_json.data.len(), 6);
        assert_eq!(program_json.identifiers["__main__.main"].pc, Some(0));
    }

    #[test]
    fn deserialize_program_json_from_json_file_b() {
        // Open json file with (valid) odd length encoded hex
        let file = File::open("cairo_programs/manually_compiled/valid_program_b.json").unwrap();
        let mut reader = BufReader::new(file);

        let program_json: ProgramJson = serde_json::from_reader(&mut reader).unwrap();
        let builtins: Vec<String> = vec![String::from("output"), String::from("range_check")];

        assert_eq!(
            program_json.prime,
            BigInt::parse_bytes(
                b"3618502788666131213697322783095070105623107215331596699973092056135872020481",
                10
            )
            .unwrap()
        );
        assert_eq!(program_json.builtins, builtins);
        assert_eq!(program_json.data.len(), 24);
        assert_eq!(program_json.identifiers["__main__.main"].pc, Some(13));
    }

    #[test]
    fn deserialize_program_json_from_json_file_gives_error() {
        // Open json file with (invalid) even length encoded hex
        let even_length_file =
            File::open("cairo_programs/manually_compiled/invalid_even_length_hex.json").unwrap();
        let mut reader = BufReader::new(even_length_file);

        let even_result: Result<ProgramJson, _> = serde_json::from_reader(&mut reader);

        assert!(even_result.is_err());

        // Open json file with (invalid) odd length encoded hex
        let odd_length_file =
            File::open("cairo_programs/manually_compiled/invalid_odd_length_hex.json").unwrap();
        let mut reader = BufReader::new(odd_length_file);

        let odd_result: Result<ProgramJson, _> = serde_json::from_reader(&mut reader);

        assert!(odd_result.is_err());
    }

    #[test]
    fn deserialize_missing_entrypoint_gives_error() {
        let even_length_file =
            File::open("cairo_programs/manually_compiled/valid_program_a.json").unwrap();
        let reader = BufReader::new(even_length_file);

        let deserialization_result = deserialize_program(reader, Some("missing_function"));
        assert!(deserialization_result.is_err());
        assert!(matches!(
            deserialization_result,
            Err(ProgramError::EntrypointNotFound(_))
        ));
    }

    #[test]
    fn deserialize_program_test() {
        let even_length_file =
            File::open("cairo_programs/manually_compiled/valid_program_a.json").unwrap();
        let reader = BufReader::new(even_length_file);

        let program: Program =
            deserialize_program(reader, Some("main")).expect("Failed to deserialize program");

        let builtins: Vec<String> = Vec::new();
        let data: Vec<MaybeRelocatable> = vec![
            MaybeRelocatable::Int(BigInt::from_i64(5189976364521848832).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(1000).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(5189976364521848832).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(2000).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(5201798304953696256).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(2345108766317314046).unwrap()),
        ];

        let mut hints: HashMap<usize, Vec<HintParams>> = HashMap::new();
        hints.insert(
            0,
            vec![HintParams {
                code: "memory[ap] = segments.add()".to_string(),
                accessible_scopes: vec![
                    String::from("starkware.cairo.common.alloc"),
                    String::from("starkware.cairo.common.alloc.alloc"),
                ],
                flow_tracking_data: FlowTrackingData {
                    ap_tracking: ApTracking {
                        group: 0,
                        offset: 0,
                    },
                    reference_ids: HashMap::new(),
                },
            }],
        );
        hints.insert(
            46,
            vec![HintParams {
                code: "import math".to_string(),
                accessible_scopes: vec![String::from("__main__"), String::from("__main__.main")],
                flow_tracking_data: FlowTrackingData {
                    ap_tracking: ApTracking {
                        group: 5,
                        offset: 0,
                    },
                    reference_ids: HashMap::new(),
                },
            }],
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
        assert_eq!(program.hints, hints);
    }

    /// Deserialize a program without an entrypoint.
    #[test]
    fn deserialize_program_without_entrypoint_test() {
        let even_length_file =
            File::open("cairo_programs/manually_compiled/valid_program_a.json").unwrap();
        let reader = BufReader::new(even_length_file);

        let program: Program =
            deserialize_program(reader, None).expect("Failed to deserialize program");

        let builtins: Vec<String> = Vec::new();
        let data: Vec<MaybeRelocatable> = vec![
            MaybeRelocatable::Int(BigInt::from_i64(5189976364521848832).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(1000).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(5189976364521848832).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(2000).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(5201798304953696256).unwrap()),
            MaybeRelocatable::Int(BigInt::from_i64(2345108766317314046).unwrap()),
        ];

        let mut hints: HashMap<usize, Vec<HintParams>> = HashMap::new();
        hints.insert(
            0,
            vec![HintParams {
                code: "memory[ap] = segments.add()".to_string(),
                accessible_scopes: vec![
                    String::from("starkware.cairo.common.alloc"),
                    String::from("starkware.cairo.common.alloc.alloc"),
                ],
                flow_tracking_data: FlowTrackingData {
                    ap_tracking: ApTracking {
                        group: 0,
                        offset: 0,
                    },
                    reference_ids: HashMap::new(),
                },
            }],
        );
        hints.insert(
            46,
            vec![HintParams {
                code: "import math".to_string(),
                accessible_scopes: vec![String::from("__main__"), String::from("__main__.main")],
                flow_tracking_data: FlowTrackingData {
                    ap_tracking: ApTracking {
                        group: 5,
                        offset: 0,
                    },
                    reference_ids: HashMap::new(),
                },
            }],
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
        assert_eq!(program.main, None);
        assert_eq!(program.hints, hints);
    }

    #[test]
    fn deserialize_constant() {
        let file =
            File::open("cairo_programs/manually_compiled/deserialize_constant_test.json").unwrap();
        let mut reader = BufReader::new(file);

        let program_json: ProgramJson = serde_json::from_reader(&mut reader).unwrap();
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
            String::from("__main__.compare_abs_arrays.SIZEOF_LOCALS"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(bigint_str!(b"-3618502788666131213697322783095070105623107215331596699973092056135872020481")),
                full_name: None,
                members: None,
            },
        );
        identifiers.insert(
            String::from("starkware.cairo.common.cairo_keccak.keccak.unsigned_div_rem"),
            Identifier {
                pc: None,
                type_: Some(String::from("alias")),
                value: None,
                full_name: None,
                members: None,
            },
        );
        identifiers.insert(
            String::from("starkware.cairo.common.cairo_keccak.packed_keccak.ALL_ONES"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(bigint_str!(
                    b"-106710729501573572985208420194530329073740042555888586719234"
                )),
                full_name: None,
                members: None,
            },
        );
        identifiers.insert(
            String::from("starkware.cairo.common.cairo_keccak.packed_keccak.BLOCK_SIZE"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(bigint!(3)),
                full_name: None,
                members: None,
            },
        );
        identifiers.insert(
            String::from("starkware.cairo.common.alloc.alloc.SIZEOF_LOCALS"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(bigint!(0)),
                full_name: None,
                members: None,
            },
        );
        identifiers.insert(
            String::from("starkware.cairo.common.uint256.SHIFT"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(bigint_str!(b"340282366920938463463374607431768211456")),
                full_name: None,
                members: None,
            },
        );

        assert_eq!(program_json.identifiers, identifiers);
    }

    #[test]
    fn value_address_no_hint_reference_default_test() {
        let valid_json = r#"
            {
                "prime": "0x000A",
                "attributes": [],
                "debug_info": {
                    "instruction_locations": {}
                },  
                "builtins": [],
                "data": [
                ],
                "identifiers": {
                },
                "hints": {
                },
                "reference_manager": {
                    "references": [
                        {
                            "ap_tracking_data": {
                                "group": 0,
                                "offset": 0
                            },
                            "pc": 0,
                            "value": ""
                        }
                    ]
                }
            }"#;

        let program_json: ProgramJson = serde_json::from_str(valid_json).unwrap();

        let reference_manager = ReferenceManager {
            references: vec![Reference {
                ap_tracking_data: ApTracking {
                    group: 0,
                    offset: 0,
                },
                pc: Some(0),
                value_address: ValueAddress::no_hint_reference_default(),
            }],
        };

        assert_eq!(program_json.reference_manager, reference_manager);
    }

    #[test]
    fn deserialize_attributes_test() {
        let valid_json = r#"
            {
                "prime": "0x000A",
                "attributes": [
                    {
                        "accessible_scopes": [
                            "openzeppelin.security.safemath.library",
                            "openzeppelin.security.safemath.library.SafeUint256",
                            "openzeppelin.security.safemath.library.SafeUint256.add"
                        ],
                        "end_pc": 381,
                        "flow_tracking_data": {
                            "ap_tracking": {
                                "group": 14,
                                "offset": 35
                            },
                            "reference_ids": {}
                        },
                        "name": "error_message",
                        "start_pc": 379,
                        "value": "SafeUint256: addition overflow"
                    },
                    {
                        "accessible_scopes": [
                            "openzeppelin.security.safemath.library",
                            "openzeppelin.security.safemath.library.SafeUint256",
                            "openzeppelin.security.safemath.library.SafeUint256.sub_le"
                        ],
                        "end_pc": 404,
                        "flow_tracking_data": {
                            "ap_tracking": {
                                "group": 15,
                                "offset": 60
                            },
                            "reference_ids": {}
                        },
                        "name": "error_message",
                        "start_pc": 402,
                        "value": "SafeUint256: subtraction overflow"
                    }
                ], 
                "debug_info": {
                    "instruction_locations": {}
                },           
                "builtins": [],
                "data": [
                ],
                "identifiers": {
                },
                "hints": {
                },
                "reference_manager": {
                    "references": [
                    ]
                }
            }"#;

        let program_json: ProgramJson = serde_json::from_str(valid_json).unwrap();

        let attributes: Vec<Attribute> = vec![
            Attribute {
                name: String::from("error_message"),
                start_pc: 379,
                end_pc: 381,
                value: String::from("SafeUint256: addition overflow"),
            },
            Attribute {
                name: String::from("error_message"),
                start_pc: 402,
                end_pc: 404,
                value: String::from("SafeUint256: subtraction overflow"),
            },
        ];

        assert_eq!(program_json.attributes, attributes);
    }

    #[test]
    fn deserialize_instruction_locations_test_no_parent() {
        let valid_json = r#"
            {
                "prime": "0x000A",
                "attributes": [], 
                "debug_info": {
                    "file_contents": {},
                    "instruction_locations": {
                        "0": {
                            "accessible_scopes": [
                                "starkware.cairo.lang.compiler.lib.registers",
                                "starkware.cairo.lang.compiler.lib.registers.get_fp_and_pc"
                            ],
                            "flow_tracking_data": {
                                "ap_tracking": {
                                    "group": 0,
                                    "offset": 0
                                },
                                "reference_ids": {}
                            },
                            "hints": [],
                            "inst": {
                                "end_col": 73,
                                "end_line": 7,
                                "input_file": {
                                    "filename": "/Users/user/test/env/lib/python3.9/site-packages/starkware/cairo/lang/compiler/lib/registers.cairo"
                                },
                                "start_col": 5,
                                "start_line": 7
                            }
                        },
                        "3": {
                            "accessible_scopes": [
                                "starkware.cairo.common.alloc",
                                "starkware.cairo.common.alloc.alloc"
                            ],
                            "flow_tracking_data": {
                                "ap_tracking": {
                                    "group": 1,
                                    "offset": 1
                                },
                                "reference_ids": {}
                            },
                            "hints": [],
                            "inst": {
                                "end_col": 40,
                                "end_line": 5,
                                "input_file": {
                                    "filename": "/Users/user/test/env/lib/python3.9/site-packages/starkware/cairo/common/alloc.cairo"
                                },
                                "start_col": 5,
                                "start_line": 5
                            }
                        }
                    }
                },          
                "builtins": [],
                "data": [
                ],
                "identifiers": {
                },
                "hints": {
                },
                "reference_manager": {
                    "references": [
                    ]
                }
            }"#;

        let program_json: ProgramJson = serde_json::from_str(valid_json).unwrap();

        let debug_info: DebugInfo = DebugInfo {
            instruction_locations: HashMap::from([
                (
                    0,
                    InstructionLocation {
                        inst: Location {
                            end_line: 7,
                            end_col: 73,
                            input_file: InputFile { filename: String::from("/Users/user/test/env/lib/python3.9/site-packages/starkware/cairo/lang/compiler/lib/registers.cairo") },
                            parent_location: None,
                            start_line: 7,
                            start_col: 5,
                        },
                        hints: vec![],
                    },
                ),
                (
                    3,
                    InstructionLocation {
                        inst: Location {
                            end_line: 5,
                            end_col: 40,
                            input_file: InputFile { filename: String::from("/Users/user/test/env/lib/python3.9/site-packages/starkware/cairo/common/alloc.cairo") },
                            parent_location: None,
                            start_line: 5,
                            start_col: 5,
                        },
                        hints: vec![],
                    },
                ),
            ]),
        };

        assert_eq!(program_json.debug_info, Some(debug_info));
    }

    #[test]
    fn deserialize_instruction_locations_test_with_parent() {
        let valid_json = r#"
            {
                "prime": "0x000A",
                "attributes": [], 
                "debug_info": {
                    "file_contents": {},
                    "instruction_locations": {
                        "4": {
                            "accessible_scopes": [
                                "__main__",
                                "__main__",
                                "__main__.constructor"
                            ],
                            "flow_tracking_data": null,
                            "hints": [],
                            "inst": {
                                "end_col": 36,
                                "end_line": 9,
                                "input_file": {
                                    "filename": "test/contracts/cairo/always_fail.cairo"
                                },
                                "parent_location": [
                                    {
                                        "end_col": 36,
                                        "end_line": 9,
                                        "input_file": {
                                            "filename": "test/contracts/cairo/always_fail.cairo"
                                        },
                                        "parent_location": [
                                            {
                                                "end_col": 15,
                                                "end_line": 11,
                                                "input_file": {
                                                    "filename": "test/contracts/cairo/always_fail.cairo"
                                                },
                                                "start_col": 5,
                                                "start_line": 11
                                            },
                                            "While trying to retrieve the implicit argument 'syscall_ptr' in:"
                                        ],
                                        "start_col": 18,
                                        "start_line": 9
                                    },
                                    "While expanding the reference 'syscall_ptr' in:"
                                ],
                                "start_col": 18,
                                "start_line": 9
                            }
                        }
                    }
                },          
                "builtins": [],
                "data": [
                ],
                "identifiers": {
                },
                "hints": {
                },
                "reference_manager": {
                    "references": [
                    ]
                }
            }"#;

        let program_json: ProgramJson = serde_json::from_str(valid_json).unwrap();

        let debug_info: DebugInfo = DebugInfo { instruction_locations: HashMap::from(
            [
                (4, InstructionLocation {
                    inst: Location { end_line: 9, end_col: 36,input_file: InputFile { filename: String::from("test/contracts/cairo/always_fail.cairo") }, parent_location: Some(
                        (Box::new(Location {
                            end_line: 9,
                            end_col: 36,
                            input_file: InputFile { filename: String::from("test/contracts/cairo/always_fail.cairo") },
                            parent_location: Some(
                                (   Box::new(Location {
                                    end_line: 11,
                                    end_col: 15,
                                    input_file: InputFile { filename: String::from("test/contracts/cairo/always_fail.cairo") },
                                    parent_location: None,
                                    start_line: 11,
                                    start_col: 5,
                                })
                                    , String::from("While trying to retrieve the implicit argument 'syscall_ptr' in:")
                                )
                            ),
                            start_line: 9,
                            start_col: 18,
                        }), String::from( "While expanding the reference 'syscall_ptr' in:"))
                    ), start_line: 9, start_col: 18 },
                    hints: vec![],
                }),
            ]
        ) };

        assert_eq!(program_json.debug_info, Some(debug_info));
    }
}
