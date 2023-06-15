use crate::math_utils::safe_div_usize;
use crate::stdlib::{cell::RefCell, collections::HashMap, prelude::*};
use crate::types::instance_definitions::keccak_instance_def::KeccakInstanceDef;
use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::MemoryError;
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::vm_core::VirtualMachine;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;
use felt::Felt252;
use num_bigint::BigUint;
use num_integer::div_ceil;
use num_traits::One;

use super::KECCAK_BUILTIN_NAME;

const KECCAK_FELT_BYTE_SIZE: usize = 25; // 200 / 8

#[derive(Debug, Clone)]
pub struct KeccakBuiltinRunner {
    ratio: Option<u32>,
    pub base: usize,
    pub(crate) cells_per_instance: u32,
    pub(crate) n_input_cells: u32,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) included: bool,
    state_rep: Vec<u32>,
    pub(crate) instances_per_component: u32,
    cache: RefCell<HashMap<Relocatable, Felt252>>,
}

impl KeccakBuiltinRunner {
    pub(crate) fn new(instance_def: &KeccakInstanceDef, included: bool) -> Self {
        KeccakBuiltinRunner {
            base: 0,
            ratio: instance_def.ratio,
            n_input_cells: instance_def._state_rep.len() as u32,
            cells_per_instance: instance_def.cells_per_builtin(),
            stop_ptr: None,
            included,
            instances_per_component: instance_def._instance_per_component,
            state_rep: instance_def._state_rep.clone(),
            cache: RefCell::new(HashMap::new()),
        }
    }

    pub fn initialize_segments(&mut self, segments: &mut MemorySegmentManager) {
        self.base = segments.add().segment_index as usize // segments.add() always returns a positive index
    }

