use std::prelude::v1::*;

use crate::{
    types::{
        instruction::{Instruction, Op1Addr, Register},
        relocatable::{MaybeRelocatable, Relocatable},
    },
    vm::errors::{
        memory_errors::MemoryError::AddressNotRelocatable, vm_errors::VirtualMachineError,
    },
};
use num_traits::ToPrimitive;

pub struct RunContext {
    pub(crate) pc: Relocatable,
    pub(crate) ap: usize,
    pub(crate) fp: usize,
}

impl RunContext {
    pub fn get_ap(&self) -> Relocatable {
        Relocatable::from((1, self.ap))
    }
    pub fn get_fp(&self) -> Relocatable {
        Relocatable::from((1, self.fp))
    }
    pub fn get_pc(&self) -> &Relocatable {
        &self.pc
    }

    pub fn compute_dst_addr(
        &self,
        instruction: &Instruction,
    ) -> Result<Relocatable, VirtualMachineError> {
        let base_addr = match instruction.dst_register {
            Register::AP => self.get_ap(),
            Register::FP => self.get_fp(),
        };
        let new_offset = instruction.off0 + base_addr.offset as isize;
        Ok(Relocatable::from((
            base_addr.segment_index,
            new_offset
                .to_usize()
                .ok_or(VirtualMachineError::BigintToUsizeFail)?,
        )))
    }

    pub fn compute_op0_addr(
        &self,
        instruction: &Instruction,
    ) -> Result<Relocatable, VirtualMachineError> {
        let base_addr = match instruction.op0_register {
            Register::AP => self.get_ap(),
            Register::FP => self.get_fp(),
        };
        let new_offset = instruction.off1 + base_addr.offset as isize;
        Ok(Relocatable::from((
            base_addr.segment_index,
            new_offset
                .to_usize()
                .ok_or(VirtualMachineError::BigintToUsizeFail)?,
        )))
    }

    pub fn compute_op1_addr(
        &self,
        instruction: &Instruction,
        op0: Option<&MaybeRelocatable>,
    ) -> Result<Relocatable, VirtualMachineError> {
        let base_addr = match instruction.op1_addr {
            Op1Addr::FP => self.get_fp(),
            Op1Addr::AP => self.get_ap(),
            Op1Addr::Imm => match instruction.off2 == 1 {
                true => self.pc,
                false => return Err(VirtualMachineError::ImmShouldBe1),
            },
            Op1Addr::Op0 => match op0 {
                Some(MaybeRelocatable::RelocatableValue(addr)) => *addr,
                Some(_) => return Err(VirtualMachineError::MemoryError(AddressNotRelocatable)),
                None => return Err(VirtualMachineError::UnknownOp0),
            },
        };
        let new_offset = instruction.off2 + base_addr.offset as isize;
        Ok(Relocatable::from((
            base_addr.segment_index,
            new_offset
                .to_usize()
                .ok_or(VirtualMachineError::BigintToUsizeFail)?,
        )))
    }

    #[doc(hidden)]
    pub(crate) fn set_ap(&mut self, ap: usize) {
        self.ap = ap;
    }

    #[doc(hidden)]
    pub(crate) fn set_fp(&mut self, fp: usize) {
        self.fp = fp;
    }

