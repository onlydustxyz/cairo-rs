use crate::stdlib::{collections::HashMap, prelude::*, sync::Arc};

#[cfg(feature = "cairo-1-hints")]
use crate::serde::deserialize_program::{ApTracking, FlowTrackingData};
use crate::{
    hint_processor::hint_processor_definition::HintReference,
    serde::deserialize_program::{
        deserialize_and_parse_program, Attribute, BuiltinName, HintParams, Identifier,
        InstructionLocation, OffsetValue, ReferenceManager,
    },
    types::{
        errors::program_errors::ProgramError, instruction::Register, relocatable::MaybeRelocatable,
    },
};
#[cfg(feature = "cairo-1-hints")]
use cairo_lang_casm_contract_class::CasmContractClass;
use felt::{Felt252, PRIME_STR};

#[cfg(feature = "std")]
use std::path::Path;

// NOTE: `Program` has been split in two containing some data that will be deep-copied
// and some that will be allocated on the heap inside an `Arc<_>`.
// This is because it has been reported that cloning the whole structure when creating
// a `CairoRunner` becomes a bottleneck, but the following solutions were tried and
// discarded:
// - Store only a reference in `CairoRunner` rather than cloning; this doesn't work
//   because then we need to introduce explicit lifetimes, which broke `cairo-rs-py`
//   since PyO3 doesn't support Python objects containing structures with lifetimes.
// - Directly pass an `Arc<Program>` to `CairoRunner::new()` and simply copy that:
//   there was a prohibitive performance hit of 10-15% when doing so, most likely
//   either because of branch mispredictions or the extra level of indirection going
//   through a random location on the heap rather than the likely-to-be-cached spot
//   on the stack.
//
// So, the compromise was to identify which data was less used and avoid copying that,
// using `Arc<_>`, while the most accessed fields remain on the stack for the main
// loop to access. The fields in `SharedProgramData` are either preprocessed and
// copied explicitly (_in addition_ to the clone of `Program`) or are used only in
// exceptional circumstances, such as when reconstructing a backtrace on execution
// failures.
// Fields in `Program` (other than `SharedProgramData` itself) are used by the main logic.
#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub(crate) struct SharedProgramData {
    pub(crate) data: Vec<MaybeRelocatable>,
    pub(crate) hints: HashMap<usize, Vec<HintParams>>,
    pub(crate) main: Option<usize>,
    //start and end labels will only be used in proof-mode
    pub(crate) start: Option<usize>,
    pub(crate) end: Option<usize>,
    pub(crate) error_message_attributes: Vec<Attribute>,
    pub(crate) instruction_locations: Option<HashMap<usize, InstructionLocation>>,
    pub(crate) identifiers: HashMap<String, Identifier>,
    pub(crate) reference_manager: Vec<HintReference>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Program {
    pub(crate) shared_program_data: Arc<SharedProgramData>,
    pub(crate) constants: HashMap<String, Felt252>,
    pub(crate) builtins: Vec<BuiltinName>,
}

impl Program {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        builtins: Vec<BuiltinName>,
        data: Vec<MaybeRelocatable>,
        main: Option<usize>,
        hints: HashMap<usize, Vec<HintParams>>,
        reference_manager: ReferenceManager,
        identifiers: HashMap<String, Identifier>,
        error_message_attributes: Vec<Attribute>,
        instruction_locations: Option<HashMap<usize, InstructionLocation>>,
    ) -> Result<Program, ProgramError> {
        let mut constants = HashMap::new();
        for (key, value) in identifiers.iter() {
            if value.type_.as_deref() == Some("const") {
                let value = value
                    .value
                    .clone()
                    .ok_or_else(|| ProgramError::ConstWithoutValue(key.clone()))?;
                constants.insert(key.clone(), value);
            }
        }
        let shared_program_data = SharedProgramData {
            data,
            hints,
            main,
            start: None,
            end: None,
            error_message_attributes,
            instruction_locations,
            identifiers,
            reference_manager: Self::get_reference_list(&reference_manager),
        };
        Ok(Self {
            shared_program_data: Arc::new(shared_program_data),
            constants,
            builtins,
        })
    }

    #[cfg(feature = "std")]
    pub fn from_file(path: &Path, entrypoint: Option<&str>) -> Result<Program, ProgramError> {
        let file_content = std::fs::read(path)?;
        deserialize_and_parse_program(&file_content, entrypoint)
    }

    pub fn from_bytes(bytes: &[u8], entrypoint: Option<&str>) -> Result<Program, ProgramError> {
        deserialize_and_parse_program(bytes, entrypoint)
    }

    pub fn prime(&self) -> &str {
        _ = self;
        PRIME_STR
    }

    pub fn iter_builtins(&self) -> impl Iterator<Item = &BuiltinName> {
        self.builtins.iter()
    }

    pub fn iter_data(&self) -> impl Iterator<Item = &MaybeRelocatable> {
        self.shared_program_data.data.iter()
    }

    pub fn data_len(&self) -> usize {
        self.shared_program_data.data.len()
    }

    pub fn builtins_len(&self) -> usize {
        self.builtins.len()
    }

    pub fn get_identifier(&self, id: &str) -> Option<&Identifier> {
        self.shared_program_data.identifiers.get(id)
    }

    pub fn iter_identifiers(&self) -> impl Iterator<Item = (&str, &Identifier)> {
        self.shared_program_data
            .identifiers
            .iter()
            .map(|(cairo_type, identifier)| (cairo_type.as_str(), identifier))
    }

    pub(crate) fn get_reference_list(reference_manager: &ReferenceManager) -> Vec<HintReference> {
        reference_manager
            .references
            .iter()
            .map(|r| {
                HintReference {
                    offset1: r.value_address.offset1.clone(),
                    offset2: r.value_address.offset2.clone(),
                    dereference: r.value_address.dereference,
                    // only store `ap` tracking data if the reference is referred to it
                    ap_tracking_data: match (&r.value_address.offset1, &r.value_address.offset2) {
                        (OffsetValue::Reference(Register::AP, _, _), _)
                        | (_, OffsetValue::Reference(Register::AP, _, _)) => {
                            Some(r.ap_tracking_data.clone())
                        }
                        _ => None,
                    },
                    cairo_type: Some(r.value_address.value_type.clone()),
                }
            })
            .collect()
    }
}