    pub fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        if self.included {
            vec![MaybeRelocatable::from((self.base as isize, 0))]
        } else {
            vec![]
        }
    }

    pub fn base(&self) -> usize {
        self.base
    }

    pub fn ratio(&self) -> Option<u32> {
        self.ratio
    }

    pub fn add_validation_rule(&self, _memory: &mut Memory) {}

    pub fn deduce_memory_cell(
        &self,
        address: Relocatable,
        memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        let index = address.offset % self.cells_per_instance as usize;
        if index < self.n_input_cells as usize {
            return Ok(None);
        }
        if let Some(felt) = self.cache.borrow().get(&address) {
            return Ok(Some(felt.into()));
        }
        let first_input_addr = (address - index)?;
        let first_output_addr = (first_input_addr + self.n_input_cells as usize)?;

        let mut input_felts = vec![];

        for i in 0..self.n_input_cells as usize {
            let val = match memory.get(&(first_input_addr + i)?) {
                Some(value) => {
                    let num = value
                        .get_int_ref()
                        .ok_or(RunnerError::BuiltinExpectedInteger(
                            KECCAK_BUILTIN_NAME,
                            (first_input_addr + i)?,
                        ))?;
                    if num >= &(Felt252::one() << self.state_rep[i]) {
                        return Err(RunnerError::IntegerBiggerThanPowerOfTwo(
                            (first_input_addr + i)?,
                            self.state_rep[i],
                            num.clone(),
                        ));
                    }
                    num.clone()
                }
                _ => return Ok(None),
            };

            input_felts.push(val)
        }

        let input_message: Vec<u8> = input_felts
            .iter()
            .flat_map(|x| Self::right_pad(&x.to_biguint().to_bytes_le(), KECCAK_FELT_BYTE_SIZE))
            .collect();
        let keccak_result = Self::keccak_f(&input_message)?;

        let mut start_index = 0_usize;
        for (i, bits) in self.state_rep.iter().enumerate() {
            let end_index = start_index + *bits as usize / 8;
            self.cache.borrow_mut().insert(
                (first_output_addr + i)?,
                Felt252::from(BigUint::from_bytes_le(
                    &keccak_result[start_index..end_index],
                )),
            );
            start_index = end_index;
        }
        Ok(self.cache.borrow().get(&address).map(|x| x.into()))
    }

    pub fn get_memory_segment_addresses(&self) -> (usize, Option<usize>) {
        (self.base, self.stop_ptr)
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        segments
            .get_segment_used_size(self.base())
            .ok_or(MemoryError::MissingSegmentUsedSizes)
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
        pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        if self.included {
            let stop_pointer_addr =
                (pointer - 1).map_err(|_| RunnerError::NoStopPointer(KECCAK_BUILTIN_NAME))?;
            let stop_pointer = segments
                .memory
                .get_relocatable(stop_pointer_addr)
                .map_err(|_| RunnerError::NoStopPointer(KECCAK_BUILTIN_NAME))?;
            if self.base as isize != stop_pointer.segment_index {
                return Err(RunnerError::InvalidStopPointerIndex(
                    KECCAK_BUILTIN_NAME,
                    stop_pointer,
                    self.base,
                ));
            }
            let stop_ptr = stop_pointer.offset;
            let num_instances = self.get_used_instances(segments)?;
            let used = num_instances * self.cells_per_instance as usize;
            if stop_ptr != used {
                return Err(RunnerError::InvalidStopPointer(
                    KECCAK_BUILTIN_NAME,
                    Relocatable::from((self.base as isize, used)),
                    Relocatable::from((self.base as isize, stop_ptr)),
                ));
            }
            self.stop_ptr = Some(stop_ptr);
            Ok(stop_pointer_addr)
        } else {
            let stop_ptr = self.base;
            self.stop_ptr = Some(stop_ptr);
            Ok(pointer)
        }
    }

    pub fn get_memory_accesses(
        &self,
        vm: &VirtualMachine,
    ) -> Result<Vec<Relocatable>, MemoryError> {
        let segment_size = vm
            .segments
            .get_segment_size(self.base)
            .ok_or(MemoryError::MissingSegmentUsedSizes)?;

        Ok((0..segment_size)
            .map(|i| (self.base as isize, i).into())
            .collect())
    }

    pub fn get_used_diluted_check_units(&self, diluted_n_bits: u32) -> usize {
        // The diluted cells are:
        // state - 25 rounds times 1600 elements.
        // parity - 24 rounds times 1600/5 elements times 3 auxiliaries.
        // after_theta_rho_pi - 24 rounds times 1600 elements.
        // theta_aux - 24 rounds times 1600 elements.
        // chi_iota_aux - 24 rounds times 1600 elements times 2 auxiliaries.
        // In total 25 * 1600 + 24 * 320 * 3 + 24 * 1600 + 24 * 1600 + 24 * 1600 * 2 = 216640.
        // But we actually allocate 4 virtual columns, of dimensions 64 * 1024, in which we embed the
        // real cells, and we don't free the unused ones.
        // So the real number is 4 * 64 * 1024 = 262144.
        safe_div_usize(262144_usize, diluted_n_bits as usize).unwrap_or(0)
    }

    fn right_pad(bytes: &[u8], final_size: usize) -> Vec<u8> {
        let zeros: Vec<u8> = vec![0; final_size - bytes.len()];
        let mut bytes_vector = bytes.to_vec();
        bytes_vector.extend(zeros);
        bytes_vector
    }

    fn keccak_f(input_message: &[u8]) -> Result<Vec<u8>, RunnerError> {
        let bigint = BigUint::from_bytes_le(input_message);
        let mut keccak_input = bigint.to_u64_digits();
        keccak_input.resize(25, 0);
        // This unwrap wont fail as keccak_input's size is always 25
        let mut keccak_input: [u64; 25] = keccak_input.try_into().unwrap();
        keccak::f1600(&mut keccak_input);
        Ok(keccak_input.iter().flat_map(|x| x.to_le_bytes()).collect())
    }
}

