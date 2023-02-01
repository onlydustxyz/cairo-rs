use std::prelude::v1::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KeccakInstanceDef {
    pub(crate) _ratio: u32,
    pub(crate) _state_rep: Vec<u32>,
    pub(crate) _instance_per_component: u32,
}

impl Default for KeccakInstanceDef {
    fn default() -> Self {
        Self {
            // _ratio should be equal to 2 ** 11 -> 2048
            _ratio: 2048,
            _state_rep: vec![200; 8],
            _instance_per_component: 16,
        }
    }
}

impl KeccakInstanceDef {
    pub(crate) fn new(_ratio: u32, _state_rep: Vec<u32>) -> Self {
        Self {
            _ratio,
            _state_rep,
            ..Default::default()
        }
    }

    pub(crate) fn _cells_per_builtin(&self) -> u32 {
        2 * self._state_rep.len() as u32
    }

    pub(crate) fn _range_check_units_per_builtin(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_range_check_units_per_builtin() {
        let builtin_instance = KeccakInstanceDef::default();
        assert_eq!(builtin_instance._range_check_units_per_builtin(), 0);
    }

    #[test]
    fn get_cells_per_builtin() {
        let builtin_instance = KeccakInstanceDef::default();
        assert_eq!(builtin_instance._cells_per_builtin(), 16);
    }

    #[test]
    fn test_new() {
        let builtin_instance = KeccakInstanceDef {
            _ratio: 2048,
            _state_rep: vec![200; 8],
            _instance_per_component: 16,
        };
        assert_eq!(KeccakInstanceDef::new(2048, vec![200; 8]), builtin_instance);
    }

    #[test]
    fn test_default() {
        let builtin_instance = KeccakInstanceDef {
            _ratio: 2048,
            _state_rep: vec![200; 8],
            _instance_per_component: 16,
        };
        assert_eq!(KeccakInstanceDef::default(), builtin_instance);
    }
}