impl Default for Program {
    fn default() -> Self {
        Self {
            shared_program_data: Arc::new(SharedProgramData::default()),
            constants: HashMap::new(),
            builtins: Vec::new(),
        }
    }
}

#[cfg(feature = "cairo-1-hints")]
// Note: This Program will only work when using run_from_entrypoint, and the Cairo1Hintprocesso
impl TryFrom<CasmContractClass> for Program {
    type Error = ProgramError;
    fn try_from(value: CasmContractClass) -> Result<Self, ProgramError> {
        let data = value
            .bytecode
            .iter()
            .map(|x| MaybeRelocatable::from(Felt252::from(&x.value)))
            .collect();
        //Hint data is going to be hosted processor-side, hints field will only store the pc where hints are located.
        // Only one pc will be stored, so the hint processor will be responsible for executing all hints for a given pc
        let hints = value
            .hints
            .iter()
            .map(|(x, _)| {
                (
                    *x,
                    vec![HintParams {
                        code: x.to_string(),
                        accessible_scopes: Vec::new(),
                        flow_tracking_data: FlowTrackingData {
                            ap_tracking: ApTracking::default(),
                            reference_ids: HashMap::new(),
                        },
                    }],
                )
            })
            .collect();
        let error_message_attributes = Vec::new();
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };
        Self::new(
            vec![],
            data,
            None,
            hints,
            reference_manager,
            HashMap::new(),
            error_message_attributes,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::serde::deserialize_program::{ApTracking, FlowTrackingData};
    use crate::utils::test_utils::*;
    use felt::felt_str;
    use num_traits::Zero;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn new() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<BuiltinName> = Vec::new();
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
        assert_eq!(program.shared_program_data.data, data);
        assert_eq!(program.shared_program_data.main, None);
        assert_eq!(program.shared_program_data.identifiers, HashMap::new());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn new_program_with_identifiers() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<BuiltinName> = Vec::new();

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
                value: Some(Felt252::zero()),
                full_name: None,
                members: None,
                cairo_type: None,
            },
        );

        let program = Program::new(
            builtins.clone(),
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
        assert_eq!(program.shared_program_data.data, data);
        assert_eq!(program.shared_program_data.main, None);
        assert_eq!(program.shared_program_data.identifiers, identifiers);
        assert_eq!(
            program.constants,
            [("__main__.main.SIZEOF_LOCALS", Felt252::zero())]
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect::<HashMap<_, _>>(),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_prime() {
        let program = Program::default();
        assert_eq!(PRIME_STR, program.prime());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn iter_builtins() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<_> = vec![BuiltinName::range_check, BuiltinName::bitwise];
        let data: Vec<_> = vec![
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(1000),
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(2000),
            mayberelocatable!(5201798304953696256),
            mayberelocatable!(2345108766317314046),
        ];

        let program = Program::new(
            builtins.clone(),
            data,
            None,
            HashMap::new(),
            reference_manager,
            HashMap::new(),
            Vec::new(),
            None,
        )
        .unwrap();

        assert_eq!(
            program.iter_builtins().cloned().collect::<Vec<_>>(),
            builtins
        );

        assert_eq!(program.builtins_len(), 2);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn iter_data() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<BuiltinName> = Vec::new();
        let data: Vec<MaybeRelocatable> = vec![
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(1000),
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(2000),
            mayberelocatable!(5201798304953696256),
            mayberelocatable!(2345108766317314046),
        ];

        let program = Program::new(
            builtins,
            data.clone(),
            None,
            HashMap::new(),
            reference_manager,
            HashMap::new(),
            Vec::new(),
            None,
        )
        .unwrap();

        assert_eq!(program.iter_data().cloned().collect::<Vec<_>>(), data);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn data_len() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<BuiltinName> = Vec::new();
        let data: Vec<MaybeRelocatable> = vec![
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(1000),
            mayberelocatable!(5189976364521848832),
            mayberelocatable!(2000),
            mayberelocatable!(5201798304953696256),
            mayberelocatable!(2345108766317314046),
        ];

        let program = Program::new(
            builtins,
            data.clone(),
            None,
            HashMap::new(),
            reference_manager,
            HashMap::new(),
            Vec::new(),
            None,
        )
        .unwrap();

        assert_eq!(program.data_len(), data.len());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_identifier() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<BuiltinName> = Vec::new();

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
                value: Some(Felt252::zero()),
                full_name: None,
                members: None,
                cairo_type: None,
            },
        );

        let program = Program::new(
            builtins,
            data,
            None,
            HashMap::new(),
            reference_manager,
            identifiers.clone(),
            Vec::new(),
            None,
        )
        .unwrap();

        assert_eq!(
            program.get_identifier("__main__.main"),
            identifiers.get("__main__.main"),
        );
        assert_eq!(
            program.get_identifier("__main__.main.SIZEOF_LOCALS"),
            identifiers.get("__main__.main.SIZEOF_LOCALS"),
        );
        assert_eq!(
            program.get_identifier("missing"),
            identifiers.get("missing"),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn iter_identifiers() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<BuiltinName> = Vec::new();

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
                value: Some(Felt252::zero()),
                full_name: None,
                members: None,
                cairo_type: None,
            },
        );

        let program = Program::new(
            builtins,
            data,
            None,
            HashMap::new(),
            reference_manager,
            identifiers.clone(),
            Vec::new(),
            None,
        )
        .unwrap();

        let collected_identifiers: HashMap<_, _> = program
            .iter_identifiers()
            .map(|(cairo_type, identifier)| (cairo_type.to_string(), identifier.clone()))
            .collect();

        assert_eq!(collected_identifiers, identifiers);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn new_program_with_invalid_identifiers() {
        let reference_manager = ReferenceManager {
            references: Vec::new(),
        };

        let builtins: Vec<BuiltinName> = Vec::new();

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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_program_test() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/manually_compiled/valid_program_a.json"),
            Some("main"),
        )
        .unwrap();

        let builtins: Vec<BuiltinName> = Vec::new();
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
                value: Some(Felt252::zero()),
                full_name: None,
                members: None,
                cairo_type: None,
            },
        );

        assert_eq!(program.builtins, builtins);
        assert_eq!(program.shared_program_data.data, data);
        assert_eq!(program.shared_program_data.main, Some(0));
        assert_eq!(program.shared_program_data.identifiers, identifiers);
    }

    /// Deserialize a program without an entrypoint.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_program_without_entrypoint_test() {
        let program = Program::from_bytes(
            include_bytes!("../../../cairo_programs/manually_compiled/valid_program_a.json"),
            None,
        )
        .unwrap();

        let builtins: Vec<BuiltinName> = Vec::new();

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
                value: Some(Felt252::zero()),
                full_name: None,
                members: None,
                cairo_type: None,
            },
        );

        assert_eq!(program.builtins, builtins);
        assert_eq!(program.shared_program_data.data, data);
        assert_eq!(program.shared_program_data.main, None);
        assert_eq!(program.shared_program_data.identifiers, identifiers);
        assert_eq!(
            program.shared_program_data.error_message_attributes,
            error_message_attributes
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deserialize_program_constants_test() {
        let program = Program::from_bytes(
            include_bytes!(
                "../../../cairo_programs/manually_compiled/deserialize_constant_test.json"
            ),
            Some("main"),
        )
        .unwrap();

        let constants = [
            ("__main__.compare_abs_arrays.SIZEOF_LOCALS", Felt252::zero()),
            (
                "starkware.cairo.common.cairo_keccak.packed_keccak.ALL_ONES",
                felt_str!(
                    "3618502788666131106986593281521497120414687020801267626233049500247285301247"
                ),
            ),
            (
                "starkware.cairo.common.cairo_keccak.packed_keccak.BLOCK_SIZE",
                Felt252::new(3),
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
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn default_program() {
        let shared_program_data = SharedProgramData {
            data: Vec::new(),
            hints: HashMap::new(),
            main: None,
            start: None,
            end: None,
            error_message_attributes: Vec::new(),
            instruction_locations: None,
            identifiers: HashMap::new(),
            reference_manager: Program::get_reference_list(&ReferenceManager {
                references: Vec::new(),
            }),
        };
        let program = Program {
            shared_program_data: Arc::new(shared_program_data),
            constants: HashMap::new(),
            builtins: Vec::new(),
        };

        assert_eq!(program, Program::default());
    }
}