#[cfg(test)]
mod tests {
    use num_traits::Num;

    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::relocatable;
    use crate::stdlib::collections::HashMap;
    use crate::types::program::Program;
    use crate::utils::test_utils::*;
    use crate::vm::runners::cairo_runner::CairoRunner;

    use crate::vm::{
        errors::{memory_errors::MemoryError, runner_errors::RunnerError},
        runners::builtin_runner::BuiltinRunner,
        vm_core::VirtualMachine,
    };

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(Some(10), vec![200; 8]), true).into();

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack() {
        let mut builtin =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(Some(10), vec![200; 8]), true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer).unwrap(),
            Relocatable::from((2, 1))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_stop_pointer() {
        let mut builtin =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(Some(10), vec![200; 8]), true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![992]);

        let pointer = Relocatable::from((2, 2));
        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(
                KECCAK_BUILTIN_NAME,
                relocatable!(0, 992),
                relocatable!(0, 0)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_when_not_included() {
        let mut builtin =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(Some(10), vec![200; 8]), false);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer).unwrap(),
            Relocatable::from((2, 2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_non_relocatable() {
        let mut builtin =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(Some(10), vec![200; 8]), true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::NoStopPointer(KECCAK_BUILTIN_NAME))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(Some(10), vec![200; 8]), true).into();

        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        let program = Program::from_bytes(
            include_bytes!("../../../../cairo_programs/_keccak.json"),
            Some("main"),
        )
        .unwrap();

        let mut cairo_runner = cairo_runner!(program);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(&mut vm).unwrap();

        cairo_runner
            .run_until_pc(address, &mut vm, &mut hint_processor)
            .unwrap();

        assert_eq!(
            builtin.get_used_cells_and_allocated_size(&vm),
            Ok((0, 1072))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(Some(10), vec![200; 8]), true).into();

        let mut vm = vm!();
        vm.current_step = 160;

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(256));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_segment_addresses() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        assert_eq!(builtin.get_memory_segment_addresses(), (0, None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_empty() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Ok(vec![
                (builtin.base() as isize, 0).into(),
                (builtin.base() as isize, 1).into(),
                (builtin.base() as isize, 2).into(),
                (builtin.base() as isize, 3).into(),
            ]),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_missing_segment_used_sizes() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true).into();
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_empty() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initial_stackincluded_test() {
        let keccak_builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        assert_eq!(
            keccak_builtin.initial_stack(),
            vec![mayberelocatable!(0, 0)]
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initial_stack_notincluded_test() {
        let keccak_builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), false);
        assert_eq!(keccak_builtin.initial_stack(), Vec::new())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_memory_valid() {
        let memory = memory![
            ((0, 16), 43),
            ((0, 17), 199),
            ((0, 18), 0),
            ((0, 19), 0),
            ((0, 20), 0),
            ((0, 21), 0),
            ((0, 22), 0),
            ((0, 23), 1),
            ((0, 24), 0),
            ((0, 25), 0),
            ((0, 26), 43),
            ((0, 27), 199),
            ((0, 28), 0),
            ((0, 29), 0),
            ((0, 30), 0),
            ((0, 31), 0),
            ((0, 32), 0),
            ((0, 33), 1),
            ((0, 34), 0),
            ((0, 35), 0)
        ];
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 25)), &memory);
        assert_eq!(
            result,
            Ok(Some(MaybeRelocatable::from(
                Felt252::from_str_radix(
                    "1006979841721999878391288827876533441431370448293338267890891",
                    10
                )
                .unwrap()
            )))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_non_reloc_address_err() {
        let memory = memory![
            ((0, 4), 32),
            ((0, 5), 72),
            ((0, 6), 0),
            ((0, 7), 120),
            ((0, 8), 52)
        ];
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 1)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_offset_lt_input_cell_length_none() {
        let memory = memory![((0, 4), 32)];
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 2)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_expected_integer() {
        let memory = memory![((0, 0), (1, 2))];

        let mut builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        builtin.n_input_cells = 1;
        builtin.cells_per_instance = 100;

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 1)), &memory);

        assert_eq!(
            result,
            Err(RunnerError::BuiltinExpectedInteger(
                KECCAK_BUILTIN_NAME,
                (0, 0).into()
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_missing_input_cells() {
        let memory = memory![((0, 1), (1, 2))];

        let mut builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        builtin.n_input_cells = 1;
        builtin.cells_per_instance = 100;

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 1)), &memory);

        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_input_cell() {
        let memory = memory![((0, 0), (1, 2))];

        let mut builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        builtin.n_input_cells = 1;
        builtin.cells_per_instance = 100;

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 0)), &memory);

        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_get_memory_err() {
        let memory = memory![((0, 35), 0)];

        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 15)), &memory);

        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_memory_int_larger_than_bits() {
        let memory = memory![
            ((0, 16), 43),
            ((0, 17), 199),
            ((0, 18), 0),
            ((0, 19), 0),
            ((0, 20), 0),
            ((0, 21), 0),
            ((0, 22), 0),
            ((0, 23), 1),
            ((0, 24), 0),
            ((0, 25), 0),
            ((0, 26), 43),
            ((0, 27), 199),
            ((0, 28), 0),
            ((0, 29), 0),
            ((0, 30), 0),
            ((0, 31), 0),
            ((0, 32), 0),
            ((0, 33), 1),
            ((0, 34), 0),
            ((0, 35), 0)
        ];

        let keccak_instance = KeccakInstanceDef::new(Some(2048), vec![1; 8]);
        let builtin = KeccakBuiltinRunner::new(&keccak_instance, true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 25)), &memory);

        assert_eq!(
            result,
            Err(RunnerError::IntegerBiggerThanPowerOfTwo(
                (0, 16).into(),
                1,
                43.into()
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_result() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        let result: usize = builtin.get_used_diluted_check_units(16);

        assert_eq!(result, 16384);
    }

    #[test]
    fn right_pad() {
        let num = [1_u8];
        let padded_num = KeccakBuiltinRunner::right_pad(&num, 5);
        assert_eq!(padded_num, vec![1, 0, 0, 0, 0]);
    }

    #[test]
    fn keccak_f() {
        let input_bytes = b"\x01\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x02\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x04\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x05\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x06\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x07\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x08\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        let expected_output_bytes = b"\xf6\x98\x81\xe1\x00!\x1f.\xc4*\x8c\x0c\x7fF\xc8q8\xdf\xb9\xbe\x07H\xca7T1\xab\x16\x17\xa9\x11\xff-L\x87\xb2iY.\x96\x82x\xde\xbb\\up?uz:0\xee\x08\x1b\x15\xd6\n\xab\r\x0b\x87T:w\x0fH\xe7!f},\x08a\xe5\xbe8\x16\x13\x9a?\xad~<9\xf7\x03`\x8b\xd8\xa3F\x8aQ\xf9\n9\xcdD\xb7.X\xf7\x8e\x1f\x17\x9e \xe5i\x01rr\xdf\xaf\x99k\x9f\x8e\x84\\\xday`\xf1``\x02q+\x8e\xad\x96\xd8\xff\xff3<\xb6\x01o\xd7\xa6\x86\x9d\xea\xbc\xfb\x08\xe1\xa3\x1c\x06z\xab@\xa1\xc1\xb1xZ\x92\x96\xc0.\x01\x13g\x93\x87!\xa6\xa8z\x9c@\x0bY'\xe7\xa7Qr\xe5\xc1\xa3\xa6\x88H\xa5\xc0@9k:y\xd1Kw\xd5";
        let output_bytes = KeccakBuiltinRunner::keccak_f(input_bytes);
        assert_eq!(output_bytes, Ok(expected_output_bytes.to_vec()));
    }
}
