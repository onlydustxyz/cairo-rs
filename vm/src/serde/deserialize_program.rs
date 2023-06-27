use crate::stdlib::{collections::HashMap, fmt, prelude::*, sync::Arc};

use super::serialize_program::serialize_value_address;
use crate::vm::runners::builtin_runner::SEGMENT_ARENA_BUILTIN_NAME;
use crate::{
    serde::{deserialize_utils, serialize_program::number_from_felt},
    types::{
        errors::program_errors::ProgramError,
        instruction::Register,
        program::{Program, SharedProgramData},
        relocatable::MaybeRelocatable,
    },
    vm::runners::builtin_runner::{
        BITWISE_BUILTIN_NAME, EC_OP_BUILTIN_NAME, HASH_BUILTIN_NAME, KECCAK_BUILTIN_NAME,
        OUTPUT_BUILTIN_NAME, POSEIDON_BUILTIN_NAME, RANGE_CHECK_BUILTIN_NAME,
        SIGNATURE_BUILTIN_NAME,
    },
};
use felt::{Felt252, PRIME_STR};
use num_traits::float::FloatCore;
use num_traits::{Num, Pow, Zero};
#[cfg(feature = "parity-scale-codec")]
use parity_scale_codec::{Decode, Encode};
use serde::{de, de::MapAccess, de::SeqAccess, Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Number;

// This enum is used to deserialize program builtins into &str and catch non-valid names
#[derive(Serialize, Deserialize, Debug, PartialEq, Copy, Clone, Eq, Hash)]
#[allow(non_camel_case_types)]
#[cfg_attr(feature = "parity-scale-codec", derive(Encode, Decode))]
pub enum BuiltinName {
    output,
    range_check,
    pedersen,
    ecdsa,
    keccak,
    bitwise,
    ec_op,
    poseidon,
    segment_arena,
}

impl BuiltinName {
    pub fn name(&self) -> &'static str {
        match self {
            BuiltinName::output => OUTPUT_BUILTIN_NAME,
            BuiltinName::range_check => RANGE_CHECK_BUILTIN_NAME,
            BuiltinName::pedersen => HASH_BUILTIN_NAME,
            BuiltinName::ecdsa => SIGNATURE_BUILTIN_NAME,
            BuiltinName::keccak => KECCAK_BUILTIN_NAME,
            BuiltinName::bitwise => BITWISE_BUILTIN_NAME,
            BuiltinName::ec_op => EC_OP_BUILTIN_NAME,
            BuiltinName::poseidon => POSEIDON_BUILTIN_NAME,
            BuiltinName::segment_arena => SEGMENT_ARENA_BUILTIN_NAME,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
pub struct ProgramJson {
    pub prime: String,
    pub builtins: Vec<BuiltinName>,
    #[serde(
        deserialize_with = "deserialize_array_of_bigint_hex",
        serialize_with = "serialize_program_data"
    )]
    pub data: Vec<MaybeRelocatable>,
    pub identifiers: HashMap<String, Identifier>,
    pub hints: HashMap<usize, Vec<HintParams>>,
    pub reference_manager: ReferenceManager,
    pub attributes: Vec<Attribute>,
    pub debug_info: Option<DebugInfo>,
    pub main_scope: String,
    pub compiler_version: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "parity-scale-codec", derive(Encode, Decode))]
