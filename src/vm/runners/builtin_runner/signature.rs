use std::prelude::v1::*;

use crate::{
    math_utils::safe_div_usize,
    types::{
        instance_definitions::ecdsa_instance_def::EcdsaInstanceDef,
        relocatable::{MaybeRelocatable, Relocatable},
    },
    vm::{
        errors::{memory_errors::MemoryError, runner_errors::RunnerError},
        vm_core::VirtualMachine,
        vm_memory::{
            memory::{Memory, ValidationRule},
            memory_segments::MemorySegmentManager,
        },
    },
};
use felt::Felt;
use num_integer::{div_ceil, Integer};
use num_traits::ToPrimitive;
use starknet_crypto::{verify, FieldElement, Signature};
use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Debug, Clone)]
pub struct SignatureBuiltinRunner {
    included: bool,
    ratio: u32,
    base: isize,
    pub(crate) cells_per_instance: u32,
    pub(crate) n_input_cells: u32,
    _total_n_bits: u32,
    pub(crate) stop_ptr: Option<usize>,
    instances_per_component: u32,
    signatures: Rc<RefCell<HashMap<Relocatable, Signature>>>,
}

impl SignatureBuiltinRunner {
    pub(crate) fn new(instance_def: &EcdsaInstanceDef, included: bool) -> Self {
        SignatureBuiltinRunner {
            base: 0,
            included,
            ratio: instance_def.ratio,
            cells_per_instance: 2,
            n_input_cells: 2,
            _total_n_bits: 251,
            stop_ptr: None,
            instances_per_component: 1,
            signatures: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    pub fn add_signature(
        &mut self,
        relocatable: Relocatable,
        (r, s): &(Felt, Felt),
    ) -> Result<(), MemoryError> {
        let r_string = r.to_str_radix(10);
        let s_string = s.to_str_radix(10);
        let (r_felt, s_felt) = (
            FieldElement::from_dec_str(&r_string)
                .map_err(|_| MemoryError::FailedStringToFieldElementConversion(r_string))?,
            FieldElement::from_dec_str(&s_string)
                .map_err(|_| MemoryError::FailedStringToFieldElementConversion(s_string))?,
        );

        let signature = Signature {
            r: r_felt,
            s: s_felt,
        };

        self.signatures
            .borrow_mut()
            .entry(relocatable)
            .or_insert(signature);

        Ok(())
    }
}

impl SignatureBuiltinRunner {
    pub fn initialize_segments(
        &mut self,
        segments: &mut MemorySegmentManager,
        memory: &mut Memory,
    ) {
        self.base = segments.add(memory).segment_index
    }

    pub fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        if self.included {
            vec![MaybeRelocatable::from((self.base, 0))]
        } else {
            vec![]
        }
    }

    pub fn base(&self) -> isize {
        self.base
    }
    pub fn add_validation_rule(&self, memory: &mut Memory) -> Result<(), RunnerError> {
        let cells_per_instance = self.cells_per_instance;
        let signatures = Rc::clone(&self.signatures);
        let rule: ValidationRule = ValidationRule(Box::new(
            move |memory: &Memory,
                  address: &Relocatable|
                  -> Result<Vec<Relocatable>, MemoryError> {
                let address_offset = address.offset.mod_floor(&(cells_per_instance as usize));
                let mem_addr_sum = memory.get(&(address + 1_i32));
                let mem_addr_less = if address.offset > 0 {
                    memory.get(
                        &address
                            .sub_usize(1)
                            .map_err(|_| MemoryError::NumOutOfBounds)?,
                    )
                } else {
                    Ok(None)
                };
                let (pubkey_addr, msg_addr) = match (address_offset, mem_addr_sum, mem_addr_less) {
                    (0, Ok(Some(_element)), _) => {
                        let pubkey_addr = address;
                        let msg_addr = address + 1_i32;
                        (*pubkey_addr, msg_addr)
                    }
                    (1, _, Ok(Some(_element))) if address.offset > 0 => {
                        let pubkey_addr = address
                            .sub_usize(1)
                            .map_err(|_| MemoryError::EffectiveSizesNotCalled)?;
                        let msg_addr = address;
                        (pubkey_addr, *msg_addr)
                    }
                    _ => return Ok(Vec::new()),
                };

                let msg = memory
                    .get_integer(&msg_addr)
                    .map_err(|_| MemoryError::FoundNonInt)?;
                let pub_key = memory
                    .get_integer(&pubkey_addr)
                    .map_err(|_| MemoryError::FoundNonInt)?;
                let signatures_map = signatures.borrow();
                let signature = signatures_map
                    .get(&pubkey_addr)
                    .ok_or(MemoryError::SignatureNotFound)?;
                let public_key = FieldElement::from_dec_str(&pub_key.to_str_radix(10))
                    .map_err(|_| MemoryError::ErrorParsingPubKey(pub_key.to_str_radix(10)))?;
                let (r, s) = (signature.r, signature.s);
                let message = FieldElement::from_dec_str(&msg.to_str_radix(10))
                    .map_err(|_| MemoryError::ErrorRetrievingMessage(msg.to_str_radix(10)))?;
                let was_verified = verify(&public_key, &message, &r, &s)
                    .map_err(|_| MemoryError::ErrorVerifyingSignature)?;
                if was_verified {
                    Ok(vec![])
                } else {
                    Err(MemoryError::InvalidSignature)
                }
            },
        ));
        memory.add_validation_rule(
            self.base
                .to_usize()
                .ok_or(RunnerError::RunnerInTemporarySegment(self.base))?,
            rule,
        );
        Ok(())
    }

    pub fn deduce_memory_cell(
        &self,
        _address: &Relocatable,
        _memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        Ok(None)
    }

    pub fn as_any(&self) -> &dyn Any {
        self
    }

    pub fn ratio(&self) -> u32 {
        self.ratio
    }

    pub fn get_allocated_memory_units(&self, vm: &VirtualMachine) -> Result<usize, MemoryError> {
        let value = safe_div_usize(vm.current_step, self.ratio as usize)
            .map_err(|_| MemoryError::ErrorCalculatingMemoryUnits)?;
        Ok(self.cells_per_instance as usize * value)
    }

    pub fn get_memory_segment_addresses(&self) -> (isize, Option<usize>) {
        (self.base, self.stop_ptr)
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        let base = self.base();
        segments
            .get_segment_used_size(
                base.try_into()
                    .map_err(|_| MemoryError::AddressInTemporarySegment(base))?,
            )
            .ok_or(MemoryError::MissingSegmentUsedSizes)
    }

    pub fn get_used_cells_and_allocated_size(
        &self,
        vm: &VirtualMachine,
    ) -> Result<(usize, usize), MemoryError> {
        let ratio = self.ratio as usize;
        let cells_per_instance = self.cells_per_instance;
        let min_step = ratio * self.instances_per_component as usize;
        if vm.current_step < min_step {
            Err(MemoryError::InsufficientAllocatedCells)
        } else {
            let used = self.get_used_cells(&vm.segments)?;
            let size = cells_per_instance as usize
                * safe_div_usize(vm.current_step, ratio)
                    .map_err(|_| MemoryError::InsufficientAllocatedCells)?;
            if used > size {
                return Err(MemoryError::InsufficientAllocatedCells);
            }
            Ok((used, size))
        }
    }

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        let used_cells = self.get_used_cells(segments)?;
        Ok(div_ceil(used_cells, self.cells_per_instance as usize))
    }

