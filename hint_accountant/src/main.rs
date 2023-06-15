#![deny(warnings)]
#![forbid(unsafe_code)]
use cairo_vm::{
    hint_processor::{
        builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
        hint_processor_definition::HintProcessor,
    },
    serde::deserialize_program::ApTracking,
    types::exec_scope::ExecutionScopes,
    vm::{errors::hint_errors::HintError, vm_core::VirtualMachine},
    with_std::collections::{HashMap, HashSet},
};
use serde::Deserialize;
use serde_json;
use serde_json::Value;

const WHITELISTS: [&'static str; 15] = [
    include_str!("../whitelists/0.10.3.json"),
    include_str!("../whitelists/0.6.0.json"),
    include_str!("../whitelists/0.8.2.json"),
    include_str!("../whitelists/384_bit_prime_field.json"),
    include_str!("../whitelists/cairo_blake2s.json"),
    include_str!("../whitelists/cairo_keccak.json"),
    include_str!("../whitelists/cairo_secp.json"),
    include_str!("../whitelists/cairo_sha256.json"),
    include_str!("../whitelists/cairo_sha256_arbitrary_input_length.json"),
    include_str!("../whitelists/ec_bigint.json"),
    include_str!("../whitelists/ec_recover.json"),
    include_str!("../whitelists/encode_packed.json"),
    include_str!("../whitelists/latest.json"),
    include_str!("../whitelists/uint256_improvements.json"),
    include_str!("../whitelists/vrf.json"),
];

#[derive(Deserialize)]
struct AllowedHintExpression {
    #[serde(rename(deserialize = "allowed_expressions"))]
    _allowed_expressions: Option<Value>,
    hint_lines: Vec<Box<str>>,
}

#[derive(Deserialize)]
struct Whitelist {
    #[serde(rename(deserialize = "allowed_reference_expressions_for_hint"))]
    allowed_hint_expressions: Vec<AllowedHintExpression>,
}

fn run() {
    let mut vm = VirtualMachine::new(false);
    let mut hint_executor = BuiltinHintProcessor::new_empty();
    let (ap_tracking_data, reference_ids, references, mut exec_scopes, constants) = (
        ApTracking::default(),
        HashMap::new(),
        HashMap::new(),
        ExecutionScopes::new(),
        HashMap::new(),
    );
    let missing_hints: HashSet<_> = WHITELISTS
        .iter()
        .flat_map(|wl| {
            serde_json::from_str::<Whitelist>(wl)
                .unwrap()
                .allowed_hint_expressions
        })
        .map(|ahe| ahe.hint_lines.join("\n"))
        .filter(|h| {
            let hint_data = hint_executor
                .compile_hint(&h, &ap_tracking_data, &reference_ids, &references)
                .expect("this implementation is infallible");
            matches!(
                hint_executor.execute_hint(&mut vm, &mut exec_scopes, &hint_data, &constants),
                Err(HintError::UnknownHint(_)),
            )
        })
        .collect();

    println!("{} missing hints:", missing_hints.len());
    for hint in missing_hints.iter() {
        println!("");
        println!("```");
        println!("%{{");
        println!("{hint}");
        println!("%}}");
        println!("```");
    }
}

fn main() {
    run()
}
