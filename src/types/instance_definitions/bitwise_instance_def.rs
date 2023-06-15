pub(crate) const CELLS_PER_BITWISE: u32 = 5;
pub(crate) const INPUT_CELLS_PER_BITWISE: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BitwiseInstanceDef {
    pub(crate) ratio: Option<u32>,
    pub(crate) total_n_bits: u32,
}

impl BitwiseInstanceDef {
    pub(crate) fn default() -> Self {
        BitwiseInstanceDef {
            ratio: Some(256),
            total_n_bits: 251,
        }
    }

    pub(crate) fn new(ratio: Option<u32>) -> Self {
        BitwiseInstanceDef {
            ratio,
            total_n_bits: 251,
        }
    }

    pub(crate) fn _cells_per_builtin(&self) -> u32 {
        CELLS_PER_BITWISE
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
        let builtin_instance = BitwiseInstanceDef::default();
        assert_eq!(builtin_instance._range_check_units_per_builtin(), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_builtin() {
        let builtin_instance = BitwiseInstanceDef::default();
        assert_eq!(builtin_instance._cells_per_builtin(), 5);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_new() {
        let builtin_instance = BitwiseInstanceDef {
            ratio: Some(8),
            total_n_bits: 251,
        };
        assert_eq!(BitwiseInstanceDef::new(Some(8)), builtin_instance);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = BitwiseInstanceDef {
            ratio: Some(256),
            total_n_bits: 251,
        };
        assert_eq!(BitwiseInstanceDef::default(), builtin_instance);
    }
}
