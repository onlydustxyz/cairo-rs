pub(crate) const CELLS_PER_EC_OP: u32 = 7;
pub(crate) const INPUT_CELLS_PER_EC_OP: u32 = 5;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct EcOpInstanceDef {
    pub(crate) ratio: Option<u32>,
    pub(crate) scalar_height: u32,
    pub(crate) _scalar_bits: u32,
}

impl EcOpInstanceDef {
    pub(crate) fn default() -> Self {
        EcOpInstanceDef {
            ratio: Some(256),
            scalar_height: 256,
            _scalar_bits: 252,
        }
    }

    pub(crate) fn new(ratio: Option<u32>) -> Self {
        EcOpInstanceDef {
            ratio,
            scalar_height: 256,
            _scalar_bits: 252,
        }
    }

    pub(crate) fn _cells_per_builtin(&self) -> u32 {
        CELLS_PER_EC_OP
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
        let builtin_instance = EcOpInstanceDef::default();
        assert_eq!(builtin_instance._range_check_units_per_builtin(), 0);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_cells_per_builtin() {
        let builtin_instance = EcOpInstanceDef::default();
        assert_eq!(builtin_instance._cells_per_builtin(), 7);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_new() {
        let builtin_instance = EcOpInstanceDef {
            ratio: Some(8),
            scalar_height: 256,
            _scalar_bits: 252,
        };
        assert_eq!(EcOpInstanceDef::new(Some(8)), builtin_instance);
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn test_default() {
        let builtin_instance = EcOpInstanceDef {
            ratio: Some(256),
            scalar_height: 256,
            _scalar_bits: 252,
        };
        assert_eq!(EcOpInstanceDef::default(), builtin_instance);
    }
}