pub struct HintParams {
    pub code: String,
    pub accessible_scopes: Vec<String>,
    pub flow_tracking_data: FlowTrackingData,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FlowTrackingData {
    pub ap_tracking: ApTracking,
    #[serde(deserialize_with = "deserialize_map_to_string_and_usize_hashmap")]
    pub reference_ids: HashMap<String, usize>,
}
#[cfg(feature = "parity-scale-codec")]
impl Encode for FlowTrackingData {
    fn encode(&self) -> Vec<u8> {
        let val = self.clone();
        let reference_ids = val
            .reference_ids
            .into_iter()
            .collect::<Vec<(String, usize)>>();
        let reference_ids: Vec<(String, [u8; core::mem::size_of::<usize>()])> =
            unsafe { core::mem::transmute(reference_ids) };

        (val.ap_tracking, reference_ids).encode()
    }
}
#[cfg(feature = "parity-scale-codec")]
impl Decode for FlowTrackingData {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        let res = <(
            ApTracking,
            Vec<(String, [u8; core::mem::size_of::<usize>()])>,
        )>::decode(input)
        .unwrap();
        let reference_ids: Vec<(String, usize)> = unsafe { core::mem::transmute(res.1) };
        Ok(FlowTrackingData {
            ap_tracking: res.0,
            reference_ids: <HashMap<String, usize>>::from_iter(reference_ids.into_iter()),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ApTracking {
    pub group: usize,
    pub offset: usize,
}

#[cfg(feature = "parity-scale-codec")]
impl Encode for ApTracking {
    fn encode(&self) -> Vec<u8> {
        (self.group as u64, self.offset as u64).encode()
    }
}
#[cfg(feature = "parity-scale-codec")]
impl Decode for ApTracking {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        let res = <(u64, u64)>::decode(input).unwrap();
        Ok(ApTracking {
            group: res.0 as usize,
            offset: res.1 as usize,
        })
    }
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

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Identifier {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pc: Option<usize>,
    #[serde(
        rename(deserialize = "type", serialize = "type"),
        skip_serializing_if = "Option::is_none"
    )]
    pub type_: Option<String>,
    #[serde(default)]
    #[serde(
        deserialize_with = "felt_from_number",
        serialize_with = "number_from_felt",
        skip_serializing_if = "Option::is_none"
    )]
    pub value: Option<Felt252>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<HashMap<String, Member>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cairo_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decorators: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<Reference>>,
}

