use num_bigint::{BigInt, Sign};

pub(crate) const CELLS_PER_HASH: u32 = 3;
pub(crate) const INPUT_CELLS_PER_HASH: u32 = 2;

#[derive(Debug, PartialEq)]
pub(crate) struct PedersenInstanceDef {
    pub(crate) ratio: Option<u32>,
    pub(crate) _repetitions: u32,
    pub(crate) _element_height: u32,
    pub(crate) _element_bits: u32,
    pub(crate) _n_inputs: u32,
    pub(crate) _hash_limit: BigInt,
}

impl PedersenInstanceDef {
    pub(crate) fn default() -> Self {
        PedersenInstanceDef {
            ratio: Some(8),
            _repetitions: 4,
            _element_height: 256,
            _element_bits: 252,
            _n_inputs: 2,
            _hash_limit: BigInt::new(Sign::Plus, vec![1, 0, 0, 0, 0, 0, 17, 134217728]),
        }
    }

    pub(crate) fn new(ratio: Option<u32>, _repetitions: u32) -> Self {
        PedersenInstanceDef {
            ratio,
            _repetitions,
            _element_height: 256,
            _element_bits: 252,
            _n_inputs: 2,
            _hash_limit: BigInt::new(Sign::Plus, vec![1, 0, 0, 0, 0, 0, 17, 134217728]),
        }
    }

    pub(crate) fn _cells_per_builtin(&self) -> u32 {
        CELLS_PER_HASH
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
        let builtin_instance = PedersenInstanceDef::default();
        assert_eq!(builtin_instance._range_check_units_per_builtin(), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_builtin() {
        let builtin_instance = PedersenInstanceDef::default();
        assert_eq!(builtin_instance._cells_per_builtin(), 3);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_new() {
        let builtin_instance = PedersenInstanceDef {
            ratio: Some(10),
            _repetitions: 2,
            _element_height: 256,
            _element_bits: 252,
            _n_inputs: 2,
            _hash_limit: BigInt::new(Sign::Plus, vec![1, 0, 0, 0, 0, 0, 17, 134217728]),
        };
        assert_eq!(PedersenInstanceDef::new(Some(10), 2), builtin_instance);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = PedersenInstanceDef {
            ratio: Some(8),
            _repetitions: 4,
            _element_height: 256,
            _element_bits: 252,
            _n_inputs: 2,
            _hash_limit: BigInt::new(Sign::Plus, vec![1, 0, 0, 0, 0, 0, 17, 134217728]),
        };
        assert_eq!(PedersenInstanceDef::default(), builtin_instance);
    }
}
