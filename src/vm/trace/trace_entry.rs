use crate::stdlib::prelude::*;
use serde::{Deserialize, Serialize};

///A trace entry for every instruction that was executed.
///Holds the register values before the instruction was executed.
/// Before relocation:
///     Register values are represented as their offsets, as their indexes will always be 0,1,1 respectively
///     The index of the last pc will not be equal to 0, but it is not appended to the trace
/// After relocation the value of each register will be a single integer
#[derive(Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TraceEntry {
    pub pc: usize,
    pub ap: usize,
    pub fp: usize,
}