    #[doc(hidden)]
    pub(crate) fn set_pc(&mut self, pc: Relocatable) {
        self.pc = pc;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::relocatable;
    use crate::types::instruction::{ApUpdate, FpUpdate, Opcode, PcUpdate, Res};
    use crate::utils::test_utils::mayberelocatable;
    use crate::vm::errors::memory_errors::MemoryError;
    use felt::Felt;

    #[test]
    fn compute_dst_addr_for_ap_register() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 3,
            imm: None,
            dst_register: Register::AP,
            op0_register: Register::FP,
            op1_addr: Op1Addr::AP,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };
        assert_eq!(
            Ok(relocatable!(1, 6)),
            run_context.compute_dst_addr(&instruction)
        );
    }

    #[test]
    fn compute_dst_addr_for_fp_register() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 3,
            imm: None,
            dst_register: Register::FP,
            op0_register: Register::AP,
            op1_addr: Op1Addr::AP,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };
        assert_eq!(
            Ok(relocatable!(1, 7)),
            run_context.compute_dst_addr(&instruction)
        );
    }

    #[test]
    fn compute_op0_addr_for_ap_register() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 3,
            imm: None,
            dst_register: Register::AP,
            op0_register: Register::AP,
            op1_addr: Op1Addr::AP,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };
        assert_eq!(
            Ok(relocatable!(1, 7)),
            run_context.compute_op0_addr(&instruction)
        );
    }

    #[test]
    fn compute_op0_addr_for_fp_register() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 3,
            imm: None,
            dst_register: Register::FP,
            op0_register: Register::FP,
            op1_addr: Op1Addr::AP,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };
        assert_eq!(
            Ok(relocatable!(1, 8)),
            run_context.compute_op0_addr(&instruction)
        );
    }

    #[test]
    fn compute_op1_addr_for_fp_op1_addr() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 3,
            imm: None,
            dst_register: Register::FP,
            op0_register: Register::AP,
            op1_addr: Op1Addr::FP,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };
        assert_eq!(
            Ok(relocatable!(1, 9)),
            run_context.compute_op1_addr(&instruction, None)
        );
    }

    #[test]
    fn compute_op1_addr_for_ap_op1_addr() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 3,
            imm: None,
            dst_register: Register::FP,
            op0_register: Register::AP,
            op1_addr: Op1Addr::AP,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };
        assert_eq!(
            Ok(relocatable!(1, 8)),
            run_context.compute_op1_addr(&instruction, None)
        );
    }

    #[test]
    fn compute_op1_addr_for_imm_op1_addr_correct_off2() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 1,
            imm: None,
            dst_register: Register::FP,
            op0_register: Register::AP,
            op1_addr: Op1Addr::Imm,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };
        assert_eq!(
            Ok(relocatable!(0, 5)),
            run_context.compute_op1_addr(&instruction, None)
        );
    }

    #[test]
    fn compute_op1_addr_for_imm_op1_addr_incorrect_off2() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 3,
            imm: None,
            dst_register: Register::FP,
            op0_register: Register::AP,
            op1_addr: Op1Addr::Imm,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };

        let error = run_context.compute_op1_addr(&instruction, None);
        assert_eq!(error, Err(VirtualMachineError::ImmShouldBe1));
        assert_eq!(
            error.unwrap_err().to_string(),
            "In immediate mode, off2 should be 1"
        );
    }

    #[test]
    fn compute_op1_addr_for_op0_op1_addr_with_op0() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 1,
            imm: None,
            dst_register: Register::FP,
            op0_register: Register::AP,
            op1_addr: Op1Addr::Op0,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };

        let op0 = mayberelocatable!(1, 7);
        assert_eq!(
            Ok(relocatable!(1, 8)),
            run_context.compute_op1_addr(&instruction, Some(&op0))
        );
    }

    #[test]
    fn compute_op1_addr_with_no_relocatable_address() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 1,
            imm: None,
            dst_register: Register::FP,
            op0_register: Register::AP,
            op1_addr: Op1Addr::Op0,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };

        let op0 = MaybeRelocatable::from(Felt::new(7));
        assert_eq!(
            Err(VirtualMachineError::MemoryError(
                MemoryError::AddressNotRelocatable
            )),
            run_context.compute_op1_addr(&instruction, Some(&op0))
        );
    }

    #[test]
    fn compute_op1_addr_for_op0_op1_addr_without_op0() {
        let instruction = Instruction {
            off0: 1,
            off1: 2,
            off2: 3,
            imm: None,
            dst_register: Register::FP,
            op0_register: Register::AP,
            op1_addr: Op1Addr::Op0,
            res: Res::Add,
            pc_update: PcUpdate::Regular,
            ap_update: ApUpdate::Regular,
            fp_update: FpUpdate::Regular,
            opcode: Opcode::NOp,
        };

        let run_context = RunContext {
            pc: relocatable!(0, 4),
            ap: 5,
            fp: 6,
        };

        let error = run_context.compute_op1_addr(&instruction, None);
        assert_eq!(error, Err(VirtualMachineError::UnknownOp0));
        assert_eq!(
            error.unwrap_err().to_string(),
            "op0 must be known in double dereference"
        );
    }
}
