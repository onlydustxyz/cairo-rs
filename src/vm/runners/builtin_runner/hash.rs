use std::any::Any;

use num_bigint::{BigInt, Sign};
use num_integer::Integer;
use starknet_crypto::{pedersen_hash, FieldElement};

use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::runners::builtin_runner::BuiltinRunner;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;

pub struct HashBuiltinRunner {
    pub base: isize,
    _ratio: usize,
    cells_per_instance: usize,
    _n_input_cells: usize,
    _stop_ptr: Option<Relocatable>,
    verified_addresses: Vec<Relocatable>,
}

impl HashBuiltinRunner {
    pub fn new(ratio: usize) -> Self {
        HashBuiltinRunner {
            base: 0,

            _ratio: ratio,
            cells_per_instance: 3,
            _n_input_cells: 2,
            _stop_ptr: None,
            verified_addresses: Vec::new(),
        }
    }
}

impl BuiltinRunner for HashBuiltinRunner {
    fn initialize_segments(&mut self, segments: &mut MemorySegmentManager, memory: &mut Memory) {
        self.base = segments.add(memory).segment_index
    }

    fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        vec![MaybeRelocatable::from((self.base, 0))]
    }

    fn base(&self) -> isize {
        self.base
    }

    fn add_validation_rule(&self, _memory: &mut Memory) -> Result<(), RunnerError> {
        Ok(())
    }

    fn deduce_memory_cell(
        &mut self,
        address: &Relocatable,
        memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        if address.offset.mod_floor(&self.cells_per_instance) != 2
            || self.verified_addresses.contains(address)
        {
            return Ok(None);
        };

        let num_a = memory.get(&MaybeRelocatable::RelocatableValue(Relocatable {
            segment_index: address.segment_index,
            offset: address.offset - 1,
        }));
        let num_b = memory.get(&MaybeRelocatable::RelocatableValue(Relocatable {
            segment_index: address.segment_index,
            offset: address.offset - 2,
        }));
        if let (Ok(Some(MaybeRelocatable::Int(num_a))), Ok(Some(MaybeRelocatable::Int(num_b)))) = (
            num_a.as_ref().map(|x| x.as_ref().map(|x| x.as_ref())),
            num_b.as_ref().map(|x| x.as_ref().map(|x| x.as_ref())),
        ) {
            self.verified_addresses.push(address.clone());

            //Convert MaybeRelocatable to FieldElement
            let a_string = num_a.to_str_radix(10);
            let b_string = num_b.to_str_radix(10);
            let (y, x) = match (
                FieldElement::from_dec_str(&a_string),
                FieldElement::from_dec_str(&b_string),
            ) {
                (Ok(field_element_a), Ok(field_element_b)) => (field_element_a, field_element_b),
                _ => return Err(RunnerError::FailedStringConversion),
            };
            //Compute pedersen Hash
            let fe_result = pedersen_hash(&x, &y);
            //Convert result from FieldElement to MaybeRelocatable
            let r_byte_slice = fe_result.to_bytes_be();
            let result = BigInt::from_bytes_be(Sign::Plus, &r_byte_slice);
            return Ok(Some(MaybeRelocatable::from(result)));
        }
        Ok(None)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{errors::memory_errors::MemoryError, vm_core::VirtualMachine};
    use crate::{bigint, bigint_str, utils::test_utils::*};
    use num_bigint::Sign;

    #[test]
    fn deduce_memory_cell_pedersen_for_preset_memory_valid() {
        let memory = memory![((0, 3), 32), ((0, 4), 72), ((0, 5), 0)];
        let mut builtin = HashBuiltinRunner::new(8);

        let result = builtin.deduce_memory_cell(&Relocatable::from((0, 5)), &memory);
        assert_eq!(
            result,
            Ok(Some(MaybeRelocatable::from(bigint_str!(
                b"3270867057177188607814717243084834301278723532952411121381966378910183338911"
            ))))
        );
        assert_eq!(builtin.verified_addresses, vec![Relocatable::from((0, 5))]);
    }

    #[test]
    fn deduce_memory_cell_pedersen_for_preset_memory_incorrect_offset() {
        let memory = memory![((0, 4), 32), ((0, 5), 72), ((0, 6), 0)];
        let mut builtin = HashBuiltinRunner::new(8);
        let result = builtin.deduce_memory_cell(&Relocatable::from((0, 6)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn deduce_memory_cell_pedersen_for_preset_memory_no_values_to_hash() {
        let memory = memory![((0, 4), 72), ((0, 5), 0)];
        let mut builtin = HashBuiltinRunner::new(8);
        let result = builtin.deduce_memory_cell(&Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn deduce_memory_cell_pedersen_for_preset_memory_already_computed() {
        let memory = memory![((0, 3), 32), ((0, 4), 72), ((0, 5), 0)];
        let mut builtin = HashBuiltinRunner::new(8);
        builtin.verified_addresses = vec![Relocatable::from((0, 5))];
        let result = builtin.deduce_memory_cell(&Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin = HashBuiltinRunner::new(256);
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    fn get_memory_accesses_empty() {
        let builtin = HashBuiltinRunner::new(256);
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    fn get_memory_accesses() {
        let builtin = HashBuiltinRunner::new(256);
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Ok(vec![
                (builtin.base, 0).into(),
                (builtin.base, 1).into(),
                (builtin.base, 2).into(),
                (builtin.base, 3).into(),
            ]),
        );
    }
}
