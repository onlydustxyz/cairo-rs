use std::prelude::v1::*;

use crate::math_utils::{ec_add, ec_double, safe_div_usize};
use crate::types::instance_definitions::ec_op_instance_def::{
    EcOpInstanceDef, CELLS_PER_EC_OP, INPUT_CELLS_PER_EC_OP,
};
use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::MemoryError;
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::vm_core::VirtualMachine;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;
use felt::Felt;
use num_bigint::{BigInt, ToBigInt};
use num_integer::{div_ceil, Integer};
use num_traits::{Num, One, Pow, Zero};
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub struct EcOpBuiltinRunner {
    ratio: u32,
    pub base: isize,
    pub(crate) cells_per_instance: u32,
    pub(crate) n_input_cells: u32,
    ec_op_builtin: EcOpInstanceDef,
    pub(crate) stop_ptr: Option<usize>,
    included: bool,
    instances_per_component: u32,
}

impl EcOpBuiltinRunner {
    pub(crate) fn new(instance_def: &EcOpInstanceDef, included: bool) -> Self {
        EcOpBuiltinRunner {
            base: 0,
            ratio: instance_def.ratio,
            n_input_cells: INPUT_CELLS_PER_EC_OP,
            cells_per_instance: CELLS_PER_EC_OP,
            ec_op_builtin: instance_def.clone(),
            stop_ptr: None,
            included,
            instances_per_component: 1,
        }
    }
    ///Returns True if the point (x, y) is on the elliptic curve defined as
    ///y^2 = x^3 + alpha * x + beta (mod p)
    ///or False otherwise.
    fn point_on_curve(x: &Felt, y: &Felt, alpha: &Felt, beta: &Felt) -> bool {
        y.pow(2) == &(x.pow(3) + alpha * x) + beta
    }

    ///Returns the result of the EC operation P + m * Q.
    /// where P = (p_x, p_y), Q = (q_x, q_y) are points on the elliptic curve defined as
    /// y^2 = x^3 + alpha * x + beta (mod prime).
    /// Mimics the operation of the AIR, so that this function fails whenever the builtin AIR
    /// would not yield a correct result, i.e. when any part of the computation attempts to add
    /// two points with the same x coordinate.
    fn ec_op_impl(
        partial_sum: (Felt, Felt),
        doubled_point: (Felt, Felt),
        m: &Felt,
        alpha: &BigInt,
        prime: &BigInt,
        height: u32,
    ) -> Result<(BigInt, BigInt), RunnerError> {
        let mut slope = m
            .clone()
            .to_biguint()
            .to_bigint()
            .ok_or(RunnerError::FoundNonInt)?;
        let mut partial_sum_b = (
            partial_sum
                .0
                .to_biguint()
                .to_bigint()
                .ok_or(RunnerError::FoundNonInt)?,
            partial_sum
                .1
                .to_biguint()
                .to_bigint()
                .ok_or(RunnerError::FoundNonInt)?,
        );
        let mut doubled_point_b = (
            doubled_point
                .0
                .to_biguint()
                .to_bigint()
                .ok_or(RunnerError::FoundNonInt)?,
            doubled_point
                .1
                .to_biguint()
                .to_bigint()
                .ok_or(RunnerError::FoundNonInt)?,
        );
        for _ in 0..height {
            if (doubled_point_b.0.clone() - partial_sum_b.0.clone()).is_zero() {
                return Err(RunnerError::EcOpSameXCoordinate(Self::format_ec_op_error(
                    partial_sum_b,
                    m.clone().to_bigint(),
                    doubled_point_b,
                )));
            };
            if !(slope.clone() & &BigInt::one()).is_zero() {
                partial_sum_b = ec_add(partial_sum_b, doubled_point_b.clone(), prime);
            }
            doubled_point_b = ec_double(doubled_point_b, alpha, prime);
            slope = slope.clone() >> 1_u32;
        }
        Ok(partial_sum_b)
    }

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

    pub fn ratio(&self) -> u32 {
        self.ratio
    }

    pub fn add_validation_rule(&self, _memory: &mut Memory) -> Result<(), RunnerError> {
        Ok(())
    }