    pub fn final_stack(
        &mut self,
        segments: &MemorySegmentManager,
        memory: &Memory,
        pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        if self.included {
            if let Ok(stop_pointer) = memory
                .get_relocatable(&(pointer.sub_usize(1)).map_err(|_| RunnerError::FinalStack)?)
            {
                if self.base() != stop_pointer.segment_index {
                    return Err(RunnerError::InvalidStopPointer("ecdsa".to_string()));
                }
                let stop_ptr = stop_pointer.offset;
                let num_instances = self
                    .get_used_instances(segments)
                    .map_err(|_| RunnerError::FinalStack)?;
                let used_cells = num_instances * self.cells_per_instance as usize;
                if stop_ptr != used_cells {
                    return Err(RunnerError::InvalidStopPointer("ecdsa".to_string()));
                }

                self.stop_ptr = Some(stop_ptr);
                Ok(pointer.sub_usize(1).map_err(|_| RunnerError::FinalStack)?)
            } else {
                Err(RunnerError::FinalStack)
            }
        } else {
            let stop_ptr = self.base() as usize;
            self.stop_ptr = Some(stop_ptr);
            Ok(pointer)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        types::instance_definitions::ecdsa_instance_def::EcdsaInstanceDef,
        utils::test_utils::*,
        vm::{
            errors::memory_errors::MemoryError,
            runners::builtin_runner::BuiltinRunner,
            vm_core::VirtualMachine,
            vm_memory::{memory::Memory, memory_segments::MemorySegmentManager},
        },
    };

    #[test]
    fn get_used_cells_and_allocated_size_error() {
        let builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);
        let mut vm = vm!();
        vm.current_step = 100;
        vm.segments.segment_used_sizes = Some(vec![1]);
        assert_eq!(
            builtin.get_used_cells_and_allocated_size(&vm),
            Err(MemoryError::InsufficientAllocatedCells)
        );
    }

