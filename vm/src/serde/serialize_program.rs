use core::str::FromStr;

use felt::Felt252;
use serde::{ser, Serialize, Serializer};
use serde_json::Number;

use super::deserialize_program::ValueAddress;

pub fn number_from_felt<S: Serializer>(
    v: &Option<Felt252>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    // If the value is `None` we don't serialize it so it's safe to unwrap.
    let num = Number::from_str(&v.clone().unwrap().to_biguint().to_str_radix(10))
        .map_err(|e| ser::Error::custom(format!("couldn't convert felt to number {e:}")))?;
    Number::serialize(&num, serializer)
}

pub fn serialize_value_address<S: Serializer>(
    v: &ValueAddress,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    format!("{:}", v).serialize(serializer)
}

#[cfg(test)]
mod tests {
    use crate::serde::deserialize_program::OffsetValue;
    use crate::serde::deserialize_program::ProgramJson;
    use crate::serde::deserialize_program::ValueAddress;
    use crate::serde::deserialize_utils::parse_value;
    use crate::stdlib::string::ToString;
    use crate::types::instruction::Register;
    use felt::Felt252;
    use num_traits::Zero;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    fn serialize_and_deserialize_program() {
        let program_bytes =
            include_bytes!("../../../cairo_programs/manually_compiled/valid_program_c.json");
        let program: ProgramJson = serde_json::from_slice(program_bytes).unwrap();
        let test_bytes = serde_json::to_vec(&program).unwrap();
        let test_value: ProgramJson = serde_json::from_slice(&test_bytes).unwrap();

        pretty_assertions::assert_eq!(program, test_value)
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_inner_dereference_test() {
        let value = "[cast([fp + (-1)] + 2, felt*)]";

        assert_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset2: OffsetValue::Value(2),
                    offset1: OffsetValue::Reference(Register::FP, -1_i32, true),
                    dereference: true,
                    value_type: "felt".to_string(),
                }
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn failing_value_in_valid_program_b() {
        let value = "cast([fp + (-4)] + 1, felt*)";

        pretty_assertions::assert_str_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::FP, -4, true),
                    offset2: OffsetValue::Value(1),
                    dereference: false,
                    value_type: "felt".to_string(),
                }
            )
        );
    }
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_no_inner_dereference_test() {
        let value = "cast(ap + 2, felt*)";

        assert_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 2_i32, false),
                    offset2: OffsetValue::Value(0),
                    dereference: false,
                    value_type: "felt".to_string(),
                }
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_no_register_test() {
        let value = "cast(825323, felt*)";
        assert_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset1: OffsetValue::Value(825323),
                    offset2: OffsetValue::Value(0),
                    dereference: false,
                    value_type: "felt".to_string(),
                }
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_no_inner_deref_and_two_offsets() {
        let value = "[cast(ap - 0 + (-1), felt*)]";

        assert_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, false),
                    offset2: OffsetValue::Value(-1),
                    dereference: true,
                    value_type: "felt".to_string(),
                }
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_inner_deref_and_offset2() {
        let value = "[cast([ap] + 1, __main__.felt*)]";
        println!("{:?}", parse_value(value));

        assert_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, true),
                    offset2: OffsetValue::Value(1),
                    dereference: true,
                    value_type: "__main__.felt".to_string(),
                }
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_inner_deref_to_pointer() {
        let value = "[cast([ap + 1] + 1, felt*)]";

        assert_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 1_i32, true),
                    offset2: OffsetValue::Value(1),
                    dereference: true,
                    value_type: "felt".to_string(),
                }
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_no_reference() {
        let value = "cast(825323, felt)";

        assert_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset1: OffsetValue::Immediate(Felt252::new(825323_i32)),
                    offset2: OffsetValue::Immediate(Felt252::zero()),
                    dereference: false,
                    value_type: "felt".to_string(),
                }
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_one_reference() {
        let value = "[cast([ap] + 1, starkware.cairo.common.cairo_secp.ec.EcPoint*)]";

        assert_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, true),
                    offset2: OffsetValue::Value(1),
                    dereference: true,
                    value_type: "starkware.cairo.common.cairo_secp.ec.EcPoint".to_string(),
                }
            )
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn parse_value_with_doble_reference() {
        let value = "[cast([ap] + 1, starkware.cairo.common.cairo_secp.ec.EcPoint**)]";

        assert_eq!(
            value,
            format!(
                "{:}",
                ValueAddress {
                    offset1: OffsetValue::Reference(Register::AP, 0_i32, true),
                    offset2: OffsetValue::Value(1),
                    dereference: true,
                    value_type: "starkware.cairo.common.cairo_secp.ec.EcPoint*".to_string(),
                }
            )
        );
    }
}