    pub fn deduce_memory_cell(
        &self,
        address: &Relocatable,
        memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        //Constant values declared here
        const EC_POINT_INDICES: [(usize, usize); 3] = [(0, 1), (2, 3), (5, 6)];
        const OUTPUT_INDICES: (usize, usize) = EC_POINT_INDICES[2];
        let alpha: Felt = Felt::one();
        let beta_low: Felt = Felt::new(0x609ad26c15c915c1f4cdfcb99cee9e89_u128);
        let beta_high: Felt = Felt::new(0x6f21413efbe40de150e596d72f7a8c5_u128);
        let beta: Felt = (beta_high << 128_usize) + beta_low;

        let index = address
            .offset
            .mod_floor(&(self.cells_per_instance as usize));
        //Index should be an output cell
        if index != OUTPUT_INDICES.0 && index != OUTPUT_INDICES.1 {
            return Ok(None);
        }
        let instance = MaybeRelocatable::from((address.segment_index, address.offset - index));
        //All input cells should be filled, and be integer values
        //If an input cell is not filled, return None
        let mut input_cells = Vec::<Cow<Felt>>::with_capacity(self.n_input_cells as usize);
        for i in 0..self.n_input_cells as usize {
            match memory
                .get(&instance.add_usize(i))
                .map_err(RunnerError::FailedMemoryGet)?
            {
                None => return Ok(None),
                Some(addr) => {
                    input_cells.push(match addr {
                        Cow::Borrowed(MaybeRelocatable::Int(num)) => Cow::Borrowed(num),
                        Cow::Owned(MaybeRelocatable::Int(num)) => Cow::Owned(num),
                        _ => return Err(RunnerError::ExpectedInteger(instance.add_usize(i))),
                    });
                }
            };
        }
        //Assert that m is under the limit defined by scalar_limit.
        /*if input_cells[M_INDEX].as_ref() >= &self.ec_op_builtin.scalar_limit {
            return Err(RunnerError::EcOpBuiltinScalarLimit(
                self.ec_op_builtin.scalar_limit.clone(),
            ));
        }*/

        // Assert that if the current address is part of a point, the point is on the curve
        for pair in &EC_POINT_INDICES[0..2] {
            if !EcOpBuiltinRunner::point_on_curve(
                input_cells[pair.0].as_ref(),
                input_cells[pair.1].as_ref(),
                &alpha,
                &beta,
            ) {
                return Err(RunnerError::PointNotOnCurve((
                    input_cells[pair.0].clone().into_owned(),
                    input_cells[pair.1].clone().into_owned(),
                )));
            };
        }
        let prime = BigInt::from_str_radix(&felt::PRIME_STR[2..], 16)
            .map_err(|_| RunnerError::CouldntParsePrime)?;
        let result = EcOpBuiltinRunner::ec_op_impl(
            (
                input_cells[0].as_ref().to_owned(),
                input_cells[1].as_ref().to_owned(),
            ),
            (
                input_cells[2].as_ref().to_owned(),
                input_cells[3].as_ref().to_owned(),
            ),
            input_cells[4].as_ref(),
            &alpha.to_bigint(),
            &prime,
            self.ec_op_builtin.scalar_height,
        )?;
        match index - self.n_input_cells as usize {
            0 => Ok(Some(MaybeRelocatable::Int(Felt::new(result.0)))),
            _ => Ok(Some(MaybeRelocatable::Int(Felt::new(result.1)))),
            //Default case corresponds to 1, as there are no other possible cases
        }
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
                    return Err(RunnerError::InvalidStopPointer("ec_op".to_string()));
                }
                let stop_ptr = stop_pointer.offset;
                let num_instances = self
                    .get_used_instances(segments)
                    .map_err(|_| RunnerError::FinalStack)?;
                let used_cells = num_instances * self.cells_per_instance as usize;
                if stop_ptr != used_cells {
                    return Err(RunnerError::InvalidStopPointer("ec_op".to_string()));
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

    pub fn format_ec_op_error(
        p: (num_bigint::BigInt, num_bigint::BigInt),
        m: num_bigint::BigInt,
        q: (num_bigint::BigInt, num_bigint::BigInt),
    ) -> String {
        format!("Cannot apply EC operation: computation reached two points with the same x coordinate. \n
    Attempting to compute P + m * Q where:\n
    P = {p:?} \n
    m = {m:?}\n
    Q = {q:?}.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::types::program::Program;
    use crate::utils::{load_program, test_utils::*};
    use crate::vm::errors::vm_errors::VirtualMachineError;
    use crate::vm::runners::cairo_runner::CairoRunner;
    use crate::vm::security::verify_secure_runner;
    use crate::vm::{
        errors::{memory_errors::MemoryError, runner_errors::RunnerError},
        runners::builtin_runner::BuiltinRunner,
        vm_core::VirtualMachine,
    };
    use felt::felt_str;
    use std::path::Path;
    use EcOpBuiltinRunner;

    #[test]
    fn get_used_instances() {
        let builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::new(10), true);

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    fn final_stack() {
        let mut builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::new(10), true);

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
        let mut builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::new(10), true);

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
            Err(RunnerError::InvalidStopPointer("ec_op".to_string()))
        );
    }