    #[test]
    fn get_used_cells_and_allocated_size_valid() {
        let builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::new(10), true);
        let mut vm = vm!();
        vm.current_step = 110;
        vm.segments.segment_used_sizes = Some(vec![1]);
        assert_eq!(builtin.get_used_cells_and_allocated_size(&vm), Ok((1, 22)));
    }

    #[test]
    fn initialize_segments_for_ecdsa() {
        let mut builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);
        let mut segments = MemorySegmentManager::new();
        let mut memory = Memory::new();
        builtin.initialize_segments(&mut segments, &mut memory);
        assert_eq!(builtin.base, 0);
    }

    #[test]
    fn get_used_instances() {
        let builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    fn final_stack() {
        let mut builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);

        let mut vm = vm!();

        vm.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin
                .final_stack(&vm.segments, &vm.memory, pointer)
                .unwrap(),
            Relocatable::from((2, 1))
        );
    }

    #[test]
    fn final_stack_error_stop_pointer() {
        let mut builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);

        let mut vm = vm!();

        vm.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![999]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, &vm.memory, pointer),
            Err(RunnerError::InvalidStopPointer("ecdsa".to_string()))
        );
    }

    #[test]
    fn final_stack_error_non_relocatable() {
        let mut builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);

        let mut vm = vm!();

        vm.memory = memory![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, &vm.memory, pointer),
            Err(RunnerError::FinalStack)
        );
    }

    #[test]
    fn get_memory_segment_addresses() {
        let builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);

        assert_eq!(builtin.get_memory_segment_addresses(), (0, None));
    }

    #[test]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::Signature(SignatureBuiltinRunner::new(
            &EcdsaInstanceDef::default(),
            true,
        ));
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    fn get_memory_accesses_empty() {
        let builtin = BuiltinRunner::Signature(SignatureBuiltinRunner::new(
            &EcdsaInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    fn get_memory_accesses() {
        let builtin = BuiltinRunner::Signature(SignatureBuiltinRunner::new(
            &EcdsaInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Ok(vec![
                (builtin.base(), 0).into(),
                (builtin.base(), 1).into(),
                (builtin.base(), 2).into(),
                (builtin.base(), 3).into(),
            ]),
        );
    }

    #[test]
    fn get_used_cells_missing_segment_used_sizes() {
        let builtin = BuiltinRunner::Signature(SignatureBuiltinRunner::new(
            &EcdsaInstanceDef::default(),
            true,
        ));
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    fn get_used_cells_empty() {
        let builtin = BuiltinRunner::Signature(SignatureBuiltinRunner::new(
            &EcdsaInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    fn get_used_cells() {
        let builtin = BuiltinRunner::Signature(SignatureBuiltinRunner::new(
            &EcdsaInstanceDef::default(),
            true,
        ));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    fn get_initial_stack_for_range_check_with_base() {
        let mut builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);
        builtin.base = 1;
        let initial_stack = builtin.initial_stack();
        assert_eq!(
            initial_stack[0].clone(),
            MaybeRelocatable::RelocatableValue((builtin.base(), 0).into())
        );
        assert_eq!(initial_stack.len(), 1);
    }

    #[test]
    fn initial_stack_not_included_test() {
        let ecdsa_builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), false);
        assert_eq!(ecdsa_builtin.initial_stack(), Vec::new())
    }

    #[test]
    fn deduce_memory_cell_test() {
        let memory = Memory::new();
        let builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(&Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn test_ratio() {
        let builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);
        assert_eq!(builtin.ratio(), 512);
    }

    #[test]
    fn test_base() {
        let builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);
        assert_eq!(builtin.base(), 0);
    }

    #[test]
    fn test_get_memory_segment_addresses() {
        let builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);

        assert_eq!(builtin.get_memory_segment_addresses(), (0, None));
    }

    #[test]
    fn deduce_memory_cell() {
        let memory = Memory::new();
        let builtin = SignatureBuiltinRunner::new(&EcdsaInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(&Relocatable::from((0, 5)), &memory);
        assert_eq!(result, Ok(None));
    }
}
