use crate::stdlib::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct KeccakInstanceDef {
    pub(crate) ratio: Option<u32>,
    pub(crate) _state_rep: Vec<u32>,
    pub(crate) _instance_per_component: u32,
}

impl Default for KeccakInstanceDef {
    fn default() -> Self {
        Self {
            // ratio should be equal to 2 ** 11 -> 2048
            ratio: Some(2048),
            _state_rep: vec![200; 8],
            _instance_per_component: 16,
        }
    }
}

impl KeccakInstanceDef {
    pub(crate) fn new(ratio: Option<u32>, _state_rep: Vec<u32>) -> Self {
        Self {
            ratio,
            _state_rep,
            ..Default::default()
        }
    }

    pub(crate) fn cells_per_builtin(&self) -> u32 {
        2 * self._state_rep.len() as u32
    }

    pub(crate) fn _range_check_units_per_builtin(&self) -> u32 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_range_check_units_per_builtin() {
        let builtin_instance = KeccakInstanceDef::default();
        assert_eq!(builtin_instance._range_check_units_per_builtin(), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_builtin() {
        let builtin_instance = KeccakInstanceDef::default();
        assert_eq!(builtin_instance.cells_per_builtin(), 16);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_new() {
        let builtin_instance = KeccakInstanceDef {
            ratio: Some(2048),
            _state_rep: vec![200; 8],
            _instance_per_component: 16,
        };
        assert_eq!(
            KeccakInstanceDef::new(Some(2048), vec![200; 8]),
            builtin_instance
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = KeccakInstanceDef {
            ratio: Some(2048),
            _state_rep: vec![200; 8],
            _instance_per_component: 16,
        };
        assert_eq!(KeccakInstanceDef::default(), builtin_instance);
    }
}