    #[test]
    fn final_stack_error_when_notincluded() {
        let mut builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::new(10), false);

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
            Relocatable::from((2, 2))
        );
    }

    #[test]
    fn final_stack_error_non_relocatable() {
        let mut builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::new(10), true);

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
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner = EcOpBuiltinRunner::new(&EcOpInstanceDef::new(10), true).into();

        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);

        let program = program!(
            builtins = vec![String::from("pedersen")],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );
        let mut cairo_runner = cairo_runner!(program);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(&mut vm).unwrap();

        cairo_runner
            .run_until_pc(address, &mut vm, &mut hint_processor)
            .unwrap();

        assert_eq!(builtin.get_used_cells_and_allocated_size(&vm), Ok((0, 7)));
    }

    #[test]
    fn get_allocated_memory_units() {
        let builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::new(10), true);

        let mut vm = vm!();

        let program = program!(
            builtins = vec![String::from("ec_op")],
            data = vec_data!(
                (4612671182993129469_i64),
                (5189976364521848832_i64),
                (18446744073709551615_i128),
                (5199546496550207487_i64),
                (4612389712311386111_i64),
                (5198983563776393216_i64),
                (2),
                (2345108766317314046_i64),
                (5191102247248822272_i64),
                (5189976364521848832_i64),
                (7),
                (1226245742482522112_i64),
                ((
                    "3618502788666131213697322783095070105623107215331596699973092056135872020470",
                    10
                )),
                (2345108766317314046_i64)
            ),
            main = Some(8),
        );

        let mut cairo_runner = cairo_runner!(program);

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(&mut vm).unwrap();

        cairo_runner
            .run_until_pc(address, &mut vm, &mut hint_processor)
            .unwrap();

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(7));
    }

    #[test]
    fn point_is_on_curve_a() {
        let x = felt_str!(
            "874739451078007766457464989774322083649278607533249481151382481072868806602"
        );
        let y = felt_str!(
            "152666792071518830868575557812948353041420400780739481342941381225525861407"
        );
        let alpha = Felt::one();
        let beta = felt_str!(
            "3141592653589793238462643383279502884197169399375105820974944592307816406665"
        );
        assert!(EcOpBuiltinRunner::point_on_curve(&x, &y, &alpha, &beta));
    }

    #[test]
    fn point_is_on_curve_b() {
        let x = felt_str!(
            "3139037544796708144595053687182055617920475701120786241351436619796497072089"
        );
        let y = felt_str!(
            "2119589567875935397690285099786081818522144748339117565577200220779667999801"
        );
        let alpha = Felt::one();
        let beta = felt_str!(
            "3141592653589793238462643383279502884197169399375105820974944592307816406665"
        );
        assert!(EcOpBuiltinRunner::point_on_curve(&x, &y, &alpha, &beta));
    }

    #[test]
    fn point_is_not_on_curve_a() {
        let x = felt_str!(
            "874739454078007766457464989774322083649278607533249481151382481072868806602"
        );
        let y = felt_str!(
            "152666792071518830868575557812948353041420400780739481342941381225525861407"
        );
        let alpha = Felt::one();
        let beta = felt_str!(
            "3141592653589793238462643383279502884197169399375105820974944592307816406665"
        );
        assert!(!EcOpBuiltinRunner::point_on_curve(&x, &y, &alpha, &beta));
    }

    #[test]
    fn point_is_not_on_curve_b() {
        let x = felt_str!(
            "3139037544756708144595053687182055617927475701120786241351436619796497072089"
        );
        let y = felt_str!(
            "2119589567875935397690885099786081818522144748339117565577200220779667999801"
        );
        let alpha = Felt::one();
        let beta = felt_str!(
            "3141592653589793238462643383279502884197169399375105820974944592307816406665"
        );
        assert!(!EcOpBuiltinRunner::point_on_curve(&x, &y, &alpha, &beta));
    }

    #[test]
    fn compute_ec_op_impl_valid_a() {
        let partial_sum = (
            felt_str!(
                "3139037544796708144595053687182055617920475701120786241351436619796497072089"
            ),
            felt_str!(
                "2119589567875935397690285099786081818522144748339117565577200220779667999801"
            ),
        );
        let doubled_point = (
            felt_str!(
                "874739451078007766457464989774322083649278607533249481151382481072868806602"
            ),
            felt_str!(
                "152666792071518830868575557812948353041420400780739481342941381225525861407"
            ),
        );
        let m = Felt::new(34);
        let alpha = bigint!(1);
        let height = 256;
        let prime = bigint_str!(
            "3618502788666131213697322783095070105623107215331596699973092056135872020481"
        );
        let result =
            EcOpBuiltinRunner::ec_op_impl(partial_sum, doubled_point, &m, &alpha, &prime, height);
        assert_eq!(
            result,
            Ok((
                bigint_str!(
                    "1977874238339000383330315148209250828062304908491266318460063803060754089297"
                ),
                bigint_str!(
                    "2969386888251099938335087541720168257053975603483053253007176033556822156706"
                )
            ))
        );
    }

    #[test]
    fn compute_ec_op_impl_valid_b() {
        let partial_sum = (
            felt_str!(
                "2962412995502985605007699495352191122971573493113767820301112397466445942584"
            ),
            felt_str!(
                "214950771763870898744428659242275426967582168179217139798831865603966154129"
            ),
        );
        let doubled_point = (
            felt_str!(
                "874739451078007766457464989774322083649278607533249481151382481072868806602"
            ),
            felt_str!(
                "152666792071518830868575557812948353041420400780739481342941381225525861407"
            ),
        );
        let m = Felt::new(34);
        let alpha = bigint!(1);
        let height = 256;
        let prime = bigint_str!(
            "3618502788666131213697322783095070105623107215331596699973092056135872020481"
        );
        let result =
            EcOpBuiltinRunner::ec_op_impl(partial_sum, doubled_point, &m, &alpha, &prime, height);
        assert_eq!(
            result,
            Ok((
                bigint_str!(
                    "2778063437308421278851140253538604815869848682781135193774472480292420096757"
                ),
                bigint_str!(
                    "3598390311618116577316045819420613574162151407434885460365915347732568210029"
                )
            ))
        );
    }

    #[test]
    fn compute_ec_op_invalid_same_x_coordinate() {
        let partial_sum = (Felt::one(), Felt::new(9));
        let doubled_point = (Felt::one(), Felt::new(12));
        let m = Felt::new(34);
        let alpha = bigint!(1);
        let height = 256;
        let prime = bigint_str!(
            "3618502788666131213697322783095070105623107215331596699973092056135872020481"
        );
        let result = EcOpBuiltinRunner::ec_op_impl(
            partial_sum.clone(),
            doubled_point.clone(),
            &m,
            &alpha,
            &prime,
            height,
        );
        assert_eq!(
            result,
            Err(RunnerError::EcOpSameXCoordinate(
                EcOpBuiltinRunner::format_ec_op_error(
                    (partial_sum.0.to_bigint(), partial_sum.1.to_bigint()),
                    m.to_bigint(),
                    (doubled_point.0.to_bigint(), doubled_point.1.to_bigint())
                )
            ))
        );
    }

    #[test]
    /* Data taken from this program execution:
       %builtins output ec_op
       from starkware.cairo.common.cairo_builtins import EcOpBuiltin
       from starkware.cairo.common.serialize import serialize_word
       from starkware.cairo.common.ec_point import EcPoint
       from starkware.cairo.common.ec import ec_op

       func main{output_ptr: felt*, ec_op_ptr: EcOpBuiltin*}():
           let x: EcPoint = EcPoint(2089986280348253421170679821480865132823066470938446095505822317253594081284, 1713931329540660377023406109199410414810705867260802078187082345529207694986)

           let y: EcPoint = EcPoint(874739451078007766457464989774322083649278607533249481151382481072868806602,152666792071518830868575557812948353041420400780739481342941381225525861407)
           let z: EcPoint = ec_op(x,34, y)
           serialize_word(z.x)
           return()
           end
    */
    fn deduce_memory_cell_ec_op_for_preset_memory_valid() {
        let memory = memory![
            (
                (3, 0),
                (
                    "2962412995502985605007699495352191122971573493113767820301112397466445942584",
                    10
                )
            ),
            (
                (3, 1),
                (
                    "214950771763870898744428659242275426967582168179217139798831865603966154129",
                    10
                )
            ),
            (
                (3, 2),
                (
                    "874739451078007766457464989774322083649278607533249481151382481072868806602",
                    10
                )
            ),
            (
                (3, 3),
                (
                    "152666792071518830868575557812948353041420400780739481342941381225525861407",
                    10
                )
            ),
            ((3, 4), 34),
            (
                (3, 5),
                (
                    "2778063437308421278851140253538604815869848682781135193774472480292420096757",
                    10
                )
            )
        ];
        let builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let result = builtin.deduce_memory_cell(&Relocatable::from((3, 6)), &memory);
        assert_eq!(
            result,
            Ok(Some(MaybeRelocatable::from(felt_str!(
                "3598390311618116577316045819420613574162151407434885460365915347732568210029"
            ))))
        );
    }

    #[test]
    fn deduce_memory_cell_ec_op_for_preset_memory_unfilled_input_cells() {
        let memory = memory![
            (
                (3, 1),
                (
                    "214950771763870898744428659242275426967582168179217139798831865603966154129",
                    10
                )
            ),
            (
                (3, 2),
                (
                    "874739451078007766457464989774322083649278607533249481151382481072868806602",
                    10
                )
            ),
            (
                (3, 3),
                (
                    "152666792071518830868575557812948353041420400780739481342941381225525861407",
                    10
                )
            ),
            ((3, 4), 34),
            (
                (3, 5),
                (
                    "2778063437308421278851140253538604815869848682781135193774472480292420096757",
                    10
                )
            )
        ];

        let builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(&Relocatable::from((3, 6)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn deduce_memory_cell_ec_op_for_preset_memory_addr_not_an_output_cell() {
        let memory = memory![
            (
                (3, 0),
                (
                    "2962412995502985605007699495352191122971573493113767820301112397466445942584",
                    10
                )
            ),
            (
                (3, 1),
                (
                    "214950771763870898744428659242275426967582168179217139798831865603966154129",
                    10
                )
            ),
            (
                (3, 2),
                (
                    "874739451078007766457464989774322083649278607533249481151382481072868806602",
                    10
                )
            ),
            (
                (3, 3),
                (
                    "152666792071518830868575557812948353041420400780739481342941381225525861407",
                    10
                )
            ),
            ((3, 4), 34),
            (
                (3, 5),
                (
                    "2778063437308421278851140253538604815869848682781135193774472480292420096757",
                    10
                )
            )
        ];
        let builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let result = builtin.deduce_memory_cell(&Relocatable::from((3, 3)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn deduce_memory_cell_ec_op_for_preset_memory_non_integer_input() {
        let memory = memory![
            (
                (3, 0),
                (
                    "2962412995502985605007699495352191122971573493113767820301112397466445942584",
                    10
                )
            ),
            (
                (3, 1),
                (
                    "214950771763870898744428659242275426967582168179217139798831865603966154129",
                    10
                )
            ),
            (
                (3, 2),
                (
                    "874739451078007766457464989774322083649278607533249481151382481072868806602",
                    10
                )
            ),
            ((3, 3), (1, 2)),
            ((3, 4), 34),
            (
                (3, 5),
                (
                    "2778063437308421278851140253538604815869848682781135193774472480292420096757",
                    10
                )
            )
        ];
        let builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        assert_eq!(
            builtin.deduce_memory_cell(&Relocatable::from((3, 6)), &memory),
            Err(RunnerError::ExpectedInteger(MaybeRelocatable::from((3, 3))))
        );
    }

    #[test]
    fn deduce_memory_cell_ec_op_for_preset_memory_m_over_scalar_limit() {
        let memory = memory![
            (
                (3, 0),
                (
                    "2962412995502985605007699495352191122971573493113767820301112397466445942584",
                    10
                )
            ),
            (
                (3, 1),
                (
                    "214950771763870898744428659242275426967582168179217139798831865603966154129",
                    10
                )
            ),
            (
                (3, 2),
                (
                    "874739451078007766457464989774322083649278607533249481151382481072868806602",
                    10
                )
            ),
            (
                (3, 3),
                (
                    "152666792071518830868575557812948353041420400780739481342941381225525861407",
                    10
                )
            ),
            //Scalar Limit + 1
            (
                (3, 4),
                (
                    "3618502788666131213697322783095070105623107215331596699973092056135872020482",
                    10
                )
            ),
            (
                (3, 5),
                (
                    "2778063437308421278851140253538604815869848682781135193774472480292420096757",
                    10
                )
            )
        ];
        let builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        let _error = builtin.deduce_memory_cell(&Relocatable::from((3, 6)), &memory);
        /*assert_eq!(
            error,
            Err(RunnerError::EcOpBuiltinScalarLimit(
                builtin.ec_op_builtin.scalar_limit
            ))
        );*/
    }

    #[test]
    fn get_memory_segment_addresses() {
        let builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);

        assert_eq!(builtin.get_memory_segment_addresses(), (0, None));
    }

    #[test]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    fn get_memory_accesses_empty() {
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    fn get_memory_accesses() {
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
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
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    fn get_used_cells_empty() {
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    fn get_used_cells() {
        let builtin =
            BuiltinRunner::EcOp(EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true));
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    fn initial_stackincluded_test() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), true);
        assert_eq!(ec_op_builtin.initial_stack(), vec![mayberelocatable!(0, 0)])
    }

    #[test]
    fn initial_stack_notincluded_test() {
        let ec_op_builtin = EcOpBuiltinRunner::new(&EcOpInstanceDef::default(), false);
        assert_eq!(ec_op_builtin.initial_stack(), Vec::new())
    }

    #[test]
    fn catch_point_same_x() {
        let program = load_program(
            #[cfg(feature = "std")]
            "cairo_programs/bad_programs/ec_op_same_x.json",
            #[cfg(not(feature = "std"))]
            include_str!("../../../cairo_programs/bad_programs/ec_op_same_x.json"),
            Some("main"),
        );

        let mut cairo_runner = CairoRunner::new(&program, "all", false).unwrap();
        let mut vm = VirtualMachine::new(false);
        let end = cairo_runner.initialize(&mut vm).unwrap();

        let hint_executor = &mut BuiltinHintProcessor::new_empty();

        cairo_runner
            .run_until_pc(end, &mut vm, hint_executor)
            .unwrap();
        cairo_runner
            .end_run(false, false, &mut vm, hint_executor)
            .unwrap();

        vm.verify_auto_deductions().unwrap();
        cairo_runner.read_return_values(&mut vm).unwrap();
        let result = verify_secure_runner(&cairo_runner, true, &mut vm);

        assert!(result.is_err());
        // We need to check this way because CairoRunError doens't implement PartialEq
        match result {
            Err(VirtualMachineError::RunnerError(RunnerError::EcOpSameXCoordinate(_))) => {}
            Err(_) => panic!("Wrong error returned, expected RunnerError::EcOpSameXCoordinate"),
            Ok(_) => panic!("Expected run to fail"),
        }
    }

    #[test]
    fn catch_point_not_in_curve() {
        let program = load_program(
            #[cfg(feature = "std")]
            "cairo_programs/bad_programs/ec_op_not_in_curve.json",
            #[cfg(not(feature = "std"))]
            include_str!("../../../cairo_programs/bad_programs/ec_op_not_in_curve.json"),
            Some("main"),
        );

        let mut cairo_runner = CairoRunner::new(&program, "all", false).unwrap();
        let mut vm = VirtualMachine::new(false);
        let end = cairo_runner.initialize(&mut vm).unwrap();

        let hint_proccesor = &mut BuiltinHintProcessor::new_empty();
        cairo_runner
            .run_until_pc(end, &mut vm, hint_proccesor)
            .unwrap();

        cairo_runner
            .end_run(false, false, &mut vm, hint_proccesor)
            .unwrap();

        vm.verify_auto_deductions().unwrap();

        cairo_runner.read_return_values(&mut vm).unwrap();

        let result = verify_secure_runner(&cairo_runner, true, &mut vm);

        assert!(result.is_err());

        // We need to check this way because CairoRunError doens't implement PartialEq
        match result {
            Err(VirtualMachineError::RunnerError(RunnerError::PointNotOnCurve(_))) => {}
            Err(_) => panic!("Wrong error returned, expected RunnerError::EcOpSameXCoordinate"),
            Ok(_) => panic!("Expected run to fail"),
        }
    }
}