#[cfg(feature = "parity-scale-codec")]
impl Encode for Identifier {
    fn encode(&self) -> Vec<u8> {
        let val = self.clone();
        let members: Option<Vec<(String, Member)>> = val.members.map(|m| m.into_iter().collect());
        (
            val.pc.map(|v| v as u64),
            val.type_,
            val.value,
            val.full_name,
            members,
            val.cairo_type,
            val.decorators,
            val.size,
            val.destination,
            val.references,
        )
            .encode()
    }
}
#[cfg(feature = "parity-scale-codec")]
impl Decode for Identifier {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        let res = <(
            Option<u64>,
            Option<String>,
            Option<Felt252>,
            Option<String>,
            Option<Vec<(String, Member)>>,
            Option<String>,
            Option<Vec<String>>,
            Option<u64>,
            Option<String>,
            Option<Vec<Reference>>,
        )>::decode(input)
        .unwrap();
        Ok(Identifier {
            pc: res.0.map(|v| v as usize),
            type_: res.1,
            value: res.2,
            full_name: res.3,
            members: res
                .4
                .map(|v| <HashMap<String, Member>>::from_iter(v.into_iter())),
            cairo_type: res.5,
            decorators: res.6,
            size: res.7,
            destination: res.8,
            references: res.9,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Member {
    pub cairo_type: String,
    pub offset: usize,
}

#[cfg(feature = "parity-scale-codec")]
impl Encode for Member {
    fn encode(&self) -> Vec<u8> {
        let val = self.clone();
        (val.cairo_type, val.offset as u64).encode()
    }
}
#[cfg(feature = "parity-scale-codec")]
impl Decode for Member {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        let res = <(String, u64)>::decode(input).unwrap();
        Ok(Member {
            cairo_type: res.0,
            offset: res.1 as usize,
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Attribute {
    pub name: String,
    pub start_pc: usize,
    pub end_pc: usize,
    pub value: String,
    pub flow_tracking_data: Option<FlowTrackingData>,
    pub accessible_scopes: Vec<String>,
}

#[cfg(feature = "parity-scale-codec")]
impl Encode for Attribute {
    fn encode(&self) -> Vec<u8> {
        let val = self.clone();
        (
            val.name,
            val.start_pc as u64,
            val.end_pc as u64,
            val.value,
            val.flow_tracking_data,
            val.accessible_scopes,
        )
            .encode()
    }
}
#[cfg(feature = "parity-scale-codec")]
impl Decode for Attribute {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        let res = <(
            String,
            u64,
            u64,
            String,
            Option<FlowTrackingData>,
            Vec<String>,
        )>::decode(input)
        .unwrap();
        Ok(Attribute {
            name: res.0,
            start_pc: res.1 as usize,
            end_pc: res.2 as usize,
            value: res.3,
            flow_tracking_data: res.4,
            accessible_scopes: res.5,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parity-scale-codec", derive(Encode, Decode))]
pub struct Location {
    pub end_line: u32,
    pub end_col: u32,
    pub input_file: InputFile,
    pub parent_location: Option<(Box<Location>, String)>,
    pub start_line: u32,
    pub start_col: u32,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct DebugInfo {
    instruction_locations: HashMap<usize, InstructionLocation>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "parity-scale-codec", derive(Encode, Decode))]
pub struct InstructionLocation {
    pub inst: Location,
    pub hints: Vec<HintLocation>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "parity-scale-codec", derive(Encode, Decode))]
pub struct InputFile {
    pub filename: String,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "parity-scale-codec", derive(Encode, Decode))]
pub struct HintLocation {
    pub location: Location,
    pub n_prefix_newlines: u32,
}

fn felt_from_number<'de, D>(deserializer: D) -> Result<Option<Felt252>, D::Error>
where
    D: Deserializer<'de>,
{
    let n = Number::deserialize(deserializer)?;
    match Felt252::parse_bytes(n.to_string().as_bytes(), 10) {
        Some(x) => Ok(Some(x)),
        None => {
            // Handle de Number with scientific notation cases
            // e.g.: n = Number(1e27)
            let felt = deserialize_scientific_notation(n);
            if felt.is_some() {
                return Ok(felt);
            }

            Err(de::Error::custom(String::from(
                "felt_from_number parse error",
            )))
        }
    }
}

fn deserialize_scientific_notation(n: Number) -> Option<Felt252> {
    match n.as_f64() {
        None => {
            let str = n.to_string();
            let list: [&str; 2] = str.split('e').collect::<Vec<&str>>().try_into().ok()?;

            let exponent = list[1].parse::<u32>().ok()?;
            let base = Felt252::parse_bytes(list[0].to_string().as_bytes(), 10)?;
            Some(base * Felt252::from(10).pow(exponent))
        }
        Some(float) => Felt252::parse_bytes(FloatCore::round(float).to_string().as_bytes(), 10),
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Default)]
pub struct ReferenceManager {
    pub references: Vec<Reference>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Reference {
    pub ap_tracking_data: ApTracking,
    pub pc: Option<usize>,
    #[serde(
        deserialize_with = "deserialize_value_address",
        serialize_with = "serialize_value_address"
    )]
    #[serde(rename(deserialize = "value", serialize = "value"))]
    pub value_address: ValueAddress,
}

#[cfg(feature = "parity-scale-codec")]
impl Encode for Reference {
    fn encode(&self) -> Vec<u8> {
        let val = self.clone();
        (
            val.ap_tracking_data,
            val.pc.map(|v| v as u64),
            val.value_address,
        )
            .encode()
    }
}
#[cfg(feature = "parity-scale-codec")]
impl Decode for Reference {
    fn decode<I: parity_scale_codec::Input>(
        input: &mut I,
    ) -> Result<Self, parity_scale_codec::Error> {
        let res = <(ApTracking, Option<u64>, ValueAddress)>::decode(input).unwrap();
        Ok(Reference {
            ap_tracking_data: res.0,
            pc: res.1.map(|v| v as usize),
            value_address: res.2,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "parity-scale-codec", derive(Encode, Decode))]
pub enum OffsetValue {
    Immediate(Felt252),
    Value(i32),
    Reference(Register, i32, bool),
}

impl fmt::Display for OffsetValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let val = match self {
            OffsetValue::Immediate(felt) => match felt {
                felt if felt == &Felt252::zero() => String::default(),
                felt if felt > &Felt252::zero() => format!("{:}", felt),
                _ => format!("({:})", felt),
            },
            OffsetValue::Value(value) => match value {
                0 => String::default(),
                1.. => format!("{:}", value),
                _ => format!("({:})", value),
            },
            OffsetValue::Reference(register, off, brackets) => {
                //
                let register = match register {
                    Register::AP => "ap".to_string(),
                    Register::FP => "fp".to_string(),
                };
                let offset = match off {
                    0 => {
                        if *brackets {
                            String::default()
                        } else {
                            format!(" - {:}", off)
                        }
                    }
                    1.. => format!(" + {:}", off),
                    _ => format!(" + ({:})", off),
                };

                if *brackets {
                    format!("[{register:}{offset:}]")
                } else {
                    format!("{register:}{offset:}")
                }
            }
        };
        write!(f, "{}", val)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "parity-scale-codec", derive(Encode, Decode))]
pub struct ValueAddress {
    pub offset1: OffsetValue,
    pub offset2: OffsetValue,
    pub dereference: bool,
    pub value_type: String,
}

impl fmt::Display for ValueAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let offset1 = format!("{:}", self.offset1);
        let offset2 = format!("{:}", self.offset2);
        let mut res = format!("cast({offset1:}");
        if !offset2.is_empty() {
            res = format!("{res:} + ");
        }

        res = format!("{res:}{offset2:}, {:}", self.value_type);
        if let OffsetValue::Value(_) = self.offset2 {
            res = format!("{res:}*");
        }
        res = format!("{res:})");
        if self.dereference {
            res = format!("[{res:}]")
        }
        // if self.dereference {
        //     res = format!("[{res:}*)]");
        // }
        // else {
        //     res = format!("{res:})")
        // }
        write!(f, "{}", res)
    }
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

struct Felt252Visitor;

impl<'de> de::Visitor<'de> for Felt252Visitor {
    type Value = Felt252;

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
            Ok(Felt252::from_str_radix(&no_prefix_hex, 16).map_err(de::Error::custom)?)
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
                data.push(MaybeRelocatable::Int(
                    Felt252::from_str_radix(&no_prefix_hex, 16).map_err(de::Error::custom)?,
                ));
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

pub fn deserialize_felt_hex<'de, D: Deserializer<'de>>(d: D) -> Result<Felt252, D::Error> {
    d.deserialize_str(Felt252Visitor)
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

pub fn serialize_program_data<S: Serializer>(
    v: &[MaybeRelocatable],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let v = v
        .iter()
        .map(|val| match val.clone() {
            MaybeRelocatable::Int(value) => format!("0x{:x}", value.to_biguint()),
            MaybeRelocatable::RelocatableValue(_) => {
                panic!("Got unexpected relocatable value in program data")
            }
        })
        .collect::<Vec<String>>();
    v.serialize(serializer)
}

pub fn deserialize_program_json(reader: &[u8]) -> Result<ProgramJson, ProgramError> {
    let program_json = serde_json::from_slice(reader)?;
    Ok(program_json)
}
pub fn deserialize_and_parse_program(
    reader: &[u8],
    entrypoint: Option<&str>,
) -> Result<Program, ProgramError> {
    let program_json: ProgramJson = deserialize_program_json(reader)?;
    parse_program_json(program_json, entrypoint)
}

pub fn parse_program_json(
    program_json: ProgramJson,
    entrypoint: Option<&str>,
) -> Result<Program, ProgramError> {
    if PRIME_STR != program_json.prime {
        return Err(ProgramError::PrimeDiffers(program_json.prime));
    }

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

    let mut constants = HashMap::new();
    for (key, value) in program_json.identifiers.iter() {
        if value.type_.as_deref() == Some("const") {
            let value = value
                .value
                .clone()
                .ok_or_else(|| ProgramError::ConstWithoutValue(key.clone()))?;
            constants.insert(key.clone(), value);
        }
    }

    let shared_program_data = SharedProgramData {
        data: program_json.data,
        hints: program_json.hints,
        main: entrypoint_pc,
        start,
        end,
        error_message_attributes: program_json
            .attributes
            .into_iter()
            .filter(|attr| attr.name == "error_message")
            .collect(),
        instruction_locations: program_json
            .debug_info
            .map(|debug_info| debug_info.instruction_locations),
        identifiers: program_json.identifiers,
        reference_manager: Program::get_reference_list(&program_json.reference_manager),
    };
    Ok(Program {
        shared_program_data: Arc::new(shared_program_data),
        constants,
        builtins: program_json.builtins,
    })
}

pub fn parse_program(program: Program) -> ProgramJson {
    ProgramJson {
        prime: program.prime().to_owned(),
        builtins: program.builtins().to_vec(),
        data: program.data().to_vec(),
        identifiers: program.identifiers().to_owned(),
        hints: program.hints().to_owned(),
        reference_manager: program.reference_manager(),
        attributes: program.error_message_attributes().to_owned(),
        debug_info: program
            .instruction_locations()
            .clone()
            .map(|instruction_locations| DebugInfo {
                instruction_locations,
            }),
        main_scope: String::default(),
        compiler_version: String::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use felt::felt_str;
    use num_traits::{One, Zero};

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg(feature = "parity-scale-codec")]
    fn test_encode_decode_attribute() {
        let attributes: Vec<Attribute> = vec![
            Attribute {
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
                accessible_scopes: vec![
                    "openzeppelin.security.safemath.library".to_string(),
                    "openzeppelin.security.safemath.library.SafeUint256".to_string(),
                    "openzeppelin.security.safemath.library.SafeUint256.add".to_string(),
                ],
            },
            Attribute {
                name: String::from("error_message"),
                start_pc: 402,
                end_pc: 404,
                value: String::from("SafeUint256: subtraction overflow"),
                flow_tracking_data: Some(FlowTrackingData {
                    ap_tracking: ApTracking {
                        group: 15,
                        offset: 60,
                    },
                    reference_ids: HashMap::new(),
                }),
                accessible_scopes: vec![
                    "openzeppelin.security.safemath.library".to_string(),
                    "openzeppelin.security.safemath.library.SafeUint256".to_string(),
                    "openzeppelin.security.safemath.library.SafeUint256.sub_le".to_string(),
                ],
            },
        ];
        for attribute in attributes.iter() {
            assert_eq!(
                *attribute,
                Attribute::decode(&mut &attribute.encode()[..]).unwrap()
            )
        }
    }

    #[test]
    #[cfg(feature = "parity-scale-codec")]
    fn test_encode_decode_flow_tracking_data() {
        let flow_tracking_data = FlowTrackingData {
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
        };
        assert_eq!(
            flow_tracking_data,
            FlowTrackingData::decode(&mut &flow_tracking_data.encode()[..]).unwrap()
        )
    }

    #[test]
    fn test_offset_value_display() {
        assert_eq!("", format!("{:}", OffsetValue::Immediate(Felt252::zero())));
        assert_eq!("", format!("{:}", OffsetValue::Value(0)));
        assert_eq!("1", format!("{:}", OffsetValue::Immediate(Felt252::one())));
        assert_eq!("(-2)", format!("{:}", OffsetValue::Value(-2)));
        assert_eq!("2", format!("{:}", OffsetValue::Value(2)));
        assert_eq!(
            "[ap]",
            format!("{:}", OffsetValue::Reference(Register::AP, 0, true))
        );
        assert_eq!(
            "[ap + 2]",
            format!("{:}", OffsetValue::Reference(Register::AP, 2, true))
        );
        assert_eq!(
            "[ap + (-2)]",
            format!("{:}", OffsetValue::Reference(Register::AP, -2, true))
        );
        assert_eq!(
            "ap - 0",
            format!("{:}", OffsetValue::Reference(Register::AP, 0, false))
        );
        assert_eq!(
            "ap + 1",
            format!("{:}", OffsetValue::Reference(Register::AP, 1, false))
        );
        assert_eq!(
            "ap + (-1)",
            format!("{:}", OffsetValue::Reference(Register::AP, -1, false))
        );

        assert_eq!(
            "[fp]",
            format!("{:}", OffsetValue::Reference(Register::FP, 0, true))
        );
        assert_eq!(
            "[fp + 2]",
            format!("{:}", OffsetValue::Reference(Register::FP, 2, true))
        );
        assert_eq!(
            "[fp + (-2)]",
            format!("{:}", OffsetValue::Reference(Register::FP, -2, true))
        );
        assert_eq!(
            "fp - 0",
            format!("{:}", OffsetValue::Reference(Register::FP, 0, false))
        );
        assert_eq!(
            "fp + 1",
            format!("{:}", OffsetValue::Reference(Register::FP, 1, false))
        );
        assert_eq!(
            "fp + (-1)",
            format!("{:}", OffsetValue::Reference(Register::FP, -1, false))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn serialization_works() {
        let reader =
            include_bytes!("../../../cairo_programs/manually_compiled/valid_program_a.json");

        let program: Program = deserialize_and_parse_program(reader, Some("main"))
            .expect("Failed to deserialize program");

        let program_json = parse_program(program.clone());

        let program_res =
            parse_program_json(program_json, Some("main")).expect("Failed to parse program");

        assert_eq!(program, program_res);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_bigint_invalid_char_error() {
        let invalid_char = r#"
            {
                "prime": "0xlambda"
            }"#;

        let invalid_char_error: Result<ProgramJson, _> = serde_json::from_str(invalid_char);

        assert!(invalid_char_error.is_err());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_from_string_json() {
        let valid_json = r#"
            {
                "prime": "0x800000000000011000000000000000000000000000000000000000000000001",
                "main_scope": "__main__",
                "compiler_version": "0.11.0",
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

        let data: Vec<MaybeRelocatable> = vec![
            MaybeRelocatable::Int(Felt252::new(5189976364521848832_i64)),
            MaybeRelocatable::Int(Felt252::new(1000_i64)),
            MaybeRelocatable::Int(Felt252::new(5189976364521848832_i64)),
            MaybeRelocatable::Int(Felt252::new(2000_i64)),
            MaybeRelocatable::Int(Felt252::new(5201798304953696256_i64)),
            MaybeRelocatable::Int(Felt252::new(2345108766317314046_i64)),
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
                        offset2: OffsetValue::Immediate(Felt252::new(2)),
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

        assert_eq!(
            program_json.prime,
            "0x800000000000011000000000000000000000000000000000000000000000001"
        );
        assert!(program_json.builtins.is_empty());
        assert_eq!(program_json.data, data);
        assert_eq!(program_json.identifiers["__main__.main"].pc, Some(0));
        assert_eq!(program_json.hints, hints);
        assert_eq!(program_json.reference_manager, reference_manager);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_program_json_from_json_file_a() {
        // Open json file with (valid) even length encoded hex
        let reader =
            include_bytes!("../../../cairo_programs/manually_compiled/valid_program_a.json");

        let program_json: ProgramJson = serde_json::from_slice(reader).unwrap();

        assert_eq!(
            program_json.prime,
            "0x800000000000011000000000000000000000000000000000000000000000001"
        );
        assert!(program_json.builtins.is_empty());
        assert_eq!(program_json.data.len(), 6);
        assert_eq!(program_json.identifiers["__main__.main"].pc, Some(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_program_json_from_json_file_b() {
        // Open json file with (valid) odd length encoded hex
        let reader =
            include_bytes!("../../../cairo_programs/manually_compiled/valid_program_b.json");

        let program_json: ProgramJson = serde_json::from_slice(reader).unwrap();
        let builtins: Vec<BuiltinName> = vec![BuiltinName::output, BuiltinName::range_check];

        assert_eq!(
            program_json.prime,
            "0x800000000000011000000000000000000000000000000000000000000000001"
        );
        assert_eq!(program_json.builtins, builtins);
        assert_eq!(program_json.data.len(), 24);
        assert_eq!(program_json.identifiers["__main__.main"].pc, Some(13));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_program_json_from_json_file_gives_error() {
        // Open json file with (invalid) even length encoded hex
        let reader = include_bytes!(
            "../../../cairo_programs/manually_compiled/invalid_even_length_hex.json"
        );

        let even_result: Result<ProgramJson, _> = serde_json::from_slice(reader);

        assert!(even_result.is_err());

        // Open json file with (invalid) odd length encoded hex
        let reader =
            include_bytes!("../../../cairo_programs/manually_compiled/invalid_odd_length_hex.json");

        let odd_result: Result<ProgramJson, _> = serde_json::from_slice(reader);

        assert!(odd_result.is_err());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_missing_entrypoint_gives_error() {
        let reader =
            include_bytes!("../../../cairo_programs/manually_compiled/valid_program_a.json");

        let deserialization_result =
            deserialize_and_parse_program(reader, Some("missing_function"));
        assert!(deserialization_result.is_err());
        assert_matches!(
            deserialization_result,
            Err(ProgramError::EntrypointNotFound(_))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_program_test() {
        let reader =
            include_bytes!("../../../cairo_programs/manually_compiled/valid_program_a.json");

        let program: Program = deserialize_and_parse_program(reader, Some("main"))
            .expect("Failed to deserialize program");

        let builtins: Vec<BuiltinName> = Vec::new();
        let data: Vec<MaybeRelocatable> = vec![
            MaybeRelocatable::Int(Felt252::new(5189976364521848832_i64)),
            MaybeRelocatable::Int(Felt252::new(1000)),
            MaybeRelocatable::Int(Felt252::new(5189976364521848832_i64)),
            MaybeRelocatable::Int(Felt252::new(2000)),
            MaybeRelocatable::Int(Felt252::new(5201798304953696256_i64)),
            MaybeRelocatable::Int(Felt252::new(2345108766317314046_i64)),
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

        assert_eq!(program.builtins, builtins);
        assert_eq!(program.shared_program_data.data, data);
        assert_eq!(program.shared_program_data.main, Some(0));
        assert_eq!(program.shared_program_data.hints, hints);
    }

    /// Deserialize a program without an entrypoint.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_program_without_entrypoint_test() {
        let reader =
            include_bytes!("../../../cairo_programs/manually_compiled/valid_program_a.json");

        let program: Program =
            deserialize_and_parse_program(reader, None).expect("Failed to deserialize program");

        let builtins: Vec<BuiltinName> = Vec::new();
        let data: Vec<MaybeRelocatable> = vec![
            MaybeRelocatable::Int(Felt252::new(5189976364521848832_i64)),
            MaybeRelocatable::Int(Felt252::new(1000)),
            MaybeRelocatable::Int(Felt252::new(5189976364521848832_i64)),
            MaybeRelocatable::Int(Felt252::new(2000)),
            MaybeRelocatable::Int(Felt252::new(5201798304953696256_i64)),
            MaybeRelocatable::Int(Felt252::new(2345108766317314046_i64)),
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

        assert_eq!(program.builtins, builtins);
        assert_eq!(program.shared_program_data.data, data);
        assert_eq!(program.shared_program_data.main, None);
        assert_eq!(program.shared_program_data.hints, hints);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_constant() {
        let reader = include_bytes!(
            "../../../cairo_programs/manually_compiled/deserialize_constant_test.json"
        );

        let program_json: ProgramJson = serde_json::from_slice(reader).unwrap();
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
                decorators: Some(vec![]),
                size: None,
                destination: None,
                references: None,
            },
        );
        identifiers.insert(
            String::from("__main__.compare_abs_arrays.SIZEOF_LOCALS"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(felt_str!(
                    "-3618502788666131213697322783095070105623107215331596699973092056135872020481"
                )),
                full_name: None,
                members: None,
                cairo_type: None,
                decorators: None,
                size: None,
                destination: None,
                references: None,
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
                cairo_type: None,
                decorators: None,
                size: None,
                destination: Some("starkware.cairo.common.math.unsigned_div_rem".to_string()),
                references: None,
            },
        );
        identifiers.insert(
            String::from("starkware.cairo.common.cairo_keccak.packed_keccak.ALL_ONES"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(felt_str!(
                    "-106710729501573572985208420194530329073740042555888586719234"
                )),
                full_name: None,
                members: None,
                cairo_type: None,
                decorators: None,
                size: None,
                destination: None,
                references: None,
            },
        );
        identifiers.insert(
            String::from("starkware.cairo.common.cairo_keccak.packed_keccak.BLOCK_SIZE"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(Felt252::new(3)),
                full_name: None,
                members: None,
                cairo_type: None,
                decorators: None,
                size: None,
                destination: None,
                references: None,
            },
        );
        identifiers.insert(
            String::from("starkware.cairo.common.alloc.alloc.SIZEOF_LOCALS"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(Felt252::zero()),
                full_name: None,
                members: None,
                cairo_type: None,
                decorators: None,
                size: None,
                destination: None,
                references: None,
            },
        );
        identifiers.insert(
            String::from("starkware.cairo.common.uint256.SHIFT"),
            Identifier {
                pc: None,
                type_: Some(String::from("const")),
                value: Some(felt_str!("340282366920938463463374607431768211456")),
                full_name: None,
                members: None,
                cairo_type: None,
                decorators: None,
                size: None,
                destination: None,
                references: None,
            },
        );

        assert_eq!(program_json.identifiers, identifiers);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn value_address_no_hint_reference_default_test() {
        let valid_json = r#"
            {
                "prime": "0x800000000000011000000000000000000000000000000000000000000000001",
                "main_scope": "__main__",
                "compiler_version": "0.11.0",
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_attributes_test() {
        let valid_json = r#"
            {
                "prime": "0x800000000000011000000000000000000000000000000000000000000000001",
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
                },
                "main_scope": "__main__",
                "compiler_version": "0.11.0"
            }"#;

        let program_json: ProgramJson = serde_json::from_str(valid_json).unwrap();

        let attributes: Vec<Attribute> = vec![
            Attribute {
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
                accessible_scopes: vec![
                    "openzeppelin.security.safemath.library".to_string(),
                    "openzeppelin.security.safemath.library.SafeUint256".to_string(),
                    "openzeppelin.security.safemath.library.SafeUint256.add".to_string(),
                ],
            },
            Attribute {
                name: String::from("error_message"),
                start_pc: 402,
                end_pc: 404,
                value: String::from("SafeUint256: subtraction overflow"),
                flow_tracking_data: Some(FlowTrackingData {
                    ap_tracking: ApTracking {
                        group: 15,
                        offset: 60,
                    },
                    reference_ids: HashMap::new(),
                }),
                accessible_scopes: vec![
                    "openzeppelin.security.safemath.library".to_string(),
                    "openzeppelin.security.safemath.library.SafeUint256".to_string(),
                    "openzeppelin.security.safemath.library.SafeUint256.sub_le".to_string(),
                ],
            },
        ];

        assert_eq!(program_json.attributes, attributes);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_instruction_locations_test_no_parent() {
        let valid_json = r#"
            {
                "prime": "0x800000000000011000000000000000000000000000000000000000000000001",
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
                },
                "main_scope": "__main__",
                "compiler_version": "0.11.0"
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_instruction_locations_test_with_parent() {
        let valid_json = r#"
            {
                "prime": "0x800000000000011000000000000000000000000000000000000000000000001",
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
                },
                "main_scope": "__main__",
                "compiler_version": "0.11.0"
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

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_program_with_type_definition() {
        let reader = include_bytes!("../../../cairo_programs/uint256_integration_tests.json");

        let program_json: ProgramJson = serde_json::from_slice(reader).unwrap();

        assert_eq!(
            program_json.identifiers["starkware.cairo.common.alloc.alloc.Return"]
                .cairo_type
                .as_ref()
                .expect("key not found"),
            "(ptr: felt*)"
        );
        assert_eq!(
            program_json.identifiers["starkware.cairo.common.uint256.uint256_add.Return"]
                .cairo_type
                .as_ref()
                .expect("key not found"),
            "(res: starkware.cairo.common.uint256.Uint256, carry: felt)"
        );
        assert_eq!(
            program_json.identifiers["__main__.test_unsigned_div_rem.Return"]
                .cairo_type
                .as_ref()
                .expect("key not found"),
            "()"
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_nonbase10_number_errors() {
        let valid_json = r#"
        {
            "value" : 0x123
        }"#;

        let iden: Result<Identifier, serde_json::Error> = serde_json::from_str(valid_json);
        assert!(iden.err().is_some());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_felt_from_number_with_scientific_notation() {
        let n = Number::deserialize(serde_json::Value::from(1000000000000000000000000000_u128))
            .unwrap();
        assert_eq!(n.to_string(), "1e27".to_owned());

        assert_matches!(
            felt_from_number(n),
            Ok(x) if x == Some(Felt252::one() * Felt252::from(10).pow(27))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_felt_from_number_with_scientific_notation_with_fractional_part() {
        let n = serde_json::Value::Number(Number::from_f64(64e+74).unwrap());

        assert_matches!(
            felt_from_number(n),
            Ok(x) if x == Some(Felt252::from_str_radix("64", 10).unwrap() * Felt252::from(10).pow(74))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_felt_from_number_with_scientific_notation_with_fractional_part_f64_max() {
        let n = serde_json::Value::Number(Number::from_f64(f64::MAX).unwrap());
        assert_eq!(
            felt_from_number(n).unwrap(),
            Some(
                Felt252::from_str_radix(
                    "2082797363194934431336897723140298717588791783575467744530053896730196177808",
                    10
                )
                .unwrap()
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_felt_from_number_with_scientific_notation_big_exponent() {
        #[derive(Deserialize, Debug, PartialEq)]
        struct Test {
            #[serde(deserialize_with = "felt_from_number")]
            f: Option<Felt252>,
        }
        let malicious_input = &format!(
            "{{ \"f\": {}e{} }}",
            String::from_utf8(vec![b'9'; 1000]).unwrap(),
            u32::MAX
        );
        let f = serde_json::from_str::<Test>(malicious_input)
            .unwrap()
            .f
            .unwrap();
        assert_eq!(
            f,
            Felt252::from_str_radix(
                "2471602022505793130446032259107029522557827898253184929958153020344968292412",
                10
            )
            .unwrap()
        );
    }
}
