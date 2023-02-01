use std::prelude::v1::*;

use crate::vm::errors::trace_errors::TraceError;
use crate::{types::relocatable::Relocatable, vm::errors::memory_errors::MemoryError};
use serde::{Deserialize, Serialize};

///A trace entry for every instruction that was executed.
///Holds the register values before the instruction was executed.
#[derive(Debug, PartialEq, Eq)]
pub struct TraceEntry {
    pub pc: Relocatable,
    pub ap: Relocatable,
    pub fp: Relocatable,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelocatedTraceEntry {
    pub ap: usize,
    pub fp: usize,
    pub pc: usize,
}

pub fn relocate_trace_register(
    value: &Relocatable,
    relocation_table: &Vec<usize>,
) -> Result<usize, TraceError> {
    let segment_index: usize = value.segment_index.try_into().map_err(|_| {
        TraceError::MemoryError(MemoryError::AddressInTemporarySegment(value.segment_index))
    })?;

    if relocation_table.len() <= segment_index {
        return Err(TraceError::NoRelocationFound);
    }
    Ok(relocation_table[segment_index] + value.offset)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relocate_relocatable_value() {
        let value = Relocatable {
            segment_index: 2,
            offset: 7,
        };
        let relocation_table = vec![1, 2, 5];
        assert_eq!(
            relocate_trace_register(&value, &relocation_table).unwrap(),
            12
        );
    }

    #[test]
    fn relocate_relocatable_value_no_relocation() {
        let value = Relocatable {
            segment_index: 2,
            offset: 7,
        };
        let relocation_table = vec![1, 2];
        let error = relocate_trace_register(&value, &relocation_table);
        assert_eq!(error, Err(TraceError::NoRelocationFound));
        assert_eq!(
            error.unwrap_err().to_string(),
            "No relocation found for this segment"
        );
    }

    #[test]
    fn relocate_temp_segment_address() {
        let value = Relocatable {
            segment_index: -2,
            offset: 7,
        };
        let error = relocate_trace_register(&value, &Vec::new());
        assert_eq!(
            error,
            Err(TraceError::MemoryError(
                MemoryError::AddressInTemporarySegment(value.segment_index)
            ))
        );
    }
}
