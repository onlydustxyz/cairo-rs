use core::ops::Shl;

use self::trace_entry::TraceEntry;
use super::{
    decoding::decoder::decode_instruction,
    errors::{memory_errors::MemoryError, vm_errors::VirtualMachineError},
    vm_memory::memory::Memory,
};
use crate::stdlib::borrow::Cow;
use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use num_traits::ToPrimitive;

pub mod trace_entry;
const OFFSET_BITS: u32 = 16;
/// Return the minimum and maximum values in the perm_range_check component.
pub fn get_perm_range_check_limits(
    trace: &[TraceEntry],
    memory: &Memory,
) -> Result<Option<(isize, isize)>, VirtualMachineError> {
    trace
        .iter()
        .try_fold(None, |offsets: Option<(isize, isize)>, trace| {
            let instruction = memory.get_integer((0, trace.pc).into())?;
            let immediate = memory.get::<Relocatable>(&(0, trace.pc + 1).into());

            let instruction = instruction
                .to_i64()
                .ok_or(VirtualMachineError::InvalidInstructionEncoding)?;
            let immediate = immediate
                .map(|x| match x {
                    Cow::Borrowed(MaybeRelocatable::Int(value)) => Ok(value.clone()),
                    Cow::Owned(MaybeRelocatable::Int(value)) => Ok(value),
                    _ => Err(MemoryError::ExpectedInteger((0, trace.pc + 1).into())),
                })
                .transpose()?;

            let decoded_instruction = decode_instruction(instruction, immediate.as_ref())?;
            let off0 = decoded_instruction.off0 + 1_isize.shl(OFFSET_BITS - 1);
            let off1 = decoded_instruction.off1 + 1_isize.shl(OFFSET_BITS - 1);
            let off2 = decoded_instruction.off2 + 1_isize.shl(OFFSET_BITS - 1);

            let min_value = off0.min(off1).min(off2);
            let max_value = off0.max(off1).max(off2);
            Ok(
                offsets.map_or(Some((min_value, max_value)), |(min_offset, max_offset)| {
                    Some((min_offset.min(min_value), max_offset.max(max_value)))
                }),
            )
        })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::test_utils::*;
    use assert_matches::assert_matches;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    /// Test that get_perm_range_check_limits() works as intended with an empty
    /// trace.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_perm_range_check_limits_empty_trace() {
        let trace = &[];
        let memory = Memory::new();

        assert_matches!(get_perm_range_check_limits(trace, &memory), Ok(None));
    }

    /// Test that get_perm_range_check_limits() works as intended with a single
    /// trace element.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_perm_range_check_limits_single_element() {
        let trace = &[TraceEntry {
            pc: 0,
            ap: 0,
            fp: 0,
        }];

        let memory = memory![((0, 0), 0xFFFF_8000_0000_u64)];
        // off0 -32768
        // off1 0
        // off2 32767
        assert_matches!(
            get_perm_range_check_limits(trace, &memory),
            Ok(Some((0, 65535)))
        );
    }

    /// Test that get_perm_range_check_limits() works as intended with multiple
    /// trace elements.
    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_perm_range_check_limits_multiple_elements() {
        let trace = &[
            TraceEntry {
                pc: 0,
                ap: 0,
                fp: 0,
            },
            TraceEntry {
                pc: 1,
                ap: 0,
                fp: 0,
            },
            TraceEntry {
                pc: 2,
                ap: 0,
                fp: 0,
            },
        ];
        let memory = memory![
            ((0, 0), 0x80FF_8000_0530_u64),
            ((0, 1), 0xBFFF_8000_0620u64),
            ((0, 2), 0x8FFF_8000_0750u64)
        ];

        assert_matches!(
            get_perm_range_check_limits(trace, &memory),
            Ok(Some((1328, 49151)))
        );
    }
}
