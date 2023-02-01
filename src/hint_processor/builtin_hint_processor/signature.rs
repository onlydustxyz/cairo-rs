use std::prelude::v1::*;

use crate::{
    hint_processor::{
        builtin_hint_processor::hint_utils::{get_integer_from_var_name, get_ptr_from_var_name},
        hint_processor_definition::HintReference,
    },
    serde::deserialize_program::ApTracking,
    vm::{
        errors::{hint_errors::HintError, vm_errors::VirtualMachineError},
        vm_core::VirtualMachine,
    },
};
use std::collections::HashMap;

pub fn verify_ecdsa_signature(
    vm: &mut VirtualMachine,
    ids_data: &HashMap<String, HintReference>,
    ap_tracking: &ApTracking,
) -> Result<(), HintError> {
    let signature_r =
        get_integer_from_var_name("signature_r", vm, ids_data, ap_tracking)?.into_owned();
    let signature_s =
        get_integer_from_var_name("signature_s", vm, ids_data, ap_tracking)?.into_owned();
    let ecdsa_ptr = get_ptr_from_var_name("ecdsa_ptr", vm, ids_data, ap_tracking)?;
    let ecdsa_builtin = &mut vm.get_signature_builtin()?;
    ecdsa_builtin
        .add_signature(ecdsa_ptr, &(signature_r, signature_s))
        .map_err(VirtualMachineError::MemoryError)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        any_box,
        hint_processor::{
            builtin_hint_processor::{
                builtin_hint_processor_definition::{BuiltinHintProcessor, HintProcessorData},
                hint_code::VERIFY_ECDSA_SIGNATURE,
            },
            hint_processor_definition::HintProcessor,
        },
        types::{
            exec_scope::ExecutionScopes,
            instance_definitions::ecdsa_instance_def::EcdsaInstanceDef,
            relocatable::MaybeRelocatable,
        },
        utils::test_utils::*,
        vm::{
            errors::memory_errors::MemoryError, runners::builtin_runner::SignatureBuiltinRunner,
            vm_memory::memory::Memory,
        },
    };
    use std::any::Any;

    #[test]
    fn verify_ecdsa_signature_valid() {
        let mut vm = vm!();
        vm.builtin_runners = vec![(
            "ecdsa".to_string(),
            SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true).into(),
        )];
        vm.memory = memory![
            ((1, 0), (2, 0)),
            (
                (1, 1),
                (
                    "3086480810278599376317923499561306189851900463386393948998357832163236918254",
                    10
                )
            ),
            (
                (1, 2),
                (
                    "598673427589502599949712887611119751108407514580626464031881322743364689811",
                    10
                )
            )
        ];
        vm.run_context.fp = 3;
        let ids_data = ids_data!["ecdsa_ptr", "signature_r", "signature_s"];
        assert_eq!(run_hint!(vm, ids_data, VERIFY_ECDSA_SIGNATURE), Ok(()));
    }
}
